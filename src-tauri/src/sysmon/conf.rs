//! Config `system` (karta Systém): které disky ukazovat a v jakých jednotkách síť. Vzorkování samo
//! volby nemá — běží jen s viditelnou kartou (2 s) nebo s položkou proužku `system` (10 s).

use super::calc::normalize_letters;
use crate::configerr::{FieldError, fe};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Nejvýš tolik disků v configu (A–Z).
pub const MAX_DISKS: usize = 26;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NetUnit {
    /// bajty za sekundu: KB/s, MB/s
    #[default]
    Bytes,
    /// bity za sekundu: kbit/s, Mbit/s (tak je uvádí poskytovatelé připojení)
    Bits,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SystemConfig {
    /// Písmena disků, jejichž volné místo karta ukazuje (`["C", "D"]`). Prázdné = všechny pevné disky.
    pub disks: Vec<String>,
    /// Jednotky rychlosti sítě: `bytes` (MB/s) nebo `bits` (Mbit/s).
    pub net_unit: NetUnit,
}

pub fn validate(c: &SystemConfig) -> Result<(), FieldError> {
    if c.disks.len() > MAX_DISKS {
        return Err(fe("system.disks", trf!("system.disks: nejvýš {MAX_DISKS} disků.", "system.disks: at most {MAX_DISKS} drives.")));
    }
    for (i, d) in c.disks.iter().enumerate() {
        if normalize_letters(std::slice::from_ref(d)).is_empty() {
            return Err(fe(format!("system.disks.{i}"), trf!("system.disks: „{d}“ není písmeno disku (třeba C).", "system.disks: “{d}” isn't a drive letter (e.g. C).")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_without_section_keeps_defaults() {
        let c: SystemConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(c, SystemConfig::default());
        assert!(c.disks.is_empty() && c.net_unit == NetUnit::Bytes);
        let c: SystemConfig = serde_json::from_str(r#"{"disks":["C","d"],"netUnit":"bits"}"#).unwrap();
        assert_eq!((c.disks.len(), c.net_unit), (2, NetUnit::Bits));
    }

    #[test]
    fn validate_table() {
        let ok = |v: &[&str]| validate(&SystemConfig { disks: v.iter().map(|s| s.to_string()).collect(), ..Default::default() }).is_ok();
        assert!(ok(&[]));
        assert!(ok(&["C", "d"]));
        assert!(ok(&["E:"]), "dvojtečka se toleruje");
        assert!(!ok(&["C", ""]));
        assert!(!ok(&["1"]));
        assert!(!ok(&["č"]));
        let many: Vec<&str> = vec!["C"; MAX_DISKS + 1];
        assert!(!ok(&many));
        let e = validate(&SystemConfig { disks: vec!["".into()], ..Default::default() }).unwrap_err();
        assert_eq!(e.field.as_deref(), Some("system.disks.0"));
    }
}
