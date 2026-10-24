//! Stav incidentů Anthropicu v kartě Claude (`smart.incidents`, výchozí VYPNUTO): „padá služba, nebo já?“
//!
//! Jediné volání sítě: `GET https://status.anthropic.com/api/v2/incidents/unresolved.json` (veřejné JSON API
//! stránky stavu, formát Statuspage) — bez cookies, bez hlavičky s identitou kromě `User-Agent: Ledge/<verze>`,
//! žádná data uživatele neodcházejí. Dotaz se posílá jen když je volba zapnutá, šuplík je otevřený naplno a na
//! otevřené záložce je karta Claude s řádkem limitů (`maybe_refresh`: volá ho `emit` při každém probuzení a
//! otevření šuplíku — žádný vlastní časovač), nejvýš jednou za `smart.incidents.everyMin` minut; termín 5 s,
//! odpověď ≤ 256 kB, přesměrování jen v rámci Anthropicu (stránka dnes vede na status.claude.com). Zobrazí se
//! jen PROBÍHAJÍCÍ incident (nejzávažnější, jinak nejčerstvější) — žádný incident nebo chyba = karta nic
//! neukáže (nevymýšlíme „vše v pořádku“). Text incidentu je cizí vstup: ořezaný, bez řídicích znaků.
//!
//! Čisté: `parse`, `due`, `redirect_ok`, `clean` (tabulkové testy nad fixture). Stav je jen v paměti.

use crate::state::{core, log, mark_dirty, now_ms};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub(crate) const URL: &str = "https://status.anthropic.com/api/v2/incidents/unresolved.json";
const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BODY: usize = 256 * 1024;
const MAX_NAME: usize = 140;

/// Probíhající incident pro kartu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    /// název incidentu (anglicky, jak ho zveřejnil Anthropic), ořezaný
    pub name: String,
    /// investigating | identified | monitoring | other (jen to, co zná UI)
    pub status: &'static str,
    /// minor | major | critical | none
    pub impact: &'static str,
    /// kolik dalších probíhajících incidentů je kromě tohoto
    pub more: u32,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    incidents: Vec<RawIncident>,
}

#[derive(Deserialize)]
struct RawIncident {
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    impact: String,
    #[serde(default)]
    updated_at: String,
}

fn impact_rank(i: &str) -> (u8, &'static str) {
    match i {
        "critical" => (3, "critical"),
        "major" => (2, "major"),
        "minor" => (1, "minor"),
        _ => (0, "none"),
    }
}

fn status_of(s: &str) -> &'static str {
    match s {
        "investigating" => "investigating",
        "identified" => "identified",
        "monitoring" => "monitoring",
        _ => "other",
    }
}

/// Neviditelné formátovací znaky (kategorie Cf): nulové mezery a spojky, značky směru textu (U+202A–U+202E,
/// U+2066–U+2069 umí v cizím textu převrátit pořadí znaků), BOM, měkký spojovník, značky jazyka. Do UI nepatří.
pub(crate) fn is_format_char(c: char) -> bool {
    matches!(c as u32,
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x180E
        | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB
        | 0xE0001 | 0xE0020..=0xE007F)
}

/// Řídicí znaky pryč (na mezeru), formátovací znaky (Cf) bez náhrady, mezery sloučené, nejvýš `MAX_NAME` znaků
/// (cizí text nikdy nejde do UI syrově).
pub(crate) fn clean(s: &str) -> String {
    let flat: String = s.chars().filter(|c| !is_format_char(*c)).map(|c| if c.is_control() { ' ' } else { c }).collect();
    let flat = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= MAX_NAME {
        return flat;
    }
    let cut: String = flat.chars().take(MAX_NAME - 1).collect();
    format!("{}…", cut.trim_end())
}

/// Odpověď API → nejzávažnější probíhající incident (při shodě nejčerstvěji upravený); žádný = `None`.
/// Vyřešené (`resolved`, `postmortem`) se přeskočí i kdyby je API vrátilo. Rozbitý JSON = chyba.
pub(crate) fn parse(json: &str) -> Result<Option<Incident>, String> {
    let raw: Raw = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut live: Vec<&RawIncident> = raw.incidents.iter().filter(|i| !matches!(i.status.as_str(), "resolved" | "postmortem") && !clean(&i.name).is_empty()).collect();
    let total = live.len() as u32;
    // nejzávažnější první, při shodě nejčerstvější (ISO čas se řadí jako text)
    live.sort_by(|a, b| impact_rank(&b.impact).0.cmp(&impact_rank(&a.impact).0).then_with(|| b.updated_at.cmp(&a.updated_at)));
    Ok(live.first().map(|i| Incident { name: clean(&i.name), status: status_of(&i.status), impact: impact_rank(&i.impact).1, more: total - 1 }))
}

/// Smí se přesměrování na tenhle host? Jen Anthropic (stránka stavu dnes vede na `status.claude.com`), https.
pub(crate) fn redirect_ok(scheme: &str, host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    scheme == "https" && ["anthropic.com", "claude.com", "claude.ai"].iter().any(|d| h == *d || h.ends_with(&format!(".{d}")))
}

/// Je čas se zeptat znovu? Ne při běžícím dotazu (do 30 s; zaseklý se odepíše), jinak po `every_min` od
/// posledního POKUSU (i neúspěšného — chyba nezkracuje odstup).
pub(crate) fn due(now: i64, attempt_ms: Option<i64>, loading: bool, every_min: u32) -> bool {
    match attempt_ms {
        None => true,
        Some(a) if loading && now - a < 30_000 => false,
        Some(a) => now - a >= every_min as i64 * 60_000,
    }
}

/// Nejkratší odstup dvou pokusů o síť vůbec, i po změně volby v Nastavení (klikání na přepínač nesmí střílet do sítě).
pub(crate) const MIN_RETRY_MS: i64 = 60_000;

/// Smí se po posledním pokusu (`last_ms`, přežije vypnutí volby) zase sáhnout na síť? Spodní mez `MIN_RETRY_MS`.
pub(crate) fn retry_floor_ok(now: i64, last_ms: Option<i64>) -> bool {
    last_ms.is_none_or(|l| now - l >= MIN_RETRY_MS)
}

#[derive(Default)]
struct State {
    incident: Option<Incident>,
    attempt_ms: Option<i64>,
    /// čas posledního pokusu; na rozdíl od `attempt_ms` se nenuluje změnou volby
    last_ms: Option<i64>,
    loading: bool,
}

static STATE: Mutex<State> = Mutex::new(State { incident: None, attempt_ms: None, last_ms: None, loading: false });

/// Pro snapshot — jen paměť; vypnuto = nic.
pub(crate) fn view(enabled: bool) -> Option<Incident> {
    if !enabled {
        return None;
    }
    STATE.lock().incident.clone()
}

/// Je co aktualizovat? Zapnuto, šuplík naplno, na otevřené záložce karta Claude s řádkem limitů.
fn wanted(opening: bool) -> Option<u32> {
    use crate::drawer::View;
    let c = core();
    let cfg = c.cfg.read();
    let on = cfg.smart.incidents.enabled && cfg.drawer.sections.claude_limits;
    let open = opening || c.drawer.view() == View::Full;
    (on && open && !c.dead() && crate::layout::page_has(&cfg.layout, &c.mode.lock(), "claude")).then_some(cfg.smart.incidents.every_min)
}

/// Zeptat se, když je čas (volá `emit` při probuzení, otevření šuplíku a změna záložky). Jinak nic.
pub(crate) fn maybe_refresh(opening: bool) {
    let Some(every) = wanted(opening) else { return };
    let now = now_ms();
    {
        let mut st = STATE.lock();
        if !due(now, st.attempt_ms, st.loading, every) || !retry_floor_ok(now, st.last_ms) {
            return;
        }
        st.attempt_ms = Some(now);
        st.last_ms = Some(now);
        st.loading = true;
    }
    tauri::async_runtime::spawn(async move {
        let res = fetch().await;
        {
            let mut st = STATE.lock();
            if st.attempt_ms != Some(now) {
                return; // novější pokus
            }
            st.loading = false;
            match res {
                Ok(inc) => st.incident = inc,
                // chyba: starý údaj se nechá (je to naposledy známá pravda), do logu jen důvod
                Err(e) => log(&format!("incidenty: {e}")),
            }
        }
        mark_dirty();
    });
}

/// Volba se změnila (Nastavení): vypnuto = zapomenout; zapnuto = zeptat se hned (jednou), nejdřív však
/// `MIN_RETRY_MS` po posledním pokusu (`maybe_refresh` mez hlídá).
pub(crate) fn on_config(old: &crate::config::Config, new: &crate::config::Config) {
    if old.smart.incidents == new.smart.incidents {
        return;
    }
    if !new.smart.incidents.enabled {
        // zapomenout údaj i rozběhlý dotaz, ale ne čas posledního pokusu (jinak by vypnout+zapnout obešlo odstup)
        let last_ms = STATE.lock().last_ms;
        *STATE.lock() = State { last_ms, ..State::default() };
    } else {
        STATE.lock().attempt_ms = None;
        maybe_refresh(false);
    }
    mark_dirty();
}

async fn fetch() -> Result<Option<Incident>, String> {
    let policy = reqwest::redirect::Policy::custom(|a| {
        let ok = a.previous().len() < 3 && redirect_ok(a.url().scheme(), a.url().host_str().unwrap_or(""));
        if ok { a.follow() } else { a.stop() }
    });
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(policy)
        .user_agent(concat!("Ledge/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client.get(URL).header(reqwest::header::ACCEPT, "application/json").send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        buf.extend_from_slice(&chunk);
        if buf.len() > MAX_BODY {
            return Err("odpověď je příliš velká".into());
        }
    }
    parse(&String::from_utf8_lossy(&buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &str = include_str!("fixtures/statuspage-unresolved.json");

    #[test]
    fn parse_picks_most_severe_then_freshest() {
        let inc = parse(FIX).unwrap().unwrap();
        assert_eq!(inc.name, "Elevated errors on app-v2 requests");
        assert_eq!((inc.status, inc.impact, inc.more), ("identified", "major", 2));
    }

    #[test]
    fn parse_table() {
        type Want<'a> = Option<(&'a str, &'a str, &'a str, u32)>;
        let cases: [(&str, &str, Want); 6] = [
            ("žádný incident", r#"{"incidents":[]}"#, None),
            ("chybí pole", r#"{}"#, None),
            ("vyřešený se přeskočí", r#"{"incidents":[{"name":"x","status":"resolved","impact":"major","updated_at":"2026-10-23T10:00:00Z"}]}"#, None),
            ("jeden minor", r#"{"incidents":[{"name":"Slow","status":"monitoring","impact":"minor","updated_at":"2026-10-23T10:00:00Z"}]}"#, Some(("Slow", "monitoring", "minor", 0))),
            ("neznámý stav a dopad", r#"{"incidents":[{"name":"Odd","status":"scheduled","impact":"?","updated_at":""}]}"#, Some(("Odd", "other", "none", 0))),
            ("shoda dopadu → čerstvější", r#"{"incidents":[{"name":"Old","status":"identified","impact":"major","updated_at":"2026-10-23T09:00:00Z"},{"name":"New","status":"identified","impact":"major","updated_at":"2026-10-23T11:00:00Z"}]}"#, Some(("New", "identified", "major", 1))),
        ];
        for (name, json, want) in cases {
            let got = parse(json).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(got.as_ref().map(|i| (i.name.as_str(), i.status, i.impact, i.more)), want, "{name}");
        }
        assert!(parse("<html>").is_err());
    }

    #[test]
    fn foreign_text_is_cleaned() {
        assert_eq!(clean("a\u{0}b\n\tc   d"), "a b c d");
        let long = "x".repeat(500);
        let c = clean(&long);
        assert_eq!(c.chars().count(), MAX_NAME);
        assert!(c.ends_with('…'));
        assert_eq!(clean("  \n "), "");
    }

    #[test]
    fn format_chars_are_dropped() {
        // nulová mezera, ZWJ, LRM/RLM, bidi přepisy a izolace, BOM, měkký spojovník, značky jazyka
        let dirty = "A\u{200B}p\u{200D}i\u{200E} \u{202E}down\u{202C} \u{2066}x\u{2069}\u{FEFF}\u{00AD}y\u{E0041}";
        assert_eq!(clean(dirty), "Api down xy");
        assert_eq!(clean("\u{200B}\u{FEFF} \u{2060}"), "");
        // běžné znaky včetně češtiny a emoji beze spojky zůstanou
        assert_eq!(clean("Chyba přihlášení ✓"), "Chyba přihlášení ✓");
        assert!(!is_format_char('a') && !is_format_char('\u{00E1}') && is_format_char('\u{200F}') && is_format_char('\u{202A}'));
    }

    #[test]
    fn retry_floor_holds_after_config_change() {
        let m = 60_000;
        let cases = [("nikdy", 5 * m, None, true), ("před 10 s", 5 * m, Some(5 * m - 10_000), false), ("před 59 s", 5 * m, Some(5 * m - 59_000), false), ("před 60 s", 5 * m, Some(4 * m), true), ("před hodinou", 100 * m, Some(m), true)];
        for (name, now, last, want) in cases {
            assert_eq!(retry_floor_ok(now, last), want, "{name}");
        }
    }

    #[test]
    fn redirect_only_inside_anthropic() {
        let cases = [("https", "status.claude.com", true), ("https", "status.anthropic.com", true), ("https", "CLAUDE.COM", true), ("http", "status.claude.com", false), ("https", "claude.com.example.com", false), ("https", "evilclaude.com", false), ("https", "example.com", false), ("https", "", false)];
        for (scheme, host, want) in cases {
            assert_eq!(redirect_ok(scheme, host), want, "{scheme}://{host}");
        }
    }

    #[test]
    fn due_respects_interval_and_loading() {
        let m = 60_000;
        type Case<'a> = (&'a str, i64, Option<i64>, bool, u32, bool);
        let cases: [Case; 7] = [
            ("nikdy se neptal", 1_000_000, None, false, 10, true),
            ("před 9 min", 10 * m, Some(m), false, 10, false),
            ("před 10 min", 11 * m, Some(m), false, 10, true),
            ("běží dotaz 5 s", 5_000, Some(0), true, 10, false),
            ("zaseklý dotaz 31 s, ale interval neuplynul", 31_000, Some(0), true, 10, false),
            ("po chybě stejný odstup", 4 * m, Some(0), false, 5, false),
            ("interval 5 min uplynul", 5 * m, Some(0), false, 5, true),
        ];
        for (name, now, attempt, loading, every, want) in cases {
            assert_eq!(due(now, attempt, loading, every), want, "{name}");
        }
    }

    #[test]
    fn url_is_the_public_status_api_over_https() {
        assert!(URL.starts_with("https://status.anthropic.com/api/v2/"));
    }
}
