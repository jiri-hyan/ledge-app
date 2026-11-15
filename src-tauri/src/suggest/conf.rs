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
    /// Návrhy „přehledu“: zadání čeká na mě, ranní shrnutí, „Dnes“ (úkoly) a „Claude od posledně“ (dřív je
    /// vypínala `reminders`). Chybí-li klíč (starý config), platí hodnota `reminders` (`digest_on`); výchozí zapnuto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<bool>,
    /// Krátký puls proužku, když se u GitHubu objeví nové PR k revizi nebo spadne CI na tvém PR (jako když
    /// Claude čeká; ve fullscreen hře nic, bez toastu a fokusu). Jen se zapnutým GitHubem. Výchozí zapnuto.
    pub github_pulse: bool,
}

impl Default for SuggestConfig {
    fn default() -> Self {
        Self { clipboard: false, dev_server: true, github: true, reminders: true, digest: None, github_pulse: true }
    }
}

impl SuggestConfig {
    /// Efektivní „Přehled“: explicitní `digest`, jinak `reminders` (migrace: kdo měl připomínky vypnuté, nedostane chipy zpět).
    pub fn digest_on(&self) -> bool {
        self.digest.unwrap_or(self.reminders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_old_config() {
        let d = SuggestConfig::default();
        assert_eq!((d.clipboard, d.dev_server, d.github, d.reminders, d.digest, d.github_pulse), (false, true, true, true, None, true));
        // starý config bez `suggest` = výchozí; částečná sekce doplní zbytek výchozími
        let c: crate::config::Config = serde_json::from_str("{}").unwrap();
        assert_eq!(c.suggest, d);
        let s: SuggestConfig = serde_json::from_str(r#"{"clipboard":true,"githubPulse":false}"#).unwrap();
        assert_eq!((s.clipboard, s.dev_server, s.github, s.reminders, s.digest, s.github_pulse), (true, true, true, true, None, false));
        // L193: efektivní přehled = explicitní digest, jinak reminders (klíč chybí / true / false)
        let cases = [(None, true, true), (None, false, false), (Some(true), false, true), (Some(false), true, false), (Some(true), true, true), (Some(false), false, false)];
        for (dg, rem, want) in cases {
            let c = SuggestConfig { digest: dg, reminders: rem, ..SuggestConfig::default() };
            assert_eq!(c.digest_on(), want, "digest {dg:?}, reminders {rem}");
        }
        let old: SuggestConfig = serde_json::from_str(r#"{"reminders":false}"#).unwrap();
        assert!(!old.digest_on(), "starý config s vypnutými připomínkami zůstane bez přehledu");
        assert!(!serde_json::to_string(&old).unwrap().contains("digest"), "nevyplněný klíč se nezapisuje");
        let back: SuggestConfig = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        assert!(crate::config::schema_json().contains("\"devServer\""));
    }
}
