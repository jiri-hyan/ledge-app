//! Sekce `palette` v configu: chování palety příkazů (Ctrl+K / 🔍). Frontend ji čte ze snapshotu
//! (`Snapshot.palette_compact` → `src/palette/compact.ts`, `palette_win_settings` →
//! `src/palette/winsettings.ts`); přepínače jsou v Nastavení → Šuplík a proužek.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PaletteConfig {
    /// Kompaktní paleta: po otevření jen vyhledávací pole, výsledky až při psaní (nedávné šipkou
    /// dolů). Výchozí false = hned celý seznam.
    pub compact: bool,
    /// Stránky Nastavení Windows (`ms-settings:`) a klasické ovládací panely v paletě. Výchozí true.
    #[serde(default = "yes")]
    pub win_settings: bool,
}

impl Default for PaletteConfig {
    fn default() -> Self {
        Self { compact: false, win_settings: true }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn compact_default_off_and_roundtrip() {
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert!(!cfg.palette.compact, "chybějící sekce = dnešní chování");
        let cfg: Config = serde_json::from_str(r#"{"palette":{}}"#).unwrap();
        assert!(!cfg.palette.compact);
        let cfg: Config = serde_json::from_str(r#"{"palette":{"compact":true}}"#).unwrap();
        assert!(cfg.palette.compact);
        let back = serde_json::to_string(&cfg).unwrap();
        assert!(back.contains(r#""palette":{"compact":true,"winSettings":true}"#), "{back}");
        assert!(crate::config::schema_json().contains("\"compact\""));
    }

    #[test]
    fn win_settings_default_on() {
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert!(cfg.palette.win_settings, "chybějící sekce = zapnuto");
        let cfg: Config = serde_json::from_str(r#"{"palette":{"compact":true}}"#).unwrap();
        assert!(cfg.palette.win_settings, "starý config bez pole = zapnuto");
        let cfg: Config = serde_json::from_str(r#"{"palette":{"winSettings":false}}"#).unwrap();
        assert!(!cfg.palette.win_settings);
        assert!(crate::config::schema_json().contains("winSettings"));
    }
}
