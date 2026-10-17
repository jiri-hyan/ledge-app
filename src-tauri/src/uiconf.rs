//! Sekce `ui` v configu: jazyk rozhraní (`ui.lang`). Frontend ho čte při prvním otevření šuplíku
//! (`src/onboarding/state.ts` → `setLang`); nastavuje ho průvodce a Nastavení.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    /// čeština
    #[default]
    Cs,
    /// English
    En,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UiConfig {
    /// Jazyk rozhraní: "cs" (výchozí) nebo "en".
    pub lang: Lang,
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn lang_default_and_values() {
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.ui.lang, super::Lang::Cs);
        let cfg: Config = serde_json::from_str(r#"{"ui":{"lang":"en"}}"#).unwrap();
        assert_eq!(cfg.ui.lang, super::Lang::En);
        assert!(serde_json::from_str::<Config>(r#"{"ui":{"lang":"de"}}"#).is_err());
        assert!(crate::config::schema_json().contains("\"lang\""));
    }
}
