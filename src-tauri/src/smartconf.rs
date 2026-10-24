//! Sekce `smart` v configu: „chytřejší šuplík“ (kolo 20) — klávesnice v šuplíku, přetažení na šuplík,
//! kolečko nad záložkami, zvuk u schvalování a stav incidentů Anthropicu. Všechno jsou volby; staré configy
//! bez sekce se chovají jako dřív (klávesy, přetažení, kolečko, zvuk i síť jsou VYPNUTÉ; zapíná je člověk v Nastavení). Frontend čte tři přepínače UI ze snapshotu (`Snapshot.smart`), zvuk a incidenty
//! řídí jen Rust (`approvalsound.rs`, `incidents.rs`).

use crate::configerr::{FieldError, fe};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Meze minimálního odstupu zvuků (s) a intervalu dotazu na incidenty (min).
pub const GAP_MIN_S: u32 = 5;
pub const GAP_MAX_S: u32 = 600;
pub const EVERY_MIN: u32 = 5;
pub const EVERY_MAX: u32 = 120;

fn gap_default() -> u32 {
    30
}
fn every_default() -> u32 {
    10
}

/// Tichý systémový zvuk, když Claude čeká na schválení nebo se ptá.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SoundConfig {
    /// Zapnuto? Výchozí false (Ledge je tichý).
    pub enabled: bool,
    /// Nejmenší odstup dvou zvuků v sekundách (5–600), výchozí 30.
    #[serde(default = "gap_default")]
    pub min_gap_s: u32,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self { enabled: false, min_gap_s: gap_default() }
    }
}

/// Stav incidentů Anthropicu v kartě Claude (jediné volání sítě této sekce).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct IncidentsConfig {
    /// Ptát se status.anthropic.com? Výchozí false (žádná síť bez zapnutí).
    pub enabled: bool,
    /// Jak často se ptát v minutách (5–120), jen když je limit karta vidět. Výchozí 10.
    #[serde(default = "every_default")]
    pub every_min: u32,
}

impl Default for IncidentsConfig {
    fn default() -> Self {
        Self { enabled: false, every_min: every_default() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SmartConfig {
    /// Alt+1…9 spustí N-tého zástupce a šipky procházejí zástupce, když má otevřený šuplík fokus. Výchozí false (zapíná člověk).
    pub quick_keys: bool,
    /// Odkaz, soubor, složka nebo .exe přetažené na otevřený šuplík nabídnou nového zástupce. Výchozí false (zapíná člověk).
    pub drop_shortcut: bool,
    /// Kolečko myši nad záložkami šuplíku přepíná záložky. Výchozí false (zapíná člověk).
    pub tab_wheel: bool,
    /// Zvuk u schvalování a otázky Clauda.
    pub approval_sound: SoundConfig,
    /// Stav incidentů Anthropicu.
    pub incidents: IncidentsConfig,
}

/// Přepínače pro UI (`Snapshot.smart`).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SmartView {
    pub quick_keys: bool,
    pub drop_shortcut: bool,
    pub tab_wheel: bool,
}

pub fn view(c: &SmartConfig) -> SmartView {
    SmartView { quick_keys: c.quick_keys, drop_shortcut: c.drop_shortcut, tab_wheel: c.tab_wheel }
}

pub fn validate(c: &SmartConfig) -> Result<(), FieldError> {
    if !(GAP_MIN_S..=GAP_MAX_S).contains(&c.approval_sound.min_gap_s) {
        return Err(fe("smart.approvalSound.minGapS", trf!("smart.approvalSound.minGapS musí být {GAP_MIN_S}–{GAP_MAX_S} sekund.", "smart.approvalSound.minGapS must be {GAP_MIN_S}–{GAP_MAX_S} seconds.")));
    }
    if !(EVERY_MIN..=EVERY_MAX).contains(&c.incidents.every_min) {
        return Err(fe("smart.incidents.everyMin", trf!("smart.incidents.everyMin musí být {EVERY_MIN}–{EVERY_MAX} minut.", "smart.incidents.everyMin must be {EVERY_MIN}–{EVERY_MAX} minutes.")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn old_config_keeps_behaviour() {
        // starý config bez sekce: žádná nová volba se nezapne sama (klávesy, přetažení, kolečko, zvuk, síť = vypnuto)
        for src in ["{}", r#"{"smart":{}}"#, r#"{"smart":{"approvalSound":{},"incidents":{}}}"#] {
            let cfg: Config = serde_json::from_str(src).unwrap();
            let s = &cfg.smart;
            assert!(!s.quick_keys && !s.drop_shortcut && !s.tab_wheel, "{src}: klávesy, přetažení a kolečko musí být vypnuté");
            assert!(!s.approval_sound.enabled && !s.incidents.enabled, "{src}: zvuk a síť musí být vypnuté");
            assert_eq!((s.approval_sound.min_gap_s, s.incidents.every_min), (30, 10), "{src}");
        }
    }

    #[test]
    fn roundtrip_and_schema() {
        let cfg: Config = serde_json::from_str(r#"{"smart":{"tabWheel":true,"approvalSound":{"enabled":true,"minGapS":60},"incidents":{"enabled":true,"everyMin":15}}}"#).unwrap();
        assert!(cfg.smart.tab_wheel && !cfg.smart.quick_keys);
        let back = serde_json::to_string(&cfg.smart).unwrap();
        assert!(back.contains(r#""approvalSound":{"enabled":true,"minGapS":60}"#), "{back}");
        let schema = crate::config::schema_json();
        for key in ["quickKeys", "dropShortcut", "tabWheel", "approvalSound", "minGapS", "incidents", "everyMin"] {
            assert!(schema.contains(key), "{key}");
        }
    }

    #[test]
    fn validate_ranges() {
        let mut c = SmartConfig::default();
        assert!(validate(&c).is_ok());
        type Case<'a> = (&'a str, fn(&mut SmartConfig, u32), u32, bool);
        let cases: [Case; 8] = [
            ("gap 4", |c, v| c.approval_sound.min_gap_s = v, 4, false),
            ("gap 5", |c, v| c.approval_sound.min_gap_s = v, 5, true),
            ("gap 600", |c, v| c.approval_sound.min_gap_s = v, 600, true),
            ("gap 601", |c, v| c.approval_sound.min_gap_s = v, 601, false),
            ("every 4", |c, v| c.incidents.every_min = v, 4, false),
            ("every 5", |c, v| c.incidents.every_min = v, 5, true),
            ("every 120", |c, v| c.incidents.every_min = v, 120, true),
            ("every 121", |c, v| c.incidents.every_min = v, 121, false),
        ];
        for (name, set, v, ok) in cases {
            c = SmartConfig::default();
            set(&mut c, v);
            assert_eq!(validate(&c).is_ok(), ok, "{name}");
        }
    }
}
