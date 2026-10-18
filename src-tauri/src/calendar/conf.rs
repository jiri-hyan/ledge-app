//! Config kalendářů (`personal.calendars`) a zacházení s adresou: tajná adresa ICS je **tajemství**
//! (kdo ji zná, čte celý kalendář). Proto: nikdy v logu ani v textu chyby, v UI jen doména (`domain`),
//! v cache na disku jen otisk (`key`). `webcal://` se bere jako `https://`, `http://` se odmítne.

use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Nejvíc kalendářů v configu.
pub const MAX_CALENDARS: usize = 8;
/// Nejdelší adresa (znaky).
pub const MAX_URL: usize = 2048;

/// Kalendář z odkazu ICS (Google: Nastavení kalendáře → „Tajná adresa ve formátu iCal“; Outlook:
/// Nastavení → Kalendář → Sdílené kalendáře → Publikovat kalendář → odkaz ICS).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct CalendarFeed {
    /// Název na kartě; prázdný = název z feedu (`X-WR-CALNAME`), jinak doména.
    pub name: String,
    /// Tajná adresa ICS — jen `https://` (nebo `webcal://`, bere se jako https). Ledge ji nikde
    /// neukazuje celou (jen doménu) a nelogovat ji.
    pub url: String,
    /// Barva kalendáře `#rrggbb`; `null` = barva podle pořadí (Nastavení přiřadí při přidání samo).
    pub color: Option<String>,
    /// Vypnutý kalendář se nestahuje ani neukazuje.
    pub enabled: bool,
}

impl Default for CalendarFeed {
    fn default() -> Self {
        Self { name: String::new(), url: String::new(), color: None, enabled: true }
    }
}

/// Adresa k použití: ořízne mezery, `webcal://` → `https://`; jen https s doménou, bez mezer.
/// Text chyby adresu nikdy neobsahuje.
pub fn normalize_url(raw: &str) -> Result<String, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err(tr("chybí adresa", "the address is missing").into());
    }
    let lower = t.to_ascii_lowercase();
    let rest = if let Some(i) = ["webcal://", "webcals://", "https://"].iter().find_map(|p| lower.starts_with(p).then_some(p.len())) {
        &t[i..]
    } else if lower.starts_with("http://") {
        return Err(tr("adresa musí být https:// (http by posílal tajnou adresu nešifrovaně)", "the address must be https:// (http would send the secret address unencrypted)").into());
    } else {
        return Err(tr("adresa musí začínat https:// nebo webcal://", "the address must start with https:// or webcal://").into());
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() || host.contains('@') || t.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(tr("neplatná adresa", "invalid address").into());
    }
    Ok(format!("https://{rest}"))
}

/// Jen doména (bez cesty, dotazu a portu) — to jediné z adresy smí UI ukázat.
pub fn domain(raw: &str) -> String {
    let t = raw.trim();
    let rest = t.split_once("://").map_or(t, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    host.rsplit_once('@').map_or(host, |(_, h)| h).split(':').next().unwrap_or("").to_ascii_lowercase()
}

/// Otisk adresy (SHA-256, 16 hex znaků) — klíč stavu a cache na disku místo adresy samotné.
pub fn key(url: &str) -> String {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(url.as_bytes());
    h.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Validace seznamu: (cesta pole, text). Text adresu nikdy neobsahuje.
pub fn validate(list: &[CalendarFeed]) -> Result<(), (String, String)> {
    if list.len() > MAX_CALENDARS {
        return Err(("personal.calendars".into(), trf!("personal.calendars: nejvýš {MAX_CALENDARS} kalendářů (je jich {}).", "personal.calendars: at most {MAX_CALENDARS} calendars (there are {}).", list.len())));
    }
    for (i, c) in list.iter().enumerate() {
        let n = i + 1;
        let label = if c.name.trim().is_empty() { trf!("č. {n}", "no. {n}") } else { c.name.clone() };
        if c.name.chars().count() > 60 {
            return Err((format!("personal.calendars.{i}.name"), trf!("Kalendář č. {n}: název je delší než 60 znaků.", "Calendar no. {n}: the name is longer than 60 characters.")));
        }
        if c.url.chars().count() > MAX_URL {
            return Err((format!("personal.calendars.{i}.url"), trf!("Kalendář {}: adresa je delší než {MAX_URL} znaků.", "Calendar {}: the address is longer than {MAX_URL} characters.", label)));
        }
        if let Err(why) = normalize_url(&c.url) {
            return Err((format!("personal.calendars.{i}.url"), trf!("Kalendář {}: {why}.", "Calendar {}: {why}.", label)));
        }
        if let Some(col) = &c.color {
            let ok = col.len() == 7 && col.starts_with('#') && col[1..].chars().all(|x| x.is_ascii_hexdigit());
            if !ok {
                return Err((format!("personal.calendars.{i}.color"), trf!("Kalendář {}: barva musí být #rrggbb.", "Calendar {}: the colour must be #rrggbb.", label)));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "https://calendar.google.com/calendar/ical/jana%40example.com/private-0123456789abcdef/basic.ics";

    #[test]
    fn normalize_table() {
        let cases: &[(&str, Option<&str>)] = &[
            (SECRET, Some(SECRET)),
            ("  webcal://outlook.office365.com/owa/calendar/abc/def/calendar.ics ", Some("https://outlook.office365.com/owa/calendar/abc/def/calendar.ics")),
            ("WEBCAL://x.cz/a.ics", Some("https://x.cz/a.ics")),
            ("HTTPS://x.cz/a.ics", Some("https://x.cz/a.ics")),
            ("http://x.cz/a.ics", None),
            ("ftp://x.cz/a.ics", None),
            ("x.cz/a.ics", None),
            ("https://", None),
            ("https:///a.ics", None),
            ("https://x.cz/a b.ics", None),
            ("https://user:heslo@x.cz/a.ics", None),
            ("", None),
        ];
        for (raw, want) in cases {
            assert_eq!(normalize_url(raw).ok().as_deref(), *want, "{raw}");
        }
        assert!(normalize_url("http://x.cz").unwrap_err().contains("https"));
    }

    #[test]
    fn domain_only_and_key_hides_url() {
        assert_eq!(domain(SECRET), "calendar.google.com");
        assert_eq!(domain("webcal://Outlook.Office365.com:443/owa/x?y=1"), "outlook.office365.com");
        assert_eq!(domain("https://u:p@x.cz/a"), "x.cz");
        assert_eq!(domain("nesmysl"), "nesmysl");
        let k = key(SECRET);
        assert_eq!(k.len(), 16);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(k, key("https://calendar.google.com/calendar/ical/x/basic.ics"));
        assert!(!SECRET.contains(&k));
    }

    #[test]
    fn validate_table_never_leaks_url() {
        let feed = |name: &str, url: &str, color: Option<&str>| CalendarFeed { name: name.into(), url: url.into(), color: color.map(Into::into), enabled: true };
        let cases: Vec<(Vec<CalendarFeed>, Option<&str>)> = vec![
            (vec![], None),
            (vec![feed("Osobní", SECRET, Some("#3b82f6"))], None),
            (vec![feed("Práce", "webcal://outlook.office365.com/x/calendar.ics", None)], None),
            // bez názvu stačí odkaz (název přijde z feedu)
            (vec![feed("", SECRET, None)], None),
            (vec![feed(&"n".repeat(61), SECRET, None)], Some("personal.calendars.0.name")),
            (vec![feed("A", SECRET, None), feed("B", "", None)], Some("personal.calendars.1.url")),
            (vec![feed("A", &SECRET.replace("https", "http"), None)], Some("personal.calendars.0.url")),
            (vec![feed("A", SECRET, Some("modrá"))], Some("personal.calendars.0.color")),
            (vec![feed("A", &format!("https://x.cz/{}", "a".repeat(MAX_URL)), None)], Some("personal.calendars.0.url")),
            ((0..=MAX_CALENDARS).map(|i| feed(&format!("K{i}"), SECRET, None)).collect(), Some("personal.calendars")),
        ];
        for (list, want) in cases {
            let got = validate(&list);
            assert_eq!(got.as_ref().err().map(|(p, _)| p.as_str()), want, "{:?}", list.iter().map(|c| &c.name).collect::<Vec<_>>());
            if let Err((_, msg)) = got {
                assert!(!msg.contains("private-0123456789abcdef") && !msg.contains("x.cz/"), "chyba nesmí obsahovat adresu: {msg}");
            }
        }
        // starý config bez pole `enabled` = zapnuto
        let c: CalendarFeed = serde_json::from_str(r#"{"name":"A","url":"https://x.cz/a.ics"}"#).unwrap();
        assert!(c.enabled && c.color.is_none());
    }
}
