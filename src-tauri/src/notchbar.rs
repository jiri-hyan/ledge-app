//! Vlastní lišta širokého notche (kolo 27, `layout.pages[].notchBar`): seznam sekcí lišty nahoře v šuplíku
//! (rozložení `appearance.layout = "notch"`), nezávisle na kartách záložky. Chybí = dnešní chování (UI lištu odvodí
//! z karet záložky a ty karty převezme, `src/notch/model.ts` `notchSections`). Typ, validace a to, jaká data
//! lišta potřebuje (odvozené `drawer.sections`, stahování kalendáře a limitů jen s viditelnou sekcí). Vzhledy sekcí
//! jsou registr v UI (`src/notch/designs.ts`); neznámé id vzhledu UI ignoruje (kreslí výchozí).
use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Sekce lišty (zrcadlo `BAR_IDS` v `src/notch/bar.ts`).
pub const SECTIONS: [&str; 7] = ["player", "days", "system", "volume", "limits", "weather", "meeting"];
/// Metriky sekce Systém (zrcadlo `METRICS`).
pub const METRICS: [&str; 4] = ["cpu", "ram", "disk", "net"];
/// Nejvýš sekcí na liště (víc se do šířky výřezu nevejde).
pub const MAX_SECTIONS: usize = 6;
const COLORS: [&str; 3] = ["accent", "load", "gradient"];

/// Tloušťka čáry grafu sekce Systém. Chybí = střední (dnešní).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LineWeight {
    Thin,
    Mid,
    Bold,
}

/// Jedna sekce lišty a její volby (které nedávají u sekce smysl, UI ignoruje).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BarSection {
    /// `player` (Právě hraje), `days` (Dny a čas), `system`, `volume` (hlasitost), `limits` (limit Claude),
    /// `weather` (počasí), `meeting` (příští schůzka z kalendáře).
    pub id: String,
    /// Vzhled sekce z registru UI (`src/notch/designs.ts`; chybí = výchozí).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<String>,
    /// Barva: `accent` (výchozí), `load` (podle zátěže zelená → jantar → červená), `gradient` (přechod) nebo `#rrggbb`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Tloušťka čáry grafů (Systém): `thin` | `mid` | `bold`. Chybí = střední.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<LineWeight>,
    /// Vyhlazené křivky grafů (Systém). Chybí = lomená čára (dnešní).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub smooth: bool,
    /// Metriky sekce Systém v pořadí: `cpu`, `ram`, `disk`, `net`. Prázdné = CPU, RAM, Disk (dnešní).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<String>,
    /// Vteřinová ručička analogových hodin (Dny a čas, vzhled `analog`; jen s otevřeným šuplíkem).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub seconds: bool,
}

/// Vlastní lišta záložky. Prázdný seznam = záložka lištu nemá (a karty nepřebírá).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct NotchBar {
    /// Sekce zleva doprava (nejvýš 6, každá jednou).
    pub sections: Vec<BarSection>,
}

fn hex_ok(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validace lišty záložky `i` (cesta k poli pro Nastavení).
pub fn validate(bar: &NotchBar, i: usize) -> Result<(), FieldError> {
    let at = |f: String| format!("layout.pages.{i}.notchBar.{f}");
    if bar.sections.len() > MAX_SECTIONS {
        return Err(fe(at("sections".into()), trf!("Lišta má nejvýš {} sekcí.", "The bar has at most {} sections.", MAX_SECTIONS)));
    }
    let mut seen = std::collections::HashSet::new();
    for (j, s) in bar.sections.iter().enumerate() {
        if !SECTIONS.contains(&s.id.as_str()) {
            return Err(fe(at(format!("sections.{j}.id")), trf!("Sekce lišty „{}“ neexistuje (jde: {}).", "Bar section “{}” doesn't exist (available: {}).", s.id, SECTIONS.join(", "))));
        }
        if !seen.insert(s.id.as_str()) {
            return Err(fe(at(format!("sections.{j}.id")), trf!("Sekce „{}“ je na liště dvakrát.", "Section “{}” is on the bar twice.", s.id)));
        }
        if let Some(c) = &s.color
            && !COLORS.contains(&c.as_str())
            && !hex_ok(c)
        {
            return Err(fe(at(format!("sections.{j}.color")), tr("Barva sekce je accent, load, gradient nebo #rrggbb.", "A section color is accent, load, gradient or #rrggbb.")));
        }
        let mut ms = std::collections::HashSet::new();
        for (k, m) in s.metrics.iter().enumerate() {
            if !METRICS.contains(&m.as_str()) || !ms.insert(m.as_str()) {
                return Err(fe(at(format!("sections.{j}.metrics.{k}")), trf!("Metrika „{}“ neexistuje nebo je dvakrát (jde: {}).", "Metric “{}” doesn't exist or is there twice (available: {}).", m, METRICS.join(", "))));
            }
        }
    }
    Ok(())
}

/// Potřebuje lišta `bar` data sekce `need`? `calendar` = schůzka, týden s tečkami nebo list kalendáře.
pub fn wants(bar: &NotchBar, need: &str) -> bool {
    bar.sections.iter().any(|s| match need {
        "calendar" => s.id == "meeting" || (s.id == "days" && matches!(s.design.as_deref(), Some("dots" | "sheet"))),
        _ => s.id == need,
    })
}

/// Chce některá vlastní lišta (na kterékoli záložce) data `need`? Bez rozložení notch nikdy — lišta se nekreslí.
pub fn any_wants(l: &crate::layout::LayoutConfig, notch: bool, need: &str) -> bool {
    notch && l.pages.iter().any(|p| p.notch_bar.as_ref().is_some_and(|b| wants(b, need)))
}

/// Chce lišta právě viditelné záložky data `need`? (stahování kalendáře / limitů jen tehdy)
pub fn page_wants(cfg: &crate::config::Config, mode: &str, need: &str) -> bool {
    notch_on(cfg) && crate::layout::page(&cfg.layout, mode).and_then(|p| p.notch_bar.as_ref()).is_some_and(|b| wants(b, need))
}

/// Čte právě viditelná záložka kalendář (karta Kalendář / Dnes / Týden, nebo vlastní lišta se schůzkou)?
pub fn calendar_visible(cfg: &crate::config::Config, mode: &str) -> bool {
    crate::layout::page_has_calendar(&cfg.layout, mode) || page_wants(cfg, mode, "calendar")
}

/// Je zapnuté rozložení notch (jen tehdy se lišta kreslí)?
pub fn notch_on(cfg: &crate::config::Config) -> bool {
    cfg.appearance.layout == crate::appearance::look::Layout::Notch
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bar(v: serde_json::Value) -> NotchBar {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn serde_vychozi_se_nezapisuje() {
        let b = bar(json!({"sections":[{"id":"system"},{"id":"days","design":"analog","seconds":true}]}));
        assert_eq!(serde_json::to_value(&b).unwrap(), json!({"sections":[{"id":"system"},{"id":"days","design":"analog","seconds":true}]}));
        let s = bar(json!({"sections":[{"id":"system","color":"load","weight":"bold","smooth":true,"metrics":["cpu","net"]}]}));
        assert_eq!(s.sections[0].weight, Some(LineWeight::Bold));
        assert_eq!(serde_json::to_value(&s).unwrap()["sections"][0]["metrics"], json!(["cpu", "net"]));
    }

    #[test]
    fn validace_tabulka() {
        let cases: &[(serde_json::Value, Option<&str>)] = &[
            (json!({"sections":[]}), None),
            (json!({"sections":[{"id":"player"},{"id":"limits"},{"id":"weather"},{"id":"meeting"}]}), None),
            (json!({"sections":[{"id":"nic"}]}), Some("layout.pages.1.notchBar.sections.0.id")),
            (json!({"sections":[{"id":"days"},{"id":"days"}]}), Some("layout.pages.1.notchBar.sections.1.id")),
            (json!({"sections":[{"id":"system","color":"#12ab9F"}]}), None),
            (json!({"sections":[{"id":"system","color":"modra"}]}), Some("layout.pages.1.notchBar.sections.0.color")),
            (json!({"sections":[{"id":"system","color":"#12ab9"}]}), Some("layout.pages.1.notchBar.sections.0.color")),
            (json!({"sections":[{"id":"system","metrics":["cpu","gpu"]}]}), Some("layout.pages.1.notchBar.sections.0.metrics.1")),
            (json!({"sections":[{"id":"system","metrics":["cpu","cpu"]}]}), Some("layout.pages.1.notchBar.sections.0.metrics.1")),
            (json!({"sections":[{"id":"player"},{"id":"days"},{"id":"system"},{"id":"volume"},{"id":"limits"},{"id":"weather"},{"id":"meeting"}]}), Some("layout.pages.1.notchBar.sections")),
        ];
        for (v, want) in cases {
            let got = validate(&bar(v.clone()), 1).err().and_then(|e| e.field);
            assert_eq!(got.as_deref(), *want, "{v}");
        }
    }

    #[test]
    fn stary_config_bez_listy() {
        let p: crate::layout::LayoutPage = serde_json::from_value(json!({"id":"prace","name":"","cards":[{"id":"system","size":"m"}]})).unwrap();
        assert_eq!(p.notch_bar, None);
        assert!(serde_json::to_value(&p).unwrap().get("notchBar").is_none(), "chybějící lišta se nezapisuje");
        let q: crate::layout::LayoutPage = serde_json::from_value(json!({"id":"prace","name":"","cards":[],"notchBar":{"sections":[]}})).unwrap();
        assert_eq!(q.notch_bar, Some(NotchBar::default()), "prázdná vlastní lišta se liší od chybějící");
    }

    #[test]
    fn derive_zapne_zdroje_sekci_jen_v_notch() {
        let mut cfg = crate::config::Config::default();
        for p in cfg.layout.pages.iter_mut() {
            p.cards.retain(|c| c.id != "media" && c.id != "weather");
        }
        cfg.layout.pages[0].notch_bar = Some(bar(json!({"sections":[{"id":"player"},{"id":"weather"},{"id":"meeting"}]})));
        crate::layout::derive(&mut cfg);
        assert!(!cfg.drawer.sections.now_playing && !cfg.drawer.sections.weather && !cfg.drawer.sections.calendar, "bez rozložení notch se lišta nekreslí");
        cfg.appearance.layout = crate::appearance::look::Layout::Notch;
        crate::layout::derive(&mut cfg);
        assert!(cfg.drawer.sections.now_playing && cfg.drawer.sections.weather && cfg.drawer.sections.calendar);
        assert!(!cfg.drawer.sections.sound, "hlasitost na liště není");
        assert!(page_wants(&cfg, crate::layout::WORK, "calendar") && !page_wants(&cfg, crate::layout::PERSONAL, "calendar"));
    }

    #[test]
    fn co_lista_potrebuje() {
        let b = bar(json!({"sections":[{"id":"days","design":"dots"},{"id":"volume"}]}));
        assert!(wants(&b, "calendar") && wants(&b, "volume") && !wants(&b, "player") && !wants(&b, "weather"));
        assert!(!wants(&bar(json!({"sections":[{"id":"days"}]})), "calendar"), "pás bez teček kalendář nečte");
        assert!(wants(&bar(json!({"sections":[{"id":"meeting"}]})), "calendar"));
        assert!(wants(&bar(json!({"sections":[{"id":"days","design":"sheet"}]})), "calendar"), "list kalendáře ukazuje dnešní události");
    }
}
