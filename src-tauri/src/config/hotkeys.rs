//! Globální zkratky: tvar v configu, normalizace, kolize a plán přeregistrace.

use super::Config;
use crate::configerr::{FieldError, fe};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct HotkeyConfig {
    /// Globální zkratky vůbec registrovat. Výchozí `false`: šéf je nepoužívá, šuplík se otevírá myší
    /// a poznámka tlačítkem. Starý config se zkratkami vypsanými, ale bez `enabled`, tak dostane
    /// `false` záměrně. Zbylá pole platí jen při `enabled: true` (i validace kolizí).
    pub enabled: bool,
    /// Otevřít/zavřít šuplík (s fokusem).
    pub toggle: String,
    /// Povolit čekající žádost Claude (registruje se jen, když nějaká čeká).
    pub approve: String,
    /// Zamítnout čekající žádost Claude.
    pub deny: String,
    /// Rychlá poznámka: otevře šuplík (s fokusem) rovnou na zápisu poznámky.
    pub note: String,
    /// Herní režim zapnout / vypnout (`gamemode.rs`). Prázdné = bez zkratky. Funguje i v herním režimu.
    pub game_mode: String,
    /// Lístky na plochu (`pins/`): průchozí režim u všech lístků zapnout / vypnout. Výjimka z
    /// `enabled`: registruje se, JEN když je na ploše aspoň jeden lístek, i s vypnutými zkratkami —
    /// na průchozí lístek nejde kliknout, tohle je cesta ven. Prázdné = bez zkratky (zbývá Ctrl+K).
    pub note_passthrough: String,
    /// Rychlý nový lístek na plochu (nová poznámka rovnou v lístku, s fokusem). Výchozí prázdné = bez
    /// zkratky. Jako `notePassthrough` platí i s vypnutými zkratkami (`enabled`), jen se zapnutými
    /// poznámkami; v herním režimu / hře přes celou obrazovku nedělá nic.
    pub note_pin: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            toggle: "Ctrl+Alt+Space".into(),
            approve: "Ctrl+Alt+Y".into(),
            deny: "Ctrl+Alt+D".into(),
            note: "Ctrl+Alt+N".into(),
            game_mode: String::new(),
            note_passthrough: "Ctrl+Alt+P".into(),
            note_pin: String::new(),
        }
    }
}


/// Tvar zkratky pro porovnání: malá písmena, sjednocené aliasy modifikátorů, modifikátory seřazené
/// (`Alt+Ctrl+n` == `ctrl + alt + N`).
pub fn normalize_hotkey(s: &str) -> String {
    let mut mods = Vec::new();
    let mut key = String::new();
    for part in s.split('+').map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty()) {
        match part.as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" | "cmdorcontrol" | "commandorctrl" => mods.push("ctrl"),
            "alt" | "option" => mods.push("alt"),
            "shift" => mods.push("shift"),
            "super" | "win" | "meta" | "cmd" | "command" => mods.push("super"),
            _ => key = part,
        }
    }
    mods.sort_unstable();
    mods.dedup();
    mods.push(&key);
    mods.join("+")
}

/// Akce, které můžou mít zkratku zaregistrovanou: s vypnutými zkratkami žádné, poznámka jen se
/// zapnutými poznámkami.
fn hotkey_actions(cfg: &Config) -> Vec<(&'static str, &str)> {
    let h = &cfg.hotkeys;
    if !h.enabled {
        return Vec::new();
    }
    let mut v = vec![("toggle", h.toggle.as_str()), ("approve", h.approve.as_str()), ("deny", h.deny.as_str())];
    if cfg.notes.enabled {
        v.push(("note", h.note.as_str()));
    }
    v.push(("gameMode", h.game_mode.as_str()));
    if cfg.notes.enabled {
        v.push(("notePassthrough", h.note_passthrough.as_str()));
        v.push(("notePin", h.note_pin.as_str()));
    }
    v
}

/// Dvě akce na stejné zkratce → chyba (druhá by se tiše nezaregistrovala nebo spouštěla obě).
pub(super) fn hotkey_collision(cfg: &Config) -> Result<(), FieldError> {
    let acts = hotkey_actions(cfg);
    for (i, (a, ka)) in acts.iter().enumerate() {
        for (b, kb) in &acts[i + 1..] {
            if !ka.trim().is_empty() && normalize_hotkey(ka) == normalize_hotkey(kb) {
                return Err(fe(format!("hotkeys.{b}"), trf!("Zkratka „{ka}“ je u dvou akcí: hotkeys.{a} a hotkeys.{b}.", "Shortcut “{ka}” is used by two actions: hotkeys.{a} and hotkeys.{b}.")));
            }
        }
    }
    Ok(())
}

/// Zkratky, které mají být právě zaregistrované: s `hotkeys.enabled = false` žádné; jinak toggle
/// vždy, note se zapnutými poznámkami, approve/deny jen když čeká žádost Claude (`approve`).
/// Průchozí režim lístků sem nepatří — ten přidává `active_hotkeys_pins`, jen když jsou lístky na ploše.
pub fn active_hotkeys(cfg: &Config, approve: bool) -> Vec<(&'static str, String)> {
    hotkey_actions(cfg)
        .into_iter()
        .filter(|(a, _)| approve || !matches!(*a, "approve" | "deny"))
        .filter(|(a, _)| !matches!(*a, "notePassthrough" | "notePin"))
        .filter(|(_, k)| !k.trim().is_empty())
        .map(|(a, k)| (a, k.to_string()))
        .collect()
}

/// `active_hotkeys` + zkratka průchozího režimu lístků, když je na ploše aspoň jeden lístek
/// (`pins_open`) — i s `hotkeys.enabled = false` (jediná cesta z průchozího lístku myší nevede) — a
/// zkratka „Nový lístek“ (`notePin`), je-li vyplněná (výchozí prázdná), taky bez ohledu na `enabled`.
/// Stejnou zkratku jako jiná registrovaná akce nepřidá (kolizi hlásí validace jen se zapnutými zkratkami).
pub fn active_hotkeys_pins(cfg: &Config, approve: bool, pins_open: bool) -> Vec<(&'static str, String)> {
    let mut v = active_hotkeys(cfg, approve);
    let mut add = |action: &'static str, k: &str, on: bool| {
        let k = k.trim();
        if on && cfg.notes.enabled && !k.is_empty() && !v.iter().any(|(_, o)| normalize_hotkey(o) == normalize_hotkey(k)) {
            v.push((action, k.to_string()));
        }
    };
    add("notePassthrough", &cfg.hotkeys.note_passthrough, pins_open);
    add("notePin", &cfg.hotkeys.note_pin, true);
    v
}

/// Plán přeregistrace: beze změny nic; jinak odregistrovat VŠECHNY staré a pak zaregistrovat celou
/// novou sadu — prohození dvou zkratek (A↔B) tak nikdy neskončí odregistrovanou novou zkratkou.
pub fn rebind_plan(old: &[(&'static str, String)], new: &[(&'static str, String)]) -> (Vec<String>, Vec<String>) {
    if old == new {
        return (Vec::new(), Vec::new());
    }
    let uniq = |v: &[(&'static str, String)]| {
        let mut out: Vec<String> = Vec::new();
        for (_, k) in v {
            if !out.iter().any(|o| normalize_hotkey(o) == normalize_hotkey(k)) {
                out.push(k.clone());
            }
        }
        out
    };
    (uniq(old), uniq(new))
}

/// Které akce zůstaly bez zkratky, protože její registraci odmítl systém (`failed` = zkratky z configu, jejichž
/// `register` selhal): dvojice (akce, zkratka). Akce se stejnou zkratkou jako selhalá jsou taky obsazené.
pub fn taken_actions(active: &[(&'static str, String)], failed: &[String]) -> Vec<(&'static str, String)> {
    active
        .iter()
        .filter(|(_, k)| failed.iter().any(|f| normalize_hotkey(f) == normalize_hotkey(k)))
        .map(|(a, k)| (*a, k.clone()))
        .collect()
}

/// Návrh jiné zkratky místo obsazené `key`: stejná klávesa s dalším modifikátorem (Shift, pak Alt, pak Ctrl+Alt
/// …), první kombinace, která není v `avoid` (zkratky z configu a už obsazené). Bez klávesy nebo bez
/// modifikátoru žádný návrh (holou klávesu globálně nikdo nechce).
pub fn suggest_hotkey(key: &str, avoid: &[String]) -> Option<String> {
    let norm = normalize_hotkey(key);
    let mut parts: Vec<&str> = norm.split('+').collect();
    let base = parts.pop().filter(|b| !b.is_empty())?;
    if parts.is_empty() {
        return None;
    }
    let has = |m: &str| parts.contains(&m);
    let used: Vec<String> = avoid.iter().map(|a| normalize_hotkey(a)).collect();
    let name = |m: &str| match m {
        "ctrl" => "Ctrl",
        "alt" => "Alt",
        "shift" => "Shift",
        _ => "Super",
    };
    for extra in ["shift", "alt", "ctrl", "super"] {
        if has(extra) {
            continue;
        }
        let mods: Vec<&str> = ["ctrl", "alt", "shift", "super"].into_iter().filter(|m| *m == extra || has(m)).collect();
        let cand = format!("{}+{}", mods.iter().map(|m| name(m)).collect::<Vec<_>>().join("+"), base.to_uppercase());
        if !used.contains(&normalize_hotkey(&cand)) {
            return Some(cand);
        }
    }
    None
}

#[cfg(test)]
mod taken_tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_string()
    }

    #[test]
    fn taken_matches_failed_keys_in_any_spelling() {
        let active = vec![("toggle", s("Ctrl+Alt+Space")), ("note", s("Ctrl+Alt+N")), ("deny", s("Ctrl+Alt+D"))];
        for (failed, want) in [
            (vec![], vec![]),
            (vec![s("alt+ctrl+n")], vec!["note"]),
            (vec![s("Ctrl+Alt+Space"), s("Ctrl+Alt+D")], vec!["toggle", "deny"]),
            (vec![s("Ctrl+Alt+X")], vec![]),
        ] {
            let got: Vec<&str> = taken_actions(&active, &failed).iter().map(|(a, _)| *a).collect();
            assert_eq!(got, want, "{failed:?}");
        }
    }

    #[test]
    fn suggestion_adds_modifier_and_skips_used() {
        for (key, avoid, want) in [
            ("Ctrl+Alt+N", vec![], Some("Ctrl+Alt+Shift+N")),
            ("Ctrl+Alt+N", vec![s("shift+alt+ctrl+n")], Some("Ctrl+Alt+Super+N")),
            ("Ctrl+N", vec![], Some("Ctrl+Shift+N")),
            ("Ctrl+Shift+N", vec![], Some("Ctrl+Alt+Shift+N")),
            ("Ctrl+Alt+Shift+Super+N", vec![], None),
            ("N", vec![], None),
            ("", vec![], None),
            ("Ctrl+", vec![], None),
        ] {
            assert_eq!(suggest_hotkey(key, &avoid).as_deref(), want, "{key}");
        }
    }
}
