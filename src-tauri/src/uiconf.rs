//! Sekce `ui` v configu: jazyk rozhraní (`ui.lang`). Frontend ho čte při prvním otevření šuplíku
//! (`src/onboarding/state.ts` → `setLang`); nastavuje ho průvodce a Nastavení. `ui.tips` = prohlídky
//! a tipy, které uživatel už viděl (`src/layout/tips.ts`, `Tour.tsx`; každá se ukáže jen jednou).

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
    /// Prohlídky a tipy, které už proběhly nebo je uživatel přeskočil (např. `tour1` = 3 bubliny při
    /// prvním otevření šuplíku). Chybí / prázdné = ukázat jednou.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tips: Vec<String>,
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

    #[test]
    fn tips_default_and_roundtrip() {
        let cfg: Config = serde_json::from_str(r#"{"ui":{"lang":"en"}}"#).unwrap();
        assert!(cfg.ui.tips.is_empty(), "starý config = prohlídka se ukáže jednou");
        assert!(serde_json::to_value(&cfg.ui).unwrap().get("tips").is_none(), "prázdné se nepíše");
        let cfg: Config = serde_json::from_str(r#"{"ui":{"tips":["tour1"]}}"#).unwrap();
        assert_eq!(cfg.ui.tips, ["tour1"]);
        assert_eq!(serde_json::to_value(&cfg.ui).unwrap()["tips"], serde_json::json!(["tour1"]));
        assert!(crate::config::schema_json().contains("\"tips\""));
    }
}
