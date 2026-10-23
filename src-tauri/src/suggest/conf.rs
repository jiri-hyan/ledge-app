//! Config `suggest`: které zdroje smí krmit pruh kontextových návrhů (a puls proužku u GitHubu).
//! Výchozí: schránka VYPNUTO (čte obsah schránky a potřebuje zapnutou historii schránky — soukromí),
//! ostatní zapnuto (dev server a připomínky už Ledge zná, GitHub se ukáže jen se zapnutou integrací).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SuggestConfig {
    /// Návrh „Uložit odkaz / Do poznámky“, když zkopíruješ adresu, kterou ještě nemáš mezi odkazy. Čte historii
    /// schránky (musí být zapnutá v `clipboard.enabled`), nic mimo ni. Výchozí vypnuto.
    pub clipboard: bool,
    /// Návrh „Otevřít localhost:PORT“ krátce po naběhnutí dev serveru spuštěného z Ledge. Výchozí zapnuto.
    pub dev_server: bool,
    /// Návrh „Otevřít PR #N“ u PR čekajícího na tvoji revizi nebo se spadlým CI. Jen se zapnutým GitHubem
    /// (`github.enabled`) a z dat, která už Ledge načetl. Výchozí zapnuto.
    pub github: bool,
    /// Návrh „Otevřít poznámku“ u připomínky po termínu. Výchozí zapnuto.
    pub reminders: bool,
    /// Krátký puls proužku, když se u GitHubu objeví nové PR k revizi nebo spadne CI na tvém PR (jako když
    /// Claude čeká; ve fullscreen hře nic, bez toastu a fokusu). Jen se zapnutým GitHubem. Výchozí zapnuto.
    pub github_pulse: bool,
}

impl Default for SuggestConfig {
    fn default() -> Self {
        Self { clipboard: false, dev_server: true, github: true, reminders: true, github_pulse: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_old_config() {
        let d = SuggestConfig::default();
        assert_eq!((d.clipboard, d.dev_server, d.github, d.reminders, d.github_pulse), (false, true, true, true, true));
        // starý config bez `suggest` = výchozí; částečná sekce doplní zbytek výchozími
        let c: crate::config::Config = serde_json::from_str("{}").unwrap();
        assert_eq!(c.suggest, d);
        let s: SuggestConfig = serde_json::from_str(r#"{"clipboard":true,"githubPulse":false}"#).unwrap();
        assert_eq!((s.clipboard, s.dev_server, s.github, s.reminders, s.github_pulse), (true, true, true, true, false));
        let back: SuggestConfig = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        assert!(crate::config::schema_json().contains("\"devServer\""));
    }
}
