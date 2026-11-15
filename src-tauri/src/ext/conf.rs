//! Config `extensions`: která rozšíření (`ledge.exe --ext <id>`) smí Ledge spouštět.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Rozšíření povolená uživatelem (`wall`, `browser`, …). Výchozí `[]` = nic neběží. Neznámé id se ignoruje
/// (nové Ledge ho může znát, starší ne). Nastavení samotných rozšíření NEJSOU v configu, ale v
/// `%LOCALAPPDATA%\Ledge\ext\<id>.json` (jen tento PC).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, transparent)]
pub struct ExtConfig(pub Vec<String>);

impl ExtConfig {
    #[allow(dead_code)] // jen testy; háček pro Nastavení (E1)
    pub fn has(&self, id: &str) -> bool {
        self.0.iter().any(|x| x == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn old_config_without_key_is_empty() {
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.extensions, ExtConfig::default());
        assert!(cfg.extensions.0.is_empty());
        // starší config s jinými sekcemi
        let cfg: Config = serde_json::from_str(r#"{"autostart": false, "roots": []}"#).unwrap();
        assert!(cfg.extensions.0.is_empty());
    }

    #[test]
    fn list_roundtrips_and_unknown_ids_survive() {
        let cfg: Config = serde_json::from_str(r#"{"extensions": ["wall", "neznamy-z-budoucna"]}"#).unwrap();
        assert!(cfg.extensions.has("wall") && cfg.extensions.has("neznamy-z-budoucna") && !cfg.extensions.has("browser"));
        let v = serde_json::to_value(&cfg).unwrap();
        assert_eq!(v["extensions"], serde_json::json!(["wall", "neznamy-z-budoucna"]));
        let back: Config = serde_json::from_value(v).unwrap();
        assert_eq!(back.extensions, cfg.extensions);
    }

    #[test]
    fn default_serializes_as_empty_array() {
        let v = serde_json::to_value(Config::default()).unwrap();
        assert_eq!(v["extensions"], serde_json::json!([]));
    }
}
