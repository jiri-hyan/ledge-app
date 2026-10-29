//! Další zdroje limitů Claude (5h / týden) mimo stavový řádek — čistá logika, I/O je v `limitslive.rs`
//! (soubor aplikace Claude) a `limitsfetch.rs` (dotaz na Anthropic).
//!
//! 1. **Aplikace Claude (desktop)** — volba `claude.limitsDesktop`: hlavní proces aplikace si sám ptá
//!    `claude.ai/api/organizations/<org>/usage` (15 min, jen když je to v aplikaci zapnuté) a každý výsledek
//!    připíše do `plan-usage-history.json` ve své datové složce (`{"version":2,"samples":[{"t":ms,"org":…,
//!    "u":{"fh":%,"sd":%}}]}`, starší `version:1` má `fh`/`sd` přímo ve vzorku). Jen procenta, **bez času
//!    resetu** — ten se doplní z jiného zdroje jen tehdy, když jeho okno v čase vzorku ještě běželo (`fill_resets`).
//!    Formát je nedokumentovaný: cokoli nečekaného = žádný vzorek (bezpečné selhání).
//! 2. **Dotaz na Anthropic** — volba `claude.limitsFetch`: tentýž dotaz, který dělá `/usage` v Claude Code
//!    (`GET https://api.anthropic.com/api/oauth/usage`, `anthropic-beta: oauth-2025-04-20`). Odpověď
//!    `{"five_hour":{"utilization":%,"resets_at":ISO|null},"seven_day":{…},…}` → `from_usage_api`.
//!    Přístupový token z `~/.claude/.credentials.json` vybírá `token_of` (jen `claudeAiOauth.accessToken`
//!    a `expiresAt`; refresh token se nečte).
//!
//! Nic se nedopočítává: okno, které nepřišlo, je `None`.

use crate::limits::{Sample, Source, Window, to_file, to_ms};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Jméno souboru historie v datové složce aplikace Claude.
pub const DESKTOP_FILE: &str = "plan-usage-history.json";
/// Délka oken: vzorek bez času resetu starší než okno už o aktuálním okně nic neříká.
pub const FIVE_HOUR_MS: i64 = 5 * 3_600_000;
pub const SEVEN_DAY_MS: i64 = 7 * 86_400_000;
/// Token, který vyprší do minuty, se už nepoužije (dotaz by skončil 401).
pub const EXPIRY_MARGIN_MS: i64 = 60_000;

fn pct_of(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|p| p.is_finite() && *p >= 0.0)
}

/// Odpověď `/api/oauth/usage` → vzorek (zdroj `Api`, čas `at` = kdy přišla). Bez obou oken `None`.
pub fn from_usage_api(text: &str, at: i64) -> Option<Sample> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let win = |k: &str| -> Option<Window> {
        let w = v.get(k)?;
        Some(Window { pct: pct_of(w.get("utilization"))?, resets_at: w.get("resets_at").and_then(to_ms) })
    };
    let (five_hour, seven_day) = (win("five_hour"), win("seven_day"));
    (five_hour.is_some() || seven_day.is_some()).then_some(Sample { at, five_hour, seven_day, source: Source::Api })
}

/// Vzorek z `/api/oauth/usage` do `ratelimits.json`: tvar `limits::to_file` + `"via":"api"`
/// (`limits::from_file` podle něj pozná zdroj; wrapper stavového řádku ho při dalším zápisu přepíše).
pub fn to_file_api(s: &Sample) -> String {
    let mut v: Value = serde_json::from_str(&to_file(s)).unwrap_or(Value::Null);
    if let Some(o) = v.as_object_mut() {
        o.insert("via".into(), Value::String("api".into()));
    }
    v.to_string()
}

/// `plan-usage-history.json` aplikace Claude → nejnovější vzorek (zdroj `Desktop`). Okno bez času resetu,
/// jehož vzorek je k `now` starší než délka okna, se zahodí (okno se mezitím určitě obnovilo).
pub fn from_desktop_history(text: &str, now: i64) -> Option<Sample> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let samples = v.get("samples")?.as_array()?;
    let last = samples.iter().filter_map(|s| Some((s.get("t").and_then(to_ms)?, s))).max_by_key(|(t, _)| *t)?;
    let (at, s) = last;
    // version 2: hodnoty v `u`; version 1: přímo ve vzorku
    let u = s.get("u").unwrap_or(s);
    let win = |k: &str, span: i64| (now - at <= span).then(|| pct_of(u.get(k))).flatten().map(|pct| Window { pct, resets_at: None });
    let (five_hour, seven_day) = (win("fh", FIVE_HOUR_MS), win("sd", SEVEN_DAY_MS));
    (five_hour.is_some() || seven_day.is_some()).then_some(Sample { at, five_hour, seven_day, source: Source::Desktop })
}

/// Doplní chybějící `resets_at` z jiných vzorků — jen když tam okno končí PO čase `best` (tedy v čase
/// `best` ještě běželo a jde o totéž okno). Procenta se nikdy nepřebírají.
pub fn fill_resets(mut best: Sample, others: &[Option<Sample>]) -> Sample {
    let pick = |cur: Option<Window>, get: fn(&Sample) -> Option<Window>| -> Option<Window> {
        let mut w = cur?;
        if w.resets_at.is_none() {
            w.resets_at = others.iter().flatten().filter_map(|o| get(o)?.resets_at).filter(|r| *r > best.at).max();
        }
        Some(w)
    };
    best.five_hour = pick(best.five_hour, |s| s.five_hour);
    best.seven_day = pick(best.seven_day, |s| s.seven_day);
    best
}

/// Kde aplikace Claude drží `plan-usage-history.json`, v pořadí zkoušení: MSIX instalace
/// (`%LOCALAPPDATA%\Claude-Data\<balíček>\`), klasická instalace (`%APPDATA%\Claude\`) a její virtualizovaná
/// kopie v MSIX bublině (`%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude\`).
/// `data_dirs` = podsložky `Claude-Data`, `packages` = složky `Packages\Claude_*` (výčet dělá volající).
pub fn desktop_candidates(roaming: &Path, data_dirs: &[PathBuf], packages: &[PathBuf]) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = data_dirs.iter().map(|d| d.join(DESKTOP_FILE)).collect();
    v.push(roaming.join("Claude").join(DESKTOP_FILE));
    v.extend(packages.iter().map(|p| p.join("LocalCache").join("Roaming").join("Claude").join(DESKTOP_FILE)));
    v
}

/// Je složka balíčku aplikace Claude (`Claude_<hash>`)? Jiné balíčky se nečtou.
pub fn is_claude_package(name: &str) -> bool {
    name.strip_prefix("Claude_").is_some_and(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Proč token nejde použít.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenErr {
    /// soubor / klíč chybí (Claude Code tu není přihlášený)
    Missing,
    /// `expiresAt` už prošel — obnoví ho až Claude Code (Ledge refresh nedělá)
    Expired,
}

/// Přístupový token z textu `~/.claude/.credentials.json`: jen `claudeAiOauth.accessToken`; `expiresAt`
/// (ms) ≤ `now + EXPIRY_MARGIN_MS` = vypršel, chybějící / 0 = nevíme (zkusí se, server případně řekne 401).
/// Refresh token se nečte a nikam nevrací.
pub fn token_of(text: &str, now: i64) -> Result<String, TokenErr> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|_| TokenErr::Missing)?;
    let o = v.get("claudeAiOauth").ok_or(TokenErr::Missing)?;
    let tok = o.get("accessToken").and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty() && t.is_ascii() && !t.contains(char::is_whitespace)).ok_or(TokenErr::Missing)?;
    if o.get("expiresAt").and_then(to_ms).is_some_and(|e| e <= now + EXPIRY_MARGIN_MS) {
        return Err(TokenErr::Expired);
    }
    Ok(tok.to_string())
}

/// Výsledek jednoho dotazu (pro stav v kartě a odstup dalšího pokusu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    /// není přihlášení Claude Code (soubor / token chybí)
    NoLogin,
    /// token vypršel nebo ho server odmítl (401 / 403)
    Expired,
    /// Anthropic brzdí (429)
    Busy,
    /// síť, 5xx, nečekaná odpověď
    Error,
}

impl Outcome {
    /// Klíč pro UI (`LimitsView.fetch`).
    pub fn key(self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::NoLogin => "noLogin",
            Outcome::Expired => "expired",
            Outcome::Busy | Outcome::Error => "error",
        }
    }

    /// Odstup dalšího pokusu (ms) po tomhle výsledku; `every_ms` = běžný interval.
    pub fn next_after_ms(self, every_ms: i64) -> i64 {
        match self {
            Outcome::Ok => every_ms,
            Outcome::Error => every_ms.max(15 * 60_000),
            Outcome::NoLogin | Outcome::Expired | Outcome::Busy => every_ms.max(30 * 60_000),
        }
    }
}

/// HTTP stav → výsledek (200 řeší volající parsováním).
pub fn outcome_of_status(status: u16) -> Outcome {
    match status {
        200 => Outcome::Ok,
        401 | 403 => Outcome::Expired,
        429 => Outcome::Busy,
        _ => Outcome::Error,
    }
}

/// Zeptat se teď? Ne při běžícím dotazu (do 30 s), ne dřív než `wait_ms` po posledním pokusu a ne, když
/// nejčerstvější vzorek (z kteréhokoli zdroje) je mladší než `every_ms` — stavový řádek pak stačí.
pub fn due(now: i64, attempt: Option<i64>, loading: bool, wait_ms: i64, newest_at: Option<i64>, every_ms: i64) -> bool {
    if newest_at.is_some_and(|a| now - a < every_ms) {
        return false;
    }
    match attempt {
        None => true,
        Some(a) if loading && now - a < 30_000 => false,
        Some(a) => now - a >= wait_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;
    /// (text, očekávaná procenta (5h, týden))
    type ApiCase<'a> = (&'a str, Option<(Option<f64>, Option<f64>)>);
    /// (pokus, běží, odstup, nejčerstvější vzorek) → due
    type DueCase = (Option<i64>, bool, i64, Option<i64>, bool);

    #[test]
    fn usage_api_table() {
        let full = r#"{"five_hour":{"utilization":8.0,"resets_at":"2027-01-15T19:20:00.029315+00:00","limit_dollars":null},
            "seven_day":{"utilization":44,"resets_at":"2027-01-18T04:00:00+00:00"},"seven_day_opus":null,"extra_usage":{"is_enabled":false}}"#;
        let s = from_usage_api(full, NOW).unwrap();
        assert_eq!(s.source, Source::Api);
        assert_eq!(s.at, NOW);
        assert_eq!(s.five_hour.unwrap().pct, 8.0);
        assert_eq!(s.five_hour.unwrap().resets_at, Some(chrono::DateTime::parse_from_rfc3339("2027-01-15T19:20:00.029315+00:00").unwrap().timestamp_millis()));
        assert_eq!(s.seven_day.unwrap().pct, 44.0);
        let cases: &[ApiCase] = &[
            (r#"{"five_hour":null,"seven_day":{"utilization":3,"resets_at":null}}"#, Some((None, Some(3.0)))),
            (r#"{"five_hour":{"utilization":null,"resets_at":null},"seven_day":null}"#, None),
            (r#"{"five_hour":{"utilization":-1},"seven_day":{"utilization":"x"}}"#, None),
            (r#"{"error":{"type":"authentication_error"}}"#, None),
            ("<html>", None),
            ("", None),
        ];
        for (txt, want) in cases {
            let got = from_usage_api(txt, NOW).map(|s| (s.five_hour.map(|w| w.pct), s.seven_day.map(|w| w.pct)));
            assert_eq!(&got, want, "{txt}");
        }
        // bez resets_at = nevíme
        assert_eq!(from_usage_api(r#"{"seven_day":{"utilization":3,"resets_at":null}}"#, NOW).unwrap().seven_day.unwrap().resets_at, None);
    }

    #[test]
    fn api_sample_round_trips_through_cache_file() {
        let s = from_usage_api(r#"{"five_hour":{"utilization":12.5,"resets_at":"2027-01-15T19:20:00+00:00"},"seven_day":null}"#, NOW).unwrap();
        let back = crate::limits::from_file(&to_file_api(&s), Source::StatusLine).unwrap();
        assert_eq!(back, s);
        // bez `via` zůstává zdroj volajícího (wrapper)
        let plain = crate::limits::from_file(&to_file(&s), Source::StatusLine).unwrap();
        assert_eq!(plain.source, Source::StatusLine);
    }

    #[test]
    fn desktop_history_table() {
        let h = |t1: i64, t2: i64| format!(r#"{{"version":2,"samples":[{{"t":{t1},"org":"o-1","u":{{"fh":5,"sd":51}}}},{{"t":{t2},"org":"o-1","u":{{"fh":62,"sd":41,"so":3}}}}]}}"#);
        // nejnovější vzorek vyhrává i mimo pořadí
        let s = from_desktop_history(&h(NOW - 60_000, NOW - 600_000), NOW).unwrap();
        assert_eq!((s.at, s.five_hour.unwrap().pct, s.seven_day.unwrap().pct), (NOW - 60_000, 5.0, 51.0));
        assert_eq!(s.source, Source::Desktop);
        assert_eq!(s.five_hour.unwrap().resets_at, None);
        // 6 h starý: 5h okno se určitě obnovilo, týden platí
        let s = from_desktop_history(&h(NOW - 7 * 3_600_000, NOW - 6 * 3_600_000), NOW).unwrap();
        assert_eq!((s.five_hour, s.seven_day.map(|w| w.pct)), (None, Some(41.0)));
        // 8 dní starý: nic
        assert_eq!(from_desktop_history(&h(NOW - 9 * 86_400_000, NOW - 8 * 86_400_000), NOW), None);
        // version 1 (hodnoty přímo ve vzorku, null = okno chybí)
        let v1 = format!(r#"{{"version":1,"samples":[{{"t":{},"fh":null,"sd":20}}]}}"#, NOW - 1000);
        let s = from_desktop_history(&v1, NOW).unwrap();
        assert_eq!((s.five_hour, s.seven_day.map(|w| w.pct)), (None, Some(20.0)));
        for bad in ["", "{}", r#"{"samples":[]}"#, r#"{"samples":[{"u":{"fh":1}}]}"#, r#"{"samples":"x"}"#, "[1,2]"] {
            assert_eq!(from_desktop_history(bad, NOW), None, "{bad}");
        }
    }

    #[test]
    fn fill_resets_only_from_window_still_running() {
        let w = |pct: f64, r: Option<i64>| Some(Window { pct, resets_at: r });
        let desk = Sample { at: NOW, five_hour: w(30.0, None), seven_day: w(50.0, None), source: Source::Desktop };
        let older = Sample { at: NOW - 3_600_000, five_hour: w(20.0, Some(NOW + 600_000)), seven_day: w(48.0, Some(NOW - 1)), source: Source::StatusLine };
        let f = fill_resets(desk, &[Some(older), None]);
        assert_eq!(f.five_hour, w(30.0, Some(NOW + 600_000)), "5h okno pořád běží → reset převzat, procenta ne");
        assert_eq!(f.seven_day, w(50.0, None), "týdenní okno skončilo před vzorkem → nevíme");
        // vlastní reset se nepřepisuje
        let own = Sample { five_hour: w(30.0, Some(NOW + 5)), ..desk };
        assert_eq!(fill_resets(own, &[Some(older)]).five_hour, w(30.0, Some(NOW + 5)));
        // chybějící okno zůstane chybět
        let none = Sample { five_hour: None, ..desk };
        assert_eq!(fill_resets(none, &[Some(older)]).five_hour, None);
    }

    #[test]
    fn desktop_candidates_and_packages() {
        let r = PathBuf::from(r"C:\Users\uzivatel\AppData\Roaming");
        let c = desktop_candidates(&r, &[PathBuf::from(r"D:\x\Claude-Data\claude_ab12")], &[PathBuf::from(r"D:\p\Claude_ab12")]);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0], PathBuf::from(r"D:\x\Claude-Data\claude_ab12").join(DESKTOP_FILE));
        assert_eq!(c[1], r.join("Claude").join(DESKTOP_FILE));
        assert!(c[2].ends_with(Path::new("LocalCache").join("Roaming").join("Claude").join(DESKTOP_FILE)));
        for (name, ok) in [("Claude_ab12cd", true), ("Claude_", false), ("ClaudeX_ab", false), ("app_ab12", false), ("Claude_a/b", false), ("Claude_ab12.", false)] {
            assert_eq!(is_claude_package(name), ok, "{name}");
        }
    }

    #[test]
    fn token_table() {
        let c = |tok: &str, exp: &str| format!(r#"{{"claudeAiOauth":{{"accessToken":"{tok}","refreshToken":"r-smyslene","expiresAt":{exp}}}}}"#);
        assert_eq!(token_of(&c("tok-smyslene", &(NOW + 3_600_000).to_string()), NOW), Ok("tok-smyslene".into()));
        assert_eq!(token_of(&c("tok-smyslene", "0"), NOW), Ok("tok-smyslene".into()), "0 = nevíme");
        assert_eq!(token_of(&c("tok-smyslene", "null"), NOW), Ok("tok-smyslene".into()));
        assert_eq!(token_of(&c("tok-smyslene", &(NOW + 30_000).to_string()), NOW), Err(TokenErr::Expired));
        assert_eq!(token_of(&c("tok-smyslene", &(NOW - 1).to_string()), NOW), Err(TokenErr::Expired));
        assert_eq!(token_of(&c("", "0"), NOW), Err(TokenErr::Missing));
        assert_eq!(token_of(&c("tok s mezerou", "0"), NOW), Err(TokenErr::Missing));
        assert_eq!(token_of(r#"{"mcpOAuth":{}}"#, NOW), Err(TokenErr::Missing));
        assert_eq!(token_of("{rozbité", NOW), Err(TokenErr::Missing));
        // refresh token se nikdy nevrátí
        assert!(token_of(&c("tok-smyslene", "0"), NOW).unwrap() != "r-smyslene");
    }

    #[test]
    fn outcome_and_due_table() {
        assert_eq!(outcome_of_status(200), Outcome::Ok);
        assert_eq!(outcome_of_status(401), Outcome::Expired);
        assert_eq!(outcome_of_status(403), Outcome::Expired);
        assert_eq!(outcome_of_status(429), Outcome::Busy);
        assert_eq!(outcome_of_status(500), Outcome::Error);
        assert_eq!(outcome_of_status(302), Outcome::Error, "přesměrování se nesleduje");
        let e = 10 * 60_000;
        assert_eq!(Outcome::Ok.next_after_ms(e), e);
        assert_eq!(Outcome::Expired.next_after_ms(e), 30 * 60_000);
        assert_eq!(Outcome::Error.next_after_ms(e), 15 * 60_000);
        // (attempt, loading, wait, newest) → due
        let cases: &[DueCase] = &[
            (None, false, e, None, true),
            (None, false, e, Some(NOW - 60_000), false), // čerstvý vzorek ze stavového řádku stačí
            (None, false, e, Some(NOW - e), true),
            (Some(NOW - 10_000), true, e, None, false),  // běží
            (Some(NOW - 40_000), true, 0, None, true),   // zaseklý dotaz se odepíše
            (Some(NOW - e + 1), false, e, None, false),
            (Some(NOW - e), false, e, None, true),
            (Some(NOW - 20 * 60_000), false, 30 * 60_000, None, false), // po 401 delší odstup
        ];
        for &(a, l, w, n, want) in cases {
            assert_eq!(due(NOW, a, l, w, n, e), want, "{a:?} {l} {w} {n:?}");
        }
    }
}
