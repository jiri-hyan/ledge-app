//! Limity Claude (5h a týdenní okno) — čistá logika: vzorek ze vstupu stavového řádku Claude Code
//! (`rate_limits`, docs: <https://code.claude.com/docs/en/statusline>) i z uloženého souboru, výběr
//! čerstvějšího ze dvou zdrojů, stáří, text proužku a rozhodnutí o připomínce resetu (odpočet
//! „za 2 h 10 min“ počítá UI: `src/limits/model.ts`). I/O (soubory, watcher, peek) je v `limitslive.rs`,
//! wrapper stavového řádku v `statusline.rs`.
//!
//! Formát souboru (vlastní `%LOCALAPPDATA%\Ledge\ratelimits.json` i záložní
//! `~/.claude/usage-log/latest.json`): `{"at": ms, "fiveHour": {"pct": 0–100, "resetsAt": ms},
//! "sevenDay": {…}}` — cizí pole se ignorují, čas smí být i v sekundách nebo ISO.
//! Nic se nedopočítává: okno, které nepřišlo, je `None` (≠ 0 %).

use serde::Serialize;
use serde_json::Value;

/// Vzorek starší než tohle se ukazuje jako „staré (před X min)“.
pub const STALE_MS: i64 = 15 * 60_000;
/// Připomínka resetu, který proběhl déle než před tímhle (PC spal, Ledge neběžel), se už neukáže.
pub const REMIND_GRACE_MS: i64 = 30 * 60_000;
/// Wrapper nepřepisuje soubor stejnými hodnotami častěji než tohle (stáří vzorku pak „lže“ nejvýš o minutu).
pub const REWRITE_SAME_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    /// spotřeba okna 0–100 (Claude Code posílá i desetinná)
    pub pct: f64,
    /// konec okna (ms epochy); `None` = nevíme
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// vlastní wrapper stavového řádku Ledge
    StatusLine,
    /// `~/.claude/usage-log/latest.json` (zapisuje ho cizí nástroj, pokud existuje)
    UsageLog,
    /// `plan-usage-history.json` aplikace Claude (volba `claude.limitsDesktop`, `limitsrc.rs`) — jen procenta
    Desktop,
    /// dotaz Ledge na Anthropic (volba `claude.limitsFetch`, `limitsfetch.rs`), uložený v `ratelimits.json` s `"via":"api"`
    Api,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    /// kdy vzorek vznikl (ms epochy)
    pub at: i64,
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    FiveHour,
    SevenDay,
}

impl Sample {
    pub fn window(&self, k: Kind) -> Option<Window> {
        match k {
            Kind::FiveHour => self.five_hour,
            Kind::SevenDay => self.seven_day,
        }
    }
}

// ---------- parsování ----------

/// Epoch ms z čísla (sekundy nebo ms) nebo řetězce (číslo / RFC 3339); jinak `None`.
pub fn to_ms(v: &Value) -> Option<i64> {
    let n = match v {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) if !s.trim().is_empty() => match s.trim().parse::<f64>() {
            Ok(n) => n,
            Err(_) => return chrono::DateTime::parse_from_rfc3339(s.trim()).ok().map(|d| d.timestamp_millis()),
        },
        _ => return None,
    };
    if !n.is_finite() || n <= 0.0 {
        return None;
    }
    Some(if n < 1e12 { (n * 1000.0).round() as i64 } else { n.round() as i64 })
}

fn pct(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|p| p.is_finite() && *p >= 0.0)
}

/// Okno ze vstupu stavového řádku: `{used_percentage, resets_at (s)}`.
fn status_window(v: Option<&Value>) -> Option<Window> {
    let w = v?;
    Some(Window { pct: pct(w.get("used_percentage"))?, resets_at: w.get("resets_at").and_then(to_ms) })
}

/// Okno ze souboru: `{pct, resetsAt}`.
fn file_window(v: Option<&Value>) -> Option<Window> {
    let w = v?;
    Some(Window { pct: pct(w.get("pct"))?, resets_at: w.get("resetsAt").and_then(to_ms) })
}

/// Vstup stavového řádku (JSON od Claude Code) → vzorek; `None`, když `rate_limits` nepřišly
/// (účet bez Pro/Max, před první odpovědí API).
pub fn from_status_input(input: &Value, at: i64) -> Option<Sample> {
    let rl = input.get("rate_limits")?;
    let five_hour = status_window(rl.get("five_hour"));
    let seven_day = status_window(rl.get("seven_day"));
    (five_hour.is_some() || seven_day.is_some()).then_some(Sample { at, five_hour, seven_day, source: Source::StatusLine })
}

/// Uložený vzorek (vlastní soubor i `usage-log/latest.json`) → vzorek; bez `at` nebo bez oken `None`.
pub fn from_file(text: &str, source: Source) -> Option<Sample> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let at = v.get("at").and_then(to_ms)?;
    let five_hour = file_window(v.get("fiveHour"));
    let seven_day = file_window(v.get("sevenDay"));
    // vzorek z dotazu na Anthropic (`limitsrc::to_file_api`) leží ve stejném souboru jako wrapper
    let source = if v.get("via").and_then(Value::as_str) == Some("api") { Source::Api } else { source };
    (five_hour.is_some() || seven_day.is_some()).then_some(Sample { at, five_hour, seven_day, source })
}

/// Vzorek do souboru (stejný tvar, jaký čte `from_file`).
pub fn to_file(s: &Sample) -> String {
    let w = |w: Option<Window>| w.map(|w| serde_json::json!({ "pct": w.pct, "resetsAt": w.resets_at }));
    serde_json::json!({ "at": s.at, "fiveHour": w(s.five_hour), "sevenDay": w(s.seven_day) }).to_string()
}

/// Má wrapper soubor přepsat? Jiné hodnoty vždy, stejné až po `REWRITE_SAME_MS` (ať stáří sedí).
pub fn should_write(old: Option<&Sample>, new: &Sample) -> bool {
    match old {
        Some(o) => o.five_hour != new.five_hour || o.seven_day != new.seven_day || new.at - o.at >= REWRITE_SAME_MS || new.at < o.at,
        None => true,
    }
}

/// Čerstvější ze dvou zdrojů podle `at`; při shodě vlastní wrapper.
pub fn fresher(own: Option<Sample>, log: Option<Sample>) -> Option<Sample> {
    match (own, log) {
        (Some(a), Some(b)) => Some(if b.at > a.at { b } else { a }),
        (a, b) => a.or(b),
    }
}

// ---------- stav okna ----------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// čerstvé procento
    Fresh(f64),
    /// procento ze staršího vzorku (před `age_min` minutami)
    Stale { pct: f64, age_min: i64 },
    /// `resetsAt` je v minulosti → okno se obnovilo, nové procento neznáme
    Reset,
}

pub fn state(w: Window, at: i64, now: i64) -> State {
    if w.resets_at.is_some_and(|r| r <= now) {
        return State::Reset;
    }
    let age = now - at;
    if age > STALE_MS { State::Stale { pct: w.pct, age_min: age / 60_000 } } else { State::Fresh(w.pct) }
}

// ---------- proužek ----------

/// Text segmentu proužku: „5h 42 % · týden 59 %“; u okna ≥ 90 % i čas resetu („5h 100 % → 14:30“).
/// Staré ani obnovené okno se nehlásí (neznáme ho). `clock` = ms → „HH:MM“ místního času.
pub fn strip_text(s: &Sample, now: i64, clock: &dyn Fn(i64) -> String) -> Option<String> {
    let mut parts = Vec::new();
    for (k, name) in [(Kind::FiveHour, "5h"), (Kind::SevenDay, crate::i18n::tr("týden", "week"))] {
        let Some(w) = s.window(k) else { continue };
        match state(w, s.at, now) {
            State::Fresh(p) => {
                let mut t = format!("{name} {:.0} %", p.floor());
                if p >= 90.0
                    && let Some(r) = w.resets_at
                {
                    t.push_str(&format!(" → {}", clock(r)));
                }
                parts.push(t);
            }
            State::Reset => parts.push(trf!("{name} obnoveno", "{name} reset")),
            State::Stale { .. } => {}
        }
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// Nejvyšší čerstvé procento (barva tečky proužku); `None` = nic čerstvého.
pub fn top_fresh(s: &Sample, now: i64) -> Option<f64> {
    [Kind::FiveHour, Kind::SevenDay]
        .iter()
        .filter_map(|k| s.window(*k))
        .filter_map(|w| match state(w, s.at, now) {
            State::Fresh(p) => Some(p),
            _ => None,
        })
        .reduce(f64::max)
}

// ---------- připomínka resetu ----------

/// Okna, která dosáhla 100 % a čekají na reset: (okno, `resetsAt`). Plní `arm`, vyprazdňuje `due`.
pub type Armed = Vec<(Kind, i64)>;

/// Zapamatuje si okna ≥ 100 % s resetem v budoucnosti (fakt ze vzorku, nezávisle na jeho stáří —
/// pozdější vzorek bez okna ho nesmaže).
pub fn arm(armed: &mut Armed, s: &Sample, now: i64) {
    for k in [Kind::FiveHour, Kind::SevenDay] {
        if let Some(Window { pct, resets_at: Some(r) }) = s.window(k)
            && pct >= 100.0
            && r > now
            && !armed.contains(&(k, r))
        {
            armed.push((k, r));
        }
    }
}

/// Nastal reset? Vrátí okna k připomenutí (reset nejvýš `REMIND_GRACE_MS` zpět) a odebere všechny
/// prošlé (i ty příliš staré — tiše).
pub fn due(armed: &mut Armed, now: i64) -> Vec<Kind> {
    let mut out = Vec::new();
    armed.retain(|&(k, r)| {
        if r > now {
            return true;
        }
        if now - r <= REMIND_GRACE_MS && !out.contains(&k) {
            out.push(k);
        }
        false
    });
    out
}

/// Nejbližší termín resetu k připomenutí (buzení emit vlákna).
pub fn next_reset(armed: &Armed) -> Option<i64> {
    armed.iter().map(|&(_, r)| r).min()
}

// ---------- upozornění na 80 % / 90 % (volba `claude.limitsAlert`) ----------

/// Prahy upozornění v procentech, vzestupně.
pub const ALERT_LEVELS: [u8; 2] = [80, 90];

/// Co už se v kterém okně ohlásilo: (okno, konec okna nebo 0, nejvyšší ohlášený práh).
pub type Alerted = Vec<(Kind, i64, u8)>;

/// Které prahy ohlásit: jednou za okno (okno = druh + `resetsAt`) a každý práh nejvýš jednou; když procenta
/// přeskočí oba prahy najednou, ohlásí se jen vyšší. Záznamy starých oken se zahodí. Vrací (okno, práh, procenta).
pub fn alerts_due(done: &mut Alerted, s: &Sample) -> Vec<(Kind, u8, f64)> {
    let mut out = Vec::new();
    let mut keep: Alerted = Vec::new();
    for k in [Kind::FiveHour, Kind::SevenDay] {
        let Some(w) = s.window(k) else { continue };
        let reset = w.resets_at.unwrap_or(0);
        let had = done.iter().find(|&&(dk, dr, _)| dk == k && dr == reset).map_or(0, |&(_, _, l)| l);
        let level = ALERT_LEVELS.iter().rev().copied().find(|&l| w.pct >= f64::from(l)).unwrap_or(0);
        if level > had {
            out.push((k, level, w.pct));
        }
        keep.push((k, reset, level.max(had)));
    }
    *done = keep;
    out
}

/// Kolik vzorků 5h okna si pamatujeme pro tempo a jak dlouho zpět (kruhový buffer v paměti, volba `limitsPace`).
pub const HIST_MAX: usize = 64;
pub const HIST_SPAN_MS: i64 = 6 * 3_600_000;

/// Přidá vzorek 5h okna do historie: starší / stejný čas se přeskočí, pokles procent (nové okno) historii
/// smaže, nejstarší (> `HIST_SPAN_MS` nebo přes `HIST_MAX`) odpadnou. Vrací, zda se vzorek přidal.
pub fn push_hist(h: &mut Vec<(i64, f64)>, at: i64, pct: f64) -> bool {
    if h.last().is_some_and(|&(t, _)| at <= t) {
        return false;
    }
    if h.last().is_some_and(|&(_, p)| pct < p - 0.5) {
        h.clear();
    }
    h.push((at, pct));
    h.retain(|&(t, _)| at - t <= HIST_SPAN_MS);
    if h.len() > HIST_MAX {
        h.drain(..h.len() - HIST_MAX);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW: i64 = 1_792_250_979_530;

    fn w(pct: f64, r: Option<i64>) -> Option<Window> {
        Some(Window { pct, resets_at: r })
    }

    #[test]
    fn hostile_limit_files_and_inputs() {
        // soubor s limity i vstup stavového řádku jsou cizí JSON: špatné typy, obří a nekonečná čísla
        let files = [
            "", "   ", "\u{feff}", "[]", "null", "42", "\"x\"", "{", "{\"at\":1}", "{\"at\":\"zítra\",\"fiveHour\":{\"pct\":1}}",
            "{\"at\":1e400,\"fiveHour\":{\"pct\":1}}", "{\"at\":1e19,\"fiveHour\":{\"pct\":1e308,\"resetsAt\":1e19}}",
            "{\"at\":1,\"fiveHour\":{\"pct\":-1}}", "{\"at\":1,\"fiveHour\":{\"pct\":\"50\"}}", "{\"at\":1,\"fiveHour\":[1,2]}", "{\"at\":1,\"sevenDay\":null}",
            "{\"at\":9223372036854775807,\"sevenDay\":{\"pct\":100,\"resetsAt\":9223372036854775807}}",
        ];
        for f in files {
            if let Some(s) = from_file(f, Source::UsageLog) {
                // co projde, s tím se dá dál počítat bez paniky (stáří, proužek, připomínka)
                let _ = strip_text(&s, NOW, &|ms| ms.to_string());
                let _ = top_fresh(&s, NOW);
                let _ = should_write(Some(&s), &s);
                let mut armed = Armed::new();
                arm(&mut armed, &s, NOW);
                let _ = (due(&mut armed, i64::MAX), next_reset(&armed));
                let _ = to_file(&s);
            }
        }
        let inputs = [
            json!({}),
            json!([]),
            json!(null),
            json!({"rate_limits": null}),
            json!({"rate_limits": []}),
            json!({"rate_limits": {"five_hour": {"used_percentage": "x"}}}),
            json!({"rate_limits": {"five_hour": {"used_percentage": 1e308, "resets_at": -1}}}),
            json!({"rate_limits": {"seven_day": {"used_percentage": 12.5, "resets_at": "x"}}}),
        ];
        for i in &inputs {
            let _ = from_status_input(i, NOW);
        }
        assert_eq!(from_status_input(&inputs[7], NOW).and_then(|s| s.seven_day).map(|w| (w.pct, w.resets_at)), Some((12.5, None)), "neplatný reset = neznámý, procento zůstane");
        assert!(from_status_input(&inputs[5], NOW).is_none(), "text místo čísla = okno není");
    }

    #[test]
    fn to_ms_table() {
        let cases = [
            (json!(1738425600), Some(1_738_425_600_000)),
            (json!(1738425600.5), Some(1_738_425_600_500)),
            (json!(1_792_258_800_099_i64), Some(1_792_258_800_099)),
            (json!("1738425600"), Some(1_738_425_600_000)),
            (json!("2026-10-17T12:00:00Z"), Some(1_792_238_400_000)),
            (json!("2026-10-17T14:00:00+02:00"), Some(1_792_238_400_000)),
            (json!("zítra"), None),
            (json!(""), None),
            (json!(null), None),
            (json!(0), None),
            (json!(-5), None),
            (json!(true), None),
        ];
        for (v, want) in cases {
            assert_eq!(to_ms(&v), want, "{v}");
        }
    }

    #[test]
    fn status_input_table() {
        let full = json!({"model":{"id":"x"},"rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600},"seven_day":{"used_percentage":41.2,"resets_at":1738857600},"spend_limit":{"used_percentage":1}}});
        let s = from_status_input(&full, NOW).unwrap();
        assert_eq!(s, Sample { at: NOW, five_hour: w(23.5, Some(1_738_425_600_000)), seven_day: w(41.2, Some(1_738_857_600_000)), source: Source::StatusLine });
        let cases = [
            (json!({}), None),
            (json!({"rate_limits":{}}), None),
            (json!({"rate_limits":null}), None),
            (json!({"rate_limits":{"five_hour":{"resets_at":1}}}), None),
            (json!({"rate_limits":{"five_hour":{"used_percentage":"12"}}}), None),
            (json!({"rate_limits":{"five_hour":{"used_percentage":-1}}}), None),
            // jen týden (5h okno Claude Code po resetu zahodí)
            (json!({"rate_limits":{"seven_day":{"used_percentage":59}}}), Some((None, w(59.0, None)))),
            (json!({"rate_limits":{"five_hour":{"used_percentage":0,"resets_at":"x"}}}), Some((w(0.0, None), None))),
        ];
        for (v, want) in cases {
            assert_eq!(from_status_input(&v, NOW).map(|s| (s.five_hour, s.seven_day)), want, "{v}");
        }
    }

    #[test]
    fn file_formats_table() {
        // usage-log/latest.json (cizí pole navíc, ms)
        let log = r#"{"at":1792250979530,"fiveHour":{"pct":3,"resetsAt":1792258800099},"sevenDay":{"pct":59,"resetsAt":1792728000099},"source":"get_usage","loggedAt":1}"#;
        let s = from_file(log, Source::UsageLog).unwrap();
        assert_eq!((s.at, s.five_hour, s.seven_day, s.source), (NOW, w(3.0, Some(1_792_258_800_099)), w(59.0, Some(1_792_728_000_099)), Source::UsageLog));
        // vlastní soubor: zápis a čtení zpět
        let own = Sample { at: NOW, five_hour: w(42.5, Some(NOW + 1000)), seven_day: None, source: Source::StatusLine };
        assert_eq!(from_file(&to_file(&own), Source::StatusLine), Some(own));
        let bad = ["", "{", "[]", r#"{"fiveHour":{"pct":3}}"#, r#"{"at":1792250979530}"#, r#"{"at":1792250979530,"fiveHour":{"pct":null},"sevenDay":null}"#];
        for t in bad {
            assert_eq!(from_file(t, Source::UsageLog), None, "{t}");
        }
        assert!(from_file("\u{feff}{\"at\":1792250979,\"sevenDay\":{\"pct\":1}}", Source::UsageLog).is_some(), "BOM a sekundy");
    }

    #[test]
    fn fresher_and_rewrite() {
        let a = Sample { at: 100, five_hour: w(1.0, None), seven_day: None, source: Source::StatusLine };
        let b = Sample { at: 200, source: Source::UsageLog, ..a };
        assert_eq!(fresher(Some(a), Some(b)), Some(b));
        assert_eq!(fresher(Some(b), Some(Sample { at: 100, ..b })), Some(b));
        assert_eq!(fresher(Some(a), Some(Sample { at: 100, ..b })), Some(a), "shoda → vlastní");
        assert_eq!(fresher(None, Some(b)), Some(b));
        assert_eq!(fresher(Some(a), None), Some(a));
        assert_eq!(fresher(None, None), None);
        let cases = [
            (None, a, true),
            (Some(a), Sample { at: 100 + REWRITE_SAME_MS - 1, ..a }, false),
            (Some(a), Sample { at: 100 + REWRITE_SAME_MS, ..a }, true),
            (Some(a), Sample { at: 101, five_hour: w(2.0, None), ..a }, true),
            (Some(a), Sample { at: 101, seven_day: w(0.0, None), ..a }, true),
            (Some(a), Sample { at: 50, ..a }, true),
        ];
        for (old, new, want) in cases {
            assert_eq!(should_write(old.as_ref(), &new), want, "{old:?} → {new:?}");
        }
    }

    #[test]
    fn state_table() {
        let m = 60_000;
        let cases = [
            (Window { pct: 42.0, resets_at: Some(NOW + 3 * m) }, NOW - 5 * m, State::Fresh(42.0)),
            (Window { pct: 42.0, resets_at: None }, NOW - STALE_MS, State::Fresh(42.0)),
            (Window { pct: 42.0, resets_at: Some(NOW + m) }, NOW - 47 * m, State::Stale { pct: 42.0, age_min: 47 }),
            (Window { pct: 100.0, resets_at: Some(NOW) }, NOW, State::Reset),
            (Window { pct: 100.0, resets_at: Some(NOW - 1) }, NOW - 99 * m, State::Reset),
        ];
        for (win, at, want) in cases {
            assert_eq!(state(win, at, NOW), want, "{win:?} {at}");
        }
    }

    #[test]
    fn strip_text_table() {
        let clock = |_: i64| "14:30".to_string();
        let s = |f: Option<Window>, d: Option<Window>, at: i64| Sample { at, five_hour: f, seven_day: d, source: Source::StatusLine };
        let later = Some(NOW + 3_600_000);
        let cases = [
            (s(w(42.7, later), w(59.0, later), NOW), Some("5h 42 % · týden 59 %")),
            (s(w(100.0, later), None, NOW), Some("5h 100 % → 14:30")),
            (s(w(95.0, None), None, NOW), Some("5h 95 %")),
            (s(w(100.0, Some(NOW - 1)), w(59.0, later), NOW), Some("5h obnoveno · týden 59 %")),
            (s(w(42.0, later), w(59.0, later), NOW - STALE_MS - 1), None),
            (s(None, None, NOW), None),
        ];
        for (sample, want) in cases {
            assert_eq!(strip_text(&sample, NOW, &clock).as_deref(), want, "{sample:?}");
        }
        assert_eq!(top_fresh(&s(w(42.0, later), w(59.0, later), NOW), NOW), Some(59.0));
        assert_eq!(top_fresh(&s(w(100.0, Some(NOW)), None, NOW), NOW), None);
    }

    #[test]
    fn reminder_decisions() {
        let m = 60_000;
        let r5 = NOW + 10 * m;
        let r7 = NOW + 3 * 24 * 60 * m;
        let mut armed = Armed::new();
        let s = |f: f64, d: f64| Sample { at: NOW, five_hour: w(f, Some(r5)), seven_day: w(d, Some(r7)), source: Source::StatusLine };
        // pod 100 % nic
        arm(&mut armed, &s(99.9, 50.0), NOW);
        assert!(armed.is_empty());
        // 100 % → čeká na reset, opakovaný vzorek nic nezdvojí
        arm(&mut armed, &s(100.0, 50.0), NOW);
        arm(&mut armed, &s(100.0, 50.0), NOW + m);
        assert_eq!(armed, vec![(Kind::FiveHour, r5)]);
        assert_eq!(next_reset(&armed), Some(r5));
        // před resetem nic, v čase resetu jednou
        assert_eq!(due(&mut armed, r5 - 1), vec![]);
        assert_eq!(due(&mut armed, r5), vec![Kind::FiveHour]);
        assert_eq!(due(&mut armed, r5 + 1), vec![], "jen jednou");
        assert_eq!(next_reset(&armed), None);
        // starý vzorek po resetu (resetsAt v minulosti) se nezapamatuje
        arm(&mut armed, &s(100.0, 100.0), r5 + m);
        assert_eq!(armed, vec![(Kind::SevenDay, r7)]);
        // reset dávno prošel (PC spal) → tiše zahodit
        assert_eq!(due(&mut armed, r7 + REMIND_GRACE_MS + 1), vec![]);
        assert!(armed.is_empty());
        // obě okna najednou
        let mut both: Armed = vec![(Kind::FiveHour, NOW), (Kind::SevenDay, NOW - m)];
        assert_eq!(due(&mut both, NOW), vec![Kind::FiveHour, Kind::SevenDay]);
        // reset bez času se nezapamatuje (neznáme kdy)
        let mut none = Armed::new();
        arm(&mut none, &Sample { at: NOW, five_hour: w(100.0, None), seven_day: None, source: Source::UsageLog }, NOW);
        assert!(none.is_empty());
    }

    #[test]
    fn hist_push_dedup_reset_and_caps() {
        let mut h = Vec::new();
        assert!(push_hist(&mut h, 1000, 10.0));
        assert!(!push_hist(&mut h, 1000, 11.0), "stejný čas");
        assert!(!push_hist(&mut h, 900, 11.0), "starší");
        assert!(push_hist(&mut h, 2000, 12.0));
        assert_eq!(h.len(), 2);
        assert!(push_hist(&mut h, 3000, 2.0), "pokles = nové okno");
        assert_eq!(h, vec![(3000, 2.0)]);
        assert!(push_hist(&mut h, 3000 + HIST_SPAN_MS + 1, 5.0));
        assert_eq!(h.len(), 1);
        let mut h = Vec::new();
        for i in 0..100 {
            push_hist(&mut h, i * 1000, 1.0 + i as f64 * 0.01);
        }
        assert_eq!(h.len(), HIST_MAX);
        assert_eq!(h.last().unwrap().0, 99_000);
    }

    #[test]
    fn alerts_once_per_window() {
        let sample = |five: f64, seven: f64, r5: i64| Sample { at: NOW, five_hour: w(five, Some(r5)), seven_day: w(seven, Some(NOW + 9)), source: Source::StatusLine };
        let mut done = Alerted::new();
        // (vzorek, očekávané (okno, práh))
        let steps: Vec<(Sample, Vec<(Kind, u8)>)> = vec![
            (sample(10.0, 10.0, NOW + 1), vec![]),
            (sample(79.9, 10.0, NOW + 1), vec![]),
            (sample(80.0, 10.0, NOW + 1), vec![(Kind::FiveHour, 80)]),
            (sample(85.0, 10.0, NOW + 1), vec![]),
            (sample(91.0, 10.0, NOW + 1), vec![(Kind::FiveHour, 90)]),
            (sample(99.0, 10.0, NOW + 1), vec![]),
            (sample(20.0, 10.0, NOW + 1), vec![]),
            (sample(95.0, 10.0, NOW + 1), vec![]),
            // nové okno (jiný resetsAt) = znovu; přeskok obou prahů = jen vyšší
            (sample(95.0, 85.0, NOW + 99), vec![(Kind::FiveHour, 90), (Kind::SevenDay, 80)]),
        ];
        for (i, (smp, want)) in steps.into_iter().enumerate() {
            let got: Vec<_> = alerts_due(&mut done, &smp).into_iter().map(|(k, l, _)| (k, l)).collect();
            assert_eq!(got, want, "krok {i}");
        }
    }
}
