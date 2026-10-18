//! Složky design stylu (vlna 6), které nejsou barvy: písmo, materiál, rozložení a předvolba šířky.
//! Samotná tabulka stylů (co který styl nastaví) je JEN ve frontendu (`src/theme/styles.ts`); Rust
//! z toho potřebuje šířku (`geom::drawer_width_for`) — ostatní jen ukládá a validuje tvarem.
//! Výchozí hodnoty = vzhled před vlnou 6 (starý config se načte beze změny).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Jméno výchozího stylu (= vzhled před zavedením stylů).
pub const CLASSIC: &str = "classic";

/// Jméno stylu: 1–24 malých písmen ASCII (nový styl = záznam ve frontendu, Rust ho nezná jménem).
pub fn style_id_ok(s: &str) -> bool {
    (1..=24).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase())
}

/// Písmo — jen systémová písma Windows (žádné stahování).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FontLook {
    /// Segoe UI Variable (dnešní)
    #[default]
    System,
    /// Cascadia Mono / Consolas
    Mono,
    /// patkové nadpisy (Sitka, Georgia), text Segoe UI
    Serif,
    /// Bahnschrift (úzké bezpatkové)
    Grotesk,
}

/// Materiál ploch šuplíku a karet.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Material {
    /// plné plochy s jemnou září akcentu (dnešní)
    #[default]
    Solid,
    /// průsvitné vrstvy se světlou hranou
    Glass,
    /// papír: bez záře, měkké stíny
    Paper,
    /// záře akcentu na okrajích (statická)
    Glow,
    /// tlusté okraje a tvrdý posunutý stín
    Hard,
    /// měkké vystouplé plochy (neumorfismus)
    Soft,
    /// čárkované rámečky bez výplně (terminál)
    Frame,
}

/// Rozložení hlavní stránky (sloupce, mezery, okraje).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    /// dnešní mřížka
    #[default]
    Classic,
    /// plovoucí panel pod proužkem (oblý ze všech stran, s odstupem)
    Float,
    /// široký hustý pruh (5 sloupců)
    Band,
    /// užší „list“ se 2 sloupci a velkými okraji
    Sheet,
    /// panoráma se 4 sloupci
    Panorama,
    /// mřížka s většími mezerami (místo pro posunuté stíny)
    Mosaic,
    /// velké měkké dlaždice ve 3 sloupcích
    Tiles,
}

/// Předvolba šířky šuplíku.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WidthPreset {
    /// `drawer.width` v px (dnešní chování)
    #[default]
    Custom,
    Narrow,
    Medium,
    Wide,
    /// celá šířka monitoru
    Full,
}

impl WidthPreset {
    /// Šířka předvolby v logických px; `None` = vlastní (`drawer.width`) nebo celá šířka monitoru.
    pub fn px(self) -> Option<u32> {
        match self {
            WidthPreset::Narrow => Some(820),
            WidthPreset::Medium => Some(1080),
            WidthPreset::Wide => Some(1440),
            WidthPreset::Custom | WidthPreset::Full => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_id_table() {
        for (s, want) in [("classic", true), ("neon", true), ("", false), ("Neon", false), ("můj", false), ("a-b", false), ("abcdefghijklmnopqrstuvwxy", false)] {
            assert_eq!(style_id_ok(s), want, "{s:?}");
        }
    }

    #[test]
    fn presets_and_serde() {
        assert_eq!(WidthPreset::Medium.px(), Some(1080), "střední = dnešní výchozí drawer.width");
        assert_eq!(WidthPreset::Custom.px(), None);
        let (lo, hi) = super::super::DRAWER_WIDTH;
        for w in [WidthPreset::Narrow, WidthPreset::Medium, WidthPreset::Wide] {
            assert!((lo..=hi).contains(&w.px().unwrap()), "{w:?} v rozsahu šířky");
        }
        assert_eq!(serde_json::to_string(&Material::Frame).unwrap(), "\"frame\"");
        assert_eq!(serde_json::to_string(&FontLook::Grotesk).unwrap(), "\"grotesk\"");
        assert_eq!(serde_json::from_str::<Layout>("\"panorama\"").unwrap(), Layout::Panorama);
    }
}
