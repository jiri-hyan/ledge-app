//! Hlavička widgetu → `WidgetMeta`. Stejné komentáře jako Script Commands (`scripts::meta::fields`),
//! widget pozná řádek `@ledge.widget <Název>`. Čisté: dostane text a druh souboru, na disk nesahá.
//! `.exe` hlavičku mít nemůže — má ji vedle sebe v textovém souboru `<jméno>.exe.ledge`.

use crate::i18n::tr;
use crate::scripts::meta::{self as smeta, Lang};
use serde::Serialize;

/// Nejkratší interval obnovy (s) — kratší se zaokrouhlí nahoru.
pub const MIN_INTERVAL_S: u32 = 30;
pub const MAX_INTERVAL_S: u32 = 24 * 3600;
pub const DEFAULT_INTERVAL_S: u32 = 300;
pub const DEFAULT_TIMEOUT_S: u32 = 10;
pub const MAX_TIMEOUT_S: u32 = 120;
pub const MAX_TITLE: usize = 60;
const MAX_DESC: usize = 160;
const MAX_ICON: usize = 8;

/// Druh souboru widgetu: skript (jazyk podle přípony) nebo program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Powershell,
    Cmd,
    Python,
    Node,
    Exe,
}

impl Kind {
    pub fn from_ext(ext: &str) -> Option<Kind> {
        if ext.eq_ignore_ascii_case("exe") {
            return Some(Kind::Exe);
        }
        Lang::from_ext(ext).map(|l| match l {
            Lang::Powershell => Kind::Powershell,
            Lang::Cmd => Kind::Cmd,
            Lang::Python => Kind::Python,
            Lang::Node => Kind::Node,
        })
    }

    /// Jazyk skriptu (`None` = program).
    pub fn lang(self) -> Option<Lang> {
        match self {
            Kind::Powershell => Some(Lang::Powershell),
            Kind::Cmd => Some(Lang::Cmd),
            Kind::Python => Some(Lang::Python),
            Kind::Node => Some(Lang::Node),
            Kind::Exe => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self.lang() {
            Some(l) => l.name(),
            None => tr("program", "program"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Kind::Powershell => "powershell",
            Kind::Cmd => "cmd",
            Kind::Python => "python",
            Kind::Node => "node",
            Kind::Exe => "exe",
        }
    }
}

/// Kde se widget smí ukázat: karta v šuplíku, položka proužku, nebo obojí.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Show {
    #[default]
    Card,
    Strip,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetMeta {
    pub title: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    /// interval obnovy (s), `MIN_INTERVAL_S`–`MAX_INTERVAL_S`
    pub interval_s: u32,
    pub show: Show,
    /// termín běhu (s), pak se strom procesu zabije
    pub timeout_s: u32,
    /// pevné argumenty
    pub args: Vec<String>,
}

/// Interval: `90` / `90s` / `5m` / `5min` / `1h` / `1h30m`. Kratší než 30 s = 30 s (minimum), delší
/// než den = chyba.
pub fn parse_interval(v: &str) -> Result<u32, String> {
    let s = v.trim().to_ascii_lowercase().replace(' ', "");
    let bad = || trf!("@ledge.refresh „{v}“ — čekám třeba 30s, 5m nebo 1h", "@ledge.refresh “{v}” — expected e.g. 30s, 5m or 1h");
    if s.is_empty() {
        return Err(bad());
    }
    let mut total: u64 = 0;
    let mut num = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        let mut unit = c.to_string();
        while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            unit.push(chars.next().unwrap_or_default());
        }
        let n: u64 = num.parse().map_err(|_| bad())?;
        num.clear();
        total += n * match unit.as_str() {
            "s" | "sec" => 1,
            "m" | "min" => 60,
            "h" | "hod" => 3600,
            _ => return Err(bad()),
        };
    }
    if !num.is_empty() {
        total += num.parse::<u64>().map_err(|_| bad())?;
    }
    if total > MAX_INTERVAL_S as u64 {
        return Err(tr("@ledge.refresh nejvýš 24h", "@ledge.refresh at most 24h").into());
    }
    Ok((total as u32).max(MIN_INTERVAL_S))
}

fn parse_show(v: &str) -> Result<Show, String> {
    let low = v.to_ascii_lowercase();
    let words: Vec<&str> = low.split([' ', ',', '|', '+']).filter(|w| !w.is_empty()).collect();
    let card = words.iter().any(|w| matches!(*w, "card" | "karta"));
    let strip = words.iter().any(|w| matches!(*w, "strip" | "proužek" | "prouzek"));
    let both = words.iter().any(|w| matches!(*w, "both" | "obojí" | "oboji"));
    if words.is_empty() {
        return Ok(Show::Card);
    }
    if words.iter().any(|w| !matches!(*w, "card" | "karta" | "strip" | "proužek" | "prouzek" | "both" | "obojí" | "oboji")) {
        return Err(trf!("@ledge.show „{v}“ — card, strip nebo both", "@ledge.show “{v}” — card, strip or both"));
    }
    Ok(if both || (card && strip) { Show::Both } else if strip { Show::Strip } else { Show::Card })
}

/// Metadata widgetu z textu hlavičky. `Ok(None)` = soubor nemá `@ledge.widget` (pomocný modul,
/// ne widget), `Err` = hlavička je, ale chybná (widget se ukáže s problémem a nespustí se).
pub fn parse(text: &str, lang: Lang) -> Result<Option<WidgetMeta>, String> {
    let f = smeta::fields(text, lang);
    let get = |k: &str| f.iter().find(|(sk, _)| sk == k).map(|(_, v)| v.as_str());
    let Some(w) = get("widget") else { return Ok(None) };
    let title = Some(w.trim()).filter(|t| !t.is_empty()).or_else(|| get("title").map(str::trim)).unwrap_or("");
    if title.is_empty() {
        return Err(tr("chybí název: @ledge.widget <Název>", "the name is missing: @ledge.widget <Name>").into());
    }
    if title.chars().count() > MAX_TITLE {
        return Err(trf!("název widgetu má nejvýš {MAX_TITLE} znaků", "a widget name has at most {MAX_TITLE} characters"));
    }
    let icon = match get("icon").filter(|v| !v.is_empty()) {
        None => None,
        Some(v) if v.chars().count() <= MAX_ICON && !v.contains(char::is_whitespace) => Some(v.to_string()),
        Some(_) => return Err(trf!("@ledge.icon = emoji nebo nejvýš {MAX_ICON} znaků bez mezer", "@ledge.icon = an emoji or at most {MAX_ICON} characters without spaces")),
    };
    let interval_s = match get("refresh").or_else(|| get("interval")) {
        None => DEFAULT_INTERVAL_S,
        Some(v) => parse_interval(v)?,
    };
    let timeout_s = match get("timeout") {
        None => DEFAULT_TIMEOUT_S,
        Some(v) => v
            .trim()
            .trim_end_matches('s')
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=MAX_TIMEOUT_S).contains(n))
            .ok_or_else(|| trf!("@ledge.timeout musí být 1–{MAX_TIMEOUT_S} s", "@ledge.timeout must be 1–{MAX_TIMEOUT_S} s"))?,
    };
    Ok(Some(WidgetMeta {
        title: title.to_string(),
        description: get("description").filter(|v| !v.is_empty()).map(|v| smeta::cut(v, MAX_DESC)),
        icon,
        interval_s,
        show: parse_show(get("show").unwrap_or(""))?,
        timeout_s,
        args: smeta::split_args(get("args").unwrap_or(""))?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(text: &str, lang: Lang) -> WidgetMeta {
        parse(text, lang).unwrap().unwrap()
    }

    #[test]
    fn interval_table() {
        let cases: &[(&str, Option<u32>)] = &[
            ("30s", Some(30)),
            ("90", Some(90)),
            ("5m", Some(300)),
            ("5 min", Some(300)),
            ("1h", Some(3600)),
            ("1h30m", Some(5400)),
            ("10s", Some(MIN_INTERVAL_S)),
            ("0", Some(MIN_INTERVAL_S)),
            ("24h", Some(86400)),
            ("25h", None),
            ("", None),
            ("pět minut", None),
            ("5x", None),
            ("m", None),
        ];
        for (v, want) in cases {
            assert_eq!(parse_interval(v).ok(), *want, "{v:?}");
        }
    }

    #[test]
    fn header_table() {
        // (jazyk, text, název / None = není widget)
        let cases: &[(Lang, &str, Option<&str>)] = &[
            (Lang::Powershell, "# @ledge.widget Kurz ČNB\n$x = 1", Some("Kurz ČNB")),
            (Lang::Python, "#!/usr/bin/env python3\n# @ledge.widget Disk\n", Some("Disk")),
            (Lang::Node, "// @ledge.widget Node\n", Some("Node")),
            (Lang::Cmd, ":: @ledge.widget Cmd\n", Some("Cmd")),
            (Lang::Cmd, "@ledge.widget Ne\n", None),
            // skript Script Commands (bez @ledge.widget) widget není
            (Lang::Powershell, "# @ledge.title Skript\n", None),
            // název z @ledge.title, když je @ledge.widget prázdné
            (Lang::Powershell, "# @ledge.widget\n# @ledge.title Z titulku\n", Some("Z titulku")),
            // sidecar k .exe: holé řádky bez komentáře
            (Lang::Python, "@ledge.widget Program\n@ledge.refresh 1m\n", Some("Program")),
        ];
        for (lang, text, want) in cases {
            let got = parse(text, *lang).unwrap_or_else(|e| panic!("{text:?}: {e}"));
            assert_eq!(got.map(|m| m.title).as_deref(), *want, "{text:?}");
        }
    }

    #[test]
    fn all_fields_and_defaults() {
        let m = ok("# @ledge.widget Docker\n# @ledge.description Běžící kontejnery\n# @ledge.icon 🐳\n# @ledge.refresh 2m\n# @ledge.show both\n# @ledge.timeout 20\n# @ledge.args --all \"a b\"", Lang::Powershell);
        assert_eq!((m.title.as_str(), m.icon.as_deref(), m.interval_s, m.show, m.timeout_s), ("Docker", Some("🐳"), 120, Show::Both, 20));
        assert_eq!(m.description.as_deref(), Some("Běžící kontejnery"));
        assert_eq!(m.args, ["--all", "a b"]);
        let d = ok("# @ledge.widget X", Lang::Python);
        assert_eq!((d.interval_s, d.show, d.timeout_s, d.args.len()), (DEFAULT_INTERVAL_S, Show::Card, DEFAULT_TIMEOUT_S, 0));
    }

    #[test]
    fn show_table() {
        let cases: &[(&str, Option<Show>)] = &[
            ("", Some(Show::Card)),
            ("card", Some(Show::Card)),
            ("strip", Some(Show::Strip)),
            ("both", Some(Show::Both)),
            ("card strip", Some(Show::Both)),
            ("card,strip", Some(Show::Both)),
            ("proužek", Some(Show::Strip)),
            ("menu", None),
        ];
        for (v, want) in cases {
            assert_eq!(parse_show(v).ok(), *want, "{v:?}");
        }
    }

    #[test]
    fn errors_table() {
        let cases: &[(&str, &str)] = &[
            ("# @ledge.widget   ", "@ledge.widget"),
            ("# @ledge.widget X\n# @ledge.refresh brzy", "brzy"),
            ("# @ledge.widget X\n# @ledge.refresh 48h", "24h"),
            ("# @ledge.widget X\n# @ledge.timeout 0", "timeout"),
            ("# @ledge.widget X\n# @ledge.timeout 999", "timeout"),
            ("# @ledge.widget X\n# @ledge.show všude", "show"),
            ("# @ledge.widget X\n# @ledge.icon dlouhá ikona", "@ledge.icon"),
            ("# @ledge.widget X\n# @ledge.args \"neuzavřené", "uvozovky"),
        ];
        for (text, want) in cases {
            let e = parse(text, Lang::Python).unwrap_err();
            assert!(e.contains(want), "{text:?} → {e}");
        }
        assert!(parse(&format!("# @ledge.widget {}", "x".repeat(MAX_TITLE + 1)), Lang::Python).is_err());
    }

    #[test]
    fn kinds() {
        assert_eq!(Kind::from_ext("PS1"), Some(Kind::Powershell));
        assert_eq!(Kind::from_ext("exe"), Some(Kind::Exe));
        assert_eq!(Kind::from_ext("mjs"), Some(Kind::Node));
        assert_eq!(Kind::from_ext("ledge"), None);
        assert_eq!(Kind::Exe.lang(), None);
    }
}
