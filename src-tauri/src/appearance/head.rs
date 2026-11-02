//! Hlavička šuplíku (kolo 27): které díly se ukazují a v jakém pořadí, styl hodin, štítků (chipů)
//! a přepínače záložek. Vykresluje to jen frontend (`src/layout/headModel.ts`); Rust ukládá a hlídá tvar.
//! Výchozí hodnoty = dnešní hlavička, jen nulové štítky (`showZero: false`) se nově schovávají.

use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Díl hlavičky. `Settings` a `Close` nejdou vypnout, `Close` je vždy úplně vpravo. Dnešní pořadí = pořadí
/// variant (kopie ve frontendu: `layout/headModel.ts` `DEFAULT_ORDER`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HeadPart {
    Clock,
    Chips,
    Tabs,
    Edit,
    Note,
    Search,
    Settings,
    Close,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClockStyle {
    /// velký čas a datum pod ním (dnešní)
    #[default]
    Big,
    /// jednořádkové „20:02 · po 2. 11.“
    Line,
    /// čas s vteřinami (vteřiny běží jen s otevřeným šuplíkem)
    Seconds,
    /// malé analogové hodiny (SVG)
    Analog,
    /// jen čas
    Time,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChipStyle {
    /// automaticky: pilulky → sloučená pilulka → tečky → ikonky, podle toho, co se vejde (výchozí od kola 27)
    #[default]
    Auto,
    /// oddělené pilulky
    Pills,
    /// jedna sloučená pilulka
    Merged,
    /// tečky s čísly
    Dots,
    /// ikonky s číslem
    Icons,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TabsStyle {
    /// segment (dnešní)
    #[default]
    Segment,
    /// podtržení
    Underline,
    /// oddělené pilulky
    Pills,
    /// jen ikony
    Icons,
    /// tečky
    Dots,
}

fn yes() -> bool {
    true
}
fn is_true(b: &bool) -> bool {
    *b
}

/// `appearance.head`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct HeadLook {
    /// Pořadí dílů zleva; chybějící díly se přidají na konec v dnešním pořadí. Prázdné = dnešní pořadí.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<HeadPart>,
    /// Vypnuté díly (`settings` a `close` vypnout nejdou).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hidden: Vec<HeadPart>,
    /// Styl hodin: `big` (výchozí), `line`, `seconds`, `analog`, `time`.
    pub clock: ClockStyle,
    /// Datum u hodin (výchozí zapnuto).
    #[serde(skip_serializing_if = "is_true")]
    pub show_date: bool,
    /// Styl štítků: `auto` (výchozí), `pills`, `merged`, `dots`, `icons`. Vždy jedna řada, co se nevejde, je pod „+N“.
    pub chips: ChipStyle,
    /// Ukazovat i štítky s nulou (třeba „0 běží“); výchozí vypnuto.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub show_zero: bool,
}

impl Default for HeadLook {
    fn default() -> Self {
        Self { order: Vec::new(), hidden: Vec::new(), clock: ClockStyle::default(), show_date: yes(), chips: ChipStyle::default(), show_zero: false }
    }
}

impl HeadLook {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// `appearance.tabs`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct TabsLook {
    /// Vzhled přepínače záložek: `segment` (výchozí), `underline`, `pills`, `icons`, `dots`.
    pub style: TabsStyle,
}

impl TabsLook {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Díly bez duplicit, `settings`/`close` nesmí být vypnuté.
pub fn validate(h: &HeadLook) -> Result<(), String> {
    for (i, p) in h.order.iter().enumerate() {
        if h.order[..i].contains(p) {
            return Err(tr("appearance.head.order: každý díl smí být jen jednou.", "appearance.head.order: each part may appear only once.").into());
        }
    }
    if h.hidden.iter().any(|p| matches!(p, HeadPart::Settings | HeadPart::Close)) {
        return Err(tr("appearance.head.hidden: nastavení a zavřít nejdou vypnout.", "appearance.head.hidden: settings and close cannot be hidden.").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_is_todays_head() {
        let h: HeadLook = serde_json::from_str("{}").unwrap();
        assert!(h.is_default());
        assert!(h.show_date && !h.show_zero);
        assert_eq!(serde_json::to_string(&h).unwrap(), r#"{"clock":"big","chips":"auto"}"#);
        assert!(TabsLook::default().is_default());
    }

    #[test]
    fn roundtrip_and_validate() {
        let h: HeadLook = serde_json::from_str(r#"{"order":["tabs","clock"],"hidden":["note"],"clock":"analog","showDate":false,"chips":"merged","showZero":true}"#).unwrap();
        assert!(validate(&h).is_ok());
        assert_eq!(serde_json::from_str::<HeadLook>(&serde_json::to_string(&h).unwrap()).unwrap(), h);
        let dup = HeadLook { order: vec![HeadPart::Tabs, HeadPart::Tabs], ..Default::default() };
        assert!(validate(&dup).is_err());
        let off = HeadLook { hidden: vec![HeadPart::Close], ..Default::default() };
        assert!(validate(&off).is_err());
    }
}
