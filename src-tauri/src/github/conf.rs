//! Config `github` (karta GitHub a položka proužku `github`). Výchozí VYPNUTO: Ledge sáhne na síť
//! (přes přihlášené `gh`) teprve po zapnutí. Co se čte: tvoje PR, žádosti o revizi a notifikace.

use crate::configerr::{FieldError, fe};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Nejkratší a nejdelší odstup automatické obnovy (minuty). Pod 2 minuty nejde (limity GitHubu).
pub const MIN_REFRESH_MIN: u32 = 2;
pub const MAX_REFRESH_MIN: u32 = 60;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GithubConfig {
    /// Zapnout GitHub: Ledge přes přihlášené `gh` (GitHub CLI) čte tvoje pull requesty a notifikace
    /// a zobrazí je v kartě GitHub / v proužku. Nic neodesílá. Vypnuto = žádný dotaz na GitHub.
    pub enabled: bool,
    /// Ukázat PR, které čekají na tvoji revizi.
    pub review: bool,
    /// Ukázat tvoje otevřené PR se stavem kontrol (CI).
    pub mine: bool,
    /// Ukázat nepřečtené notifikace.
    pub notifications: bool,
    /// Jak často (v minutách) se data obnovují, dokud je karta vidět nebo je zapnutá položka proužku `github`.
    /// 2–60. Zavřený šuplík bez položky proužku neobnovuje nic.
    pub refresh_min: u32,
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self { enabled: false, review: true, mine: true, notifications: true, refresh_min: 5 }
    }
}

impl GithubConfig {
    /// Odstup automatické obnovy v ms.
    pub fn interval_ms(&self) -> i64 {
        i64::from(self.refresh_min.clamp(MIN_REFRESH_MIN, MAX_REFRESH_MIN)) * 60_000
    }
    /// Je zapnuté aspoň něco, co se dá číst?
    pub fn any(&self) -> bool {
        self.enabled && (self.review || self.mine || self.notifications)
    }
}

pub fn validate(c: &GithubConfig) -> Result<(), FieldError> {
    if !(MIN_REFRESH_MIN..=MAX_REFRESH_MIN).contains(&c.refresh_min) {
        return Err(fe("github.refreshMin", trf!("github.refreshMin musí být {MIN_REFRESH_MIN}–{MAX_REFRESH_MIN} minut.", "github.refreshMin must be {MIN_REFRESH_MIN}–{MAX_REFRESH_MIN} minutes.")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off_and_old_config_keeps_it() {
        let d = GithubConfig::default();
        assert!(!d.enabled && !d.any(), "výchozí vypnuto");
        let c: GithubConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(c, d);
        let c: GithubConfig = serde_json::from_str(r#"{"enabled":true,"mine":false,"refreshMin":10}"#).unwrap();
        assert_eq!((c.enabled, c.review, c.mine, c.notifications, c.refresh_min), (true, true, false, true, 10));
        assert_eq!(c.interval_ms(), 600_000);
    }

    #[test]
    fn interval_and_validation() {
        let with = |m: u32| GithubConfig { refresh_min: m, ..Default::default() };
        assert!(validate(&with(2)).is_ok() && validate(&with(60)).is_ok());
        assert!(validate(&with(1)).is_err() && validate(&with(0)).is_err() && validate(&with(61)).is_err());
        assert_eq!(validate(&with(0)).unwrap_err().field.as_deref(), Some("github.refreshMin"));
        assert_eq!(with(0).interval_ms(), 120_000, "i špatná hodnota nikdy nejde pod 2 minuty");
        assert_eq!(with(999).interval_ms(), 3_600_000);
        let all_off = GithubConfig { enabled: true, review: false, mine: false, notifications: false, refresh_min: 5 };
        assert!(!all_off.any());
        assert!(GithubConfig { enabled: true, ..Default::default() }.any());
    }
}
