//! Sekce `palette` v configu: chování palety příkazů (Ctrl+K / 🔍). Frontend ji čte ze snapshotu
//! (`Snapshot.palette_compact` → `src/palette/compact.ts`); přepínač je v Nastavení → Šuplík a proužek.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PaletteConfig {
    /// Kompaktní paleta: po otevření jen vyhledávací pole, výsledky až při psaní (nedávné šipkou
    /// dolů). Výchozí false = hned celý seznam.
    pub compact: bool,
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
        assert!(back.contains(r#""palette":{"compact":true}"#), "{back}");
        assert!(crate::config::schema_json().contains("\"compact\""));
    }
}
