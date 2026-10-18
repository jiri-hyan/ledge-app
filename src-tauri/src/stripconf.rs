//! Config proužku (`strip`): které položky a v jakém pořadí, vlastní text, barva na položku a „v klidu
//! skryté“ položky. Re-export z `config.rs` (`config::StripConfig`, `config::StripItem`), takže se importy
//! jinde nemění. Čisté skládání segmentů (`compose`) volá `stripview::refresh_strip` — tady je jen logika
//! nad hotovými segmenty (tabulkové testy), žádný stav ani I/O.

use crate::configerr::{FieldError, fe};
use crate::strip::{Rgba, Segment};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Nejvýš tolik znaků (Unicode skalárů) má text položky `custom`.
pub const CUSTOM_MAX: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct StripConfig {
    /// Proužek nahoře uprostřed. false = zůstane jen svítící linka při událostech.
    pub enabled: bool,
    /// Které informace proužek ukazuje (pořadí = pořadí v proužku).
    pub items: Vec<StripItem>,
    /// Neprůhlednost pozadí proužku 0–1.
    pub opacity: f32,
    /// Monitor: "primary" nebo index od 0 (pořadí z Windows).
    pub monitor: String,
    /// Text položky `custom`: krátký text a/nebo emoji, nejvýš 24 znaků. Prázdný = položka nic neukáže.
    pub custom: String,
    /// Barva textu položky: klíč = položka (`clock`, `custom`, `mode`…), hodnota = předvolba
    /// (`accent` = akcent motivu, `text`, `muted`, `violet`, `blue`, `teal`, `green`, `amber`, `red`).
    /// Chybějící = výchozí barva položky. Text se vždy zesvětlí / ztmaví na čitelný kontrast.
    pub colors: BTreeMap<StripItem, StripColor>,
    /// Položky, které se v klidu schovají a ukážou se jen, když se něco děje (běží server, pracuje nebo
    /// čeká Claude, běží časovač, hraje hudba, limity ≥ 70 %, připomínka, barevná linka). Smysl má u `mode`,
    /// `clock`, `custom`, `limits`; `running`, `claude`, `waiting`, `repos`, `timer`, `media` se v klidu
    /// neukazují samy.
    /// Výchozí prázdné (dnešní chování).
    pub quiet: Vec<StripItem>,
    /// Revize výchozích položek, které už byly do `items` jednou doplněny (`attention::upgrade_items`).
    /// Ručně neměnit; 0 = starý config → při načtení se doplní `waiting`. Chybějící v souboru = 0
    /// (ne výchozí hodnota struktury — ta má aktuální revizi pro nový config).
    #[serde(default)]
    pub items_rev: u32,
    /// Styl zavřeného Ledge: `classic` (výchozí = proužek podle `items`, tvar `appearance.strip.style`),
    /// `island`, `dots`, `line`, `icons`, `clock`, `corner`, `ghost`, `bar`, `notch`, `progress`, `ticker`.
    /// Ostatní styly než `classic` samy určí tvar, místo i obsah podle toho, co se děje (`looks.rs`);
    /// `items`, `quiet`, `colors`, `custom` a `opacity` platí jen pro `classic`.
    pub look: crate::looks::Look,
    /// Pokročilé u stylů mimo `classic`: co styl nemá ukazovat (`claude` = pracuje / hotovo, `waiting`,
    /// `running`, `repos`, `timer`, `media`, `limits`, `clock`). Výchozí prázdné = všechno.
    pub look_hide: Vec<StripItem>,
}

impl Default for StripConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            items: vec![StripItem::Mode, StripItem::Clock, StripItem::Running, StripItem::Claude, StripItem::Waiting, StripItem::Repos],
            opacity: 0.82,
            monitor: "primary".into(),
            custom: String::new(),
            colors: BTreeMap::new(),
            quiet: Vec::new(),
            items_rev: crate::attention::ITEMS_REV,
            look: crate::looks::Look::Classic,
            look_hide: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub enum StripItem {
    Mode,
    /// čas + den a datum
    Clock,
    /// běžící servery jménem a portem
    Running,
    Claude,
    /// kolik rep má neuložené / nepushnuté změny
    Repos,
    /// limity Claude 5h / týden (`limitslive::strip_segments`); výchozí vypnuto
    Limits,
    /// kolik Claude session čeká na tebe (schválení nebo otázka); 0 = nic
    Waiting,
    /// vlastní text (`strip.custom`)
    Custom,
    /// časovač: odpočet / stopky po minutách (`timer.rs`); bez položky ho přidá `timer.strip`
    Timer,
    /// právě hraje: „♪ Název · Interpret“ jen při přehrávání (`media::strip_segment`); výchozí vypnuto
    Media,
}

/// Předvolby barvy položky (žádný volný hex — kontrast hlídá `appearance::geom::readable`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StripColor {
    /// akcent motivu (nebo vlastní `appearance.accent`)
    Accent,
    /// jasný text
    Text,
    /// tlumený text
    Muted,
    Violet,
    Blue,
    Teal,
    Green,
    Amber,
    Red,
}

impl StripColor {
    #[cfg(test)]
    pub const ALL: [StripColor; 9] = [Self::Accent, Self::Text, Self::Muted, Self::Violet, Self::Blue, Self::Teal, Self::Green, Self::Amber, Self::Red];

    /// Základní barva předvolby (`accent` = akcent motivu); čitelnost řeší až `geom::readable`.
    pub fn base(self, accent: Rgba) -> Rgba {
        match self {
            Self::Accent => accent,
            Self::Text => Rgba::TEXT,
            Self::Muted => Rgba::DIM,
            Self::Violet => Rgba::VIOLET,
            Self::Blue => Rgba::BLUE,
            Self::Teal => Rgba::TEAL,
            Self::Green => Rgba::GREEN,
            Self::Amber => Rgba::AMBER,
            Self::Red => Rgba::RED,
        }
    }
}

/// Validace nových polí (chyby s polem pro Nastavení).
pub fn validate(s: &StripConfig) -> Result<(), FieldError> {
    let n = s.custom.chars().count();
    if n > CUSTOM_MAX {
        return Err(fe(
            "strip.custom",
            trf!("strip.custom: nejvýš {CUSTOM_MAX} znaků (je jich {n}).", "strip.custom: at most {CUSTOM_MAX} characters (there are {n})."),
        ));
    }
    Ok(())
}

/// Segment položky `custom`: oříznutý text (bez řídicích znaků, nejvýš `CUSTOM_MAX`); prázdný = nic.
pub(crate) fn custom_segment(text: &str) -> Option<Segment> {
    let clean: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let t: String = clean.trim().chars().take(CUSTOM_MAX).collect();
    let t = t.trim_end();
    (!t.is_empty()).then(|| Segment { text: t.to_string(), color: Rgba::TEXT, dot: None, strong: false, pill: false })
}

/// Je položka právě „událost“ (proužek se probudí z klidu)? Stálé položky (režim, hodiny, vlastní text)
/// nikdy; repa také ne — neuložená práce je stav, který trvá, ne událost. Limity jen při ≥ 70 %
/// (jantarová / červená tečka z `limitslive::strip_segments`). Právě hraje jen při přehrávání (pauza /
/// nic = žádný segment).
pub(crate) fn is_event(item: StripItem, segs: &[Segment]) -> bool {
    match item {
        StripItem::Mode | StripItem::Clock | StripItem::Custom | StripItem::Repos => false,
        StripItem::Limits => segs.iter().any(|s| s.dot == Some(Rgba::AMBER) || s.dot == Some(Rgba::RED)),
        StripItem::Running | StripItem::Claude | StripItem::Waiting | StripItem::Timer | StripItem::Media => !segs.is_empty(),
    }
}

/// Složí segmenty položek do proužku: „v klidu skryté“ položky (`quiet`) vynechá, když se nic neděje
/// (`busy_extra` = připomínka / barevná linka / jiná událost mimo položky), a položkám s volbou barvy
/// přebarví text (`color_of` = předvolba → čitelná barva pro motiv).
pub(crate) fn compose(parts: Vec<(StripItem, Vec<Segment>)>, s: &StripConfig, busy_extra: bool, color_of: impl Fn(StripColor) -> Rgba) -> Vec<Segment> {
    let busy = busy_extra || parts.iter().any(|(it, segs)| is_event(*it, segs));
    let mut out = Vec::new();
    for (it, mut segs) in parts {
        if !busy && s.quiet.contains(&it) {
            continue;
        }
        if let Some(c) = s.colors.get(&it) {
            let col = color_of(*c);
            for x in &mut segs {
                x.color = col;
            }
        }
        out.extend(segs);
    }
    out
}

/// Položky v pořadí proužku + automaticky přidaný časovač (běží, `timer.strip` zapnuté a položka
/// `timer` v `items` chybí) — na začátek, ať je vidět i při plném proužku.
pub(crate) fn effective_items(items: &[StripItem], timer_active: bool, timer_auto: bool) -> Vec<StripItem> {
    let mut v = items.to_vec();
    if timer_active && timer_auto && !v.contains(&StripItem::Timer) {
        v.insert(0, StripItem::Timer);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::Theme;
    use crate::appearance::geom::{contrast, ink, palette};
    use StripItem::*;

    fn seg(text: &str, dot: Option<Rgba>) -> Segment {
        Segment { text: text.into(), color: Rgba::DIM, dot, strong: false, pill: false }
    }
    fn texts(v: &[Segment]) -> Vec<&str> {
        v.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn serde_old_and_new_configs() {
        // starý config: nová pole chybí → výchozí (prázdný text, žádné barvy, nic skrytého)
        let s: StripConfig = serde_json::from_str(r#"{"items":["mode","clock"],"itemsRev":1}"#).unwrap();
        assert_eq!(s.items, vec![Mode, Clock]);
        assert_eq!((s.custom.as_str(), s.colors.len(), s.quiet.len()), ("", 0, 0));
        // nový tvar se čte i zapisuje camelCase, mapa barev má klíče jako položky
        let json = r#"{"items":["custom","timer"],"custom":"🎯 Deep work","colors":{"custom":"violet","clock":"accent"},"quiet":["clock","limits"]}"#;
        let s: StripConfig = serde_json::from_str(json).unwrap();
        assert_eq!(s.items, vec![Custom, Timer]);
        assert_eq!(s.colors.get(&Custom), Some(&StripColor::Violet));
        assert_eq!(s.colors.get(&Clock), Some(&StripColor::Accent));
        assert_eq!(s.quiet, vec![Clock, Limits]);
        let back = serde_json::to_value(&s).unwrap();
        assert_eq!(back["colors"]["custom"], "violet");
        assert_eq!(back["items"][1], "timer");
        // neznámá barva / položka = chyba (jako dřív u neznámé položky), ne tiché zahození
        assert!(serde_json::from_str::<StripConfig>(r#"{"colors":{"clock":"pink"}}"#).is_err());
        assert!(serde_json::from_str::<StripConfig>(r#"{"quiet":["nic"]}"#).is_err());
        // právě hraje: camelCase `media`, výchozí proužek ji nemá (starý config beze změny)
        let s: StripConfig = serde_json::from_str(r#"{"items":["clock","media"],"colors":{"media":"teal"},"quiet":["media"]}"#).unwrap();
        assert_eq!((s.items.as_slice(), s.colors.get(&Media), s.quiet.as_slice()), (&[Clock, Media][..], Some(&StripColor::Teal), &[Media][..]));
        assert_eq!(serde_json::to_value(&s).unwrap()["items"][1], "media");
        assert!(!StripConfig::default().items.contains(&Media));
    }

    #[test]
    fn upgrade_items_still_works_with_new_fields() {
        // migrace `waiting` (attention::upgrade_items) se novými poli nemění — přes config::load
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.json");
        std::fs::write(&p, r#"{"strip":{"items":["mode","claude","custom"],"custom":"ahoj"}}"#).unwrap();
        let cfg = crate::config::load(&p).unwrap().unwrap();
        assert_eq!(cfg.strip.items, vec![Mode, Claude, Waiting, Custom]);
        assert_eq!(cfg.strip.custom, "ahoj");
        // `media` v položkách migraci nevadí a sama se nikam nedoplní (výchozí vypnutá, ITEMS_REV beze změny)
        std::fs::write(&p, r#"{"strip":{"items":["claude","media"]}}"#).unwrap();
        assert_eq!(crate::config::load(&p).unwrap().unwrap().strip.items, vec![Claude, Waiting, Media]);
        std::fs::write(&p, r#"{"strip":{"items":["mode","claude"]}}"#).unwrap();
        assert!(!crate::config::load(&p).unwrap().unwrap().strip.items.contains(&Media));
    }

    #[test]
    fn custom_text_validation_and_segment() {
        let cases: [(&str, Option<&str>); 6] = [
            ("", None),
            ("   ", None),
            ("  🎯 Fokus  ", Some("🎯 Fokus")),
            ("a\nb", Some("a b")),
            ("123456789012345678901234", Some("123456789012345678901234")),
            ("1234567890123456789012345678", Some("123456789012345678901234")),
        ];
        for (inp, want) in cases {
            assert_eq!(custom_segment(inp).map(|s| s.text), want.map(String::from), "{inp:?}");
        }
        let mut s = StripConfig { custom: "x".repeat(CUSTOM_MAX), ..StripConfig::default() };
        assert!(validate(&s).is_ok());
        s.custom = "é".repeat(CUSTOM_MAX + 1);
        assert_eq!(validate(&s).unwrap_err().field.as_deref(), Some("strip.custom"));
        // emoji = jeden znak (Unicode skalár), ne bajty
        s.custom = "🎯".repeat(CUSTOM_MAX);
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn event_table() {
        let amber = [seg("5h 72 %", Some(Rgba::AMBER))];
        let green = [seg("5h 10 %", Some(Rgba::GREEN))];
        let one = [seg("x", None)];
        let cases: [(StripItem, &[Segment], bool); 12] = [
            (Mode, &one, false),
            (Clock, &one, false),
            (Custom, &one, false),
            (Repos, &one, false),
            (Running, &one, true),
            (Running, &[], false),
            (Claude, &one, true),
            (Timer, &one, true),
            (Limits, &amber, true),
            (Limits, &green, false),
            // právě hraje: segment existuje jen při přehrávání → událost; pauza / nic = žádný segment = klid
            (Media, &one, true),
            (Media, &[], false),
        ];
        for (it, segs, want) in cases {
            assert_eq!(is_event(it, segs), want, "{it:?} {segs:?}");
        }
    }

    #[test]
    fn compose_quiet_and_colors() {
        let parts = |running: bool| -> Vec<(StripItem, Vec<Segment>)> {
            vec![
                (Mode, vec![seg("Práce", None)]),
                (Clock, vec![seg("08:31", None)]),
                (Running, if running { vec![seg("web :5173", Some(Rgba::GREEN))] } else { vec![] }),
                (Custom, vec![seg("🎯", None)]),
            ]
        };
        let red = |_: StripColor| Rgba::RED;
        // (název, quiet, běží server, jiná událost, očekávané texty)
        type Case<'a> = (&'a str, Vec<StripItem>, bool, bool, Vec<&'a str>);
        let cases: [Case; 5] = [
            ("výchozí: nic skrytého", vec![], false, false, vec!["Práce", "08:31", "🎯"]),
            ("klid: skryté hodiny zmizí", vec![Clock, Custom], false, false, vec!["Práce"]),
            ("událost položky: hodiny zpět", vec![Clock, Custom], true, false, vec!["Práce", "08:31", "web :5173", "🎯"]),
            ("událost mimo položky (připomínka, linka)", vec![Clock], false, true, vec!["Práce", "08:31", "🎯"]),
            ("všechno skryté v klidu = prázdný proužek", vec![Mode, Clock, Custom], false, false, vec![]),
        ];
        for (name, quiet, running, extra, want) in cases {
            let s = StripConfig { quiet, ..StripConfig::default() };
            assert_eq!(texts(&compose(parts(running), &s, extra, red)), want, "{name}");
        }
        // právě hraje = událost: skryté hodiny se ukážou; pauza / nic (žádný segment) = klid
        let s = StripConfig { quiet: vec![Clock], ..StripConfig::default() };
        let media = |playing: bool| vec![(Clock, vec![seg("08:31", None)]), (Media, if playing { vec![seg("♪ Song", None)] } else { vec![] })];
        assert_eq!(texts(&compose(media(true), &s, false, red)), vec!["08:31", "♪ Song"]);
        assert!(compose(media(false), &s, false, red).is_empty());
        // barva přebarví jen text vybrané položky, tečku stavu nechá
        let mut s = StripConfig::default();
        s.colors.insert(Clock, StripColor::Teal);
        s.colors.insert(Running, StripColor::Violet);
        let out = compose(parts(true), &s, false, |c| c.base(Rgba::BLUE));
        assert_eq!(out[0].color, Rgba::DIM);
        assert_eq!(out[1].color, Rgba::TEAL);
        assert_eq!((out[2].color, out[2].dot), (Rgba::VIOLET, Some(Rgba::GREEN)));
    }

    #[test]
    fn timer_auto_item() {
        let base = [Mode, Clock];
        type Case<'a> = (&'a str, &'a [StripItem], bool, bool, Vec<StripItem>);
        let cases: [Case; 5] = [
            ("neběží: beze změny", &base, false, true, vec![Mode, Clock]),
            ("běží: na začátek", &base, true, true, vec![Timer, Mode, Clock]),
            ("běží, auto vypnuté", &base, true, false, vec![Mode, Clock]),
            ("už v položkách: na svém místě", &[Mode, Timer], true, true, vec![Mode, Timer]),
            ("prázdný proužek", &[], true, true, vec![Timer]),
        ];
        for (name, items, active, auto, want) in cases {
            assert_eq!(effective_items(items, active, auto), want, "{name}");
        }
    }

    #[test]
    fn every_preset_readable_in_every_theme() {
        // barva na položku nesmí rozbít kontrast textu proužku (≥ 4.5:1 na obou koncích přechodu)
        for t in Theme::ALL {
            let p = palette(t);
            for c in StripColor::ALL {
                let x = crate::appearance::geom::readable(c.base(p.accent[0]), &p);
                for bg in [p.top, p.bottom] {
                    let k = contrast(ink(x, p.light), bg);
                    assert!(k >= 4.5, "{t:?} {c:?}: {k:.2}");
                }
            }
        }
        // čitelná barva zůstane beze změny (tmavý motiv, jantar)
        let dark = palette(Theme::Dark);
        assert_eq!(crate::appearance::geom::readable(Rgba::AMBER, &dark), Rgba::AMBER);
    }
}
