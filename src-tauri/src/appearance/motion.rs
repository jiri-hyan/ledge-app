//! Pohyb šuplíku z JEDNOHO místa: `appearance.motion` (styl + rychlost). Dřív existoval ještě
//! `drawer.animationMs` (vypínač „Animace vysunutí“ v panelu Šuplík) se stejným významem jako styl
//! `none` — sjednoceno; starý klíč se už nezapisuje, jen se při načtení převede ([`upgrade_loaded`]).

use super::{MotionLook, MotionStyle};
use crate::config::Config;
use serde_json::Value;

/// Základní délka vysunutí (ms) při rychlosti 1 — dřívější výchozí `drawer.animationMs`.
pub const BASE_SLIDE_MS: u64 = 170;

/// Délka animace okna šuplíku (ms) pro Rust: `0` = bez morfu i bez vysunutí (okamžitě). Morf (pozadí
/// „solid“) časuje frontend (pružiny, odskok, rozplynutí…); tohle číslo rozhoduje, jestli morf vůbec
/// běží, a je délkou vysunutí okna v akrylovém režimu. `os_reduced` = systémové „omezit pohyb“
/// (Windows → Usnadnění → Efekty animace vypnuté) → vždy 0, jako styl `none`. Čistá funkce.
pub fn drawer_anim_ms(m: &MotionLook, os_reduced: bool) -> u64 {
    if m.style == MotionStyle::None || os_reduced {
        return 0;
    }
    let speed = if m.speed.is_finite() && m.speed > 0.0 { m.speed as f64 } else { 1.0 };
    (BASE_SLIDE_MS as f64 / speed).round() as u64
}

/// [`drawer_anim_ms`] se skutečným systémovým nastavením (jedno `SystemParametersInfoW`, levné) —
/// volá vlákno `ui` před každým příkazem šuplíku, takže změna v nastavení Windows platí hned.
pub fn anim_ms_now(m: &MotionLook) -> u64 {
    if m.style == MotionStyle::None {
        return 0;
    }
    drawer_anim_ms(m, crate::attention::os_animations_off())
}

/// Převod starého `drawer.animationMs` na `appearance.motion` (čistá funkce nad syrovým JSON configu):
/// - chybí / není číslo → beze změny;
/// - `0` (vypnutá „Animace vysunutí“) → styl `none` — i když `motion.style` v souboru je: s nulou se
///   dřív nehýbalo nic, ať byl styl jakýkoli, takže chování zůstane stejné;
/// - jiná než výchozí délka a `motion.speed` v souboru NENÍ → rychlost `170 / ms` (oříznutá na 0.5–2,
///   zaokrouhlená na desetiny); výslovně uložená rychlost má přednost.
pub fn legacy_animation_ms(raw: &Value, out: &mut MotionLook) {
    let Some(ms) = raw.pointer("/drawer/animationMs").and_then(Value::as_f64) else { return };
    if ms <= 0.0 {
        out.style = MotionStyle::None;
        return;
    }
    if raw.pointer("/appearance/motion/speed").is_some() || (ms - BASE_SLIDE_MS as f64).abs() < 0.5 {
        return;
    }
    let speed = (BASE_SLIDE_MS as f64 / ms).clamp(0.5, 2.0);
    out.speed = ((speed * 10.0).round() / 10.0) as f32;
}

/// Úpravy vzhledu po načtení souboru (`config::load`). Nic se nezapisuje — starý klíč zmizí z
/// `config.json` při nejbližším uložení (struktura ho už nemá).
pub fn upgrade_loaded(raw: &Value, cfg: &mut Config) {
    legacy_animation_ms(raw, &mut cfg.appearance.motion);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn look(style: MotionStyle, speed: f32) -> MotionLook {
        MotionLook { style, speed }
    }

    #[test]
    fn anim_ms_table() {
        // (pohyb, systémové „omezit pohyb“, délka vysunutí)
        let cases = [
            (look(MotionStyle::Spring, 1.0), false, 170),
            (look(MotionStyle::Smooth, 1.0), false, 170),
            (look(MotionStyle::Snappy, 2.0), false, 85),
            (look(MotionStyle::Spring, 0.5), false, 340),
            (look(MotionStyle::Bounce, 1.0), false, 170),
            (look(MotionStyle::Fade, 2.0), false, 85),
            (look(MotionStyle::Notch, 1.0), false, 170),
            (look(MotionStyle::Drop, 1.0), false, 170),
            (look(MotionStyle::Unfold, 2.0), false, 85),
            (look(MotionStyle::None, 1.0), false, 0),
            (look(MotionStyle::None, 2.0), false, 0),
            (look(MotionStyle::Spring, 0.0), false, 170), // nesmysl z ruky → jako rychlost 1, ne dělení nulou
            // systémové „omezit pohyb“ = bez morfu i bez vysunutí, ať je styl jakýkoli
            (look(MotionStyle::Spring, 1.0), true, 0),
            (look(MotionStyle::Bounce, 0.5), true, 0),
            (look(MotionStyle::None, 1.0), true, 0),
        ];
        for (m, reduced, want) in cases {
            assert_eq!(drawer_anim_ms(&m, reduced), want, "{m:?} reduced={reduced}");
        }
    }

    #[test]
    fn legacy_animation_ms_table() {
        let def = MotionLook::default();
        // (popis, syrový JSON, výsledek po převodu)
        let cases: Vec<(&str, &str, MotionLook)> = vec![
            ("bez klíče", r#"{}"#, def.clone()),
            ("výchozí 170", r#"{"drawer":{"animationMs":170}}"#, def.clone()),
            ("vypnuto", r#"{"drawer":{"animationMs":0}}"#, look(MotionStyle::None, 1.0)),
            (
                "vypnuto přebije uložený styl (dřív se stejně nehýbalo nic)",
                r#"{"drawer":{"animationMs":0},"appearance":{"motion":{"style":"smooth","speed":1.5}}}"#,
                look(MotionStyle::None, 1.5),
            ),
            ("pomalejší 340 → 0.5", r#"{"drawer":{"animationMs":340}}"#, look(MotionStyle::Spring, 0.5)),
            ("rychlejší 85 → 2", r#"{"drawer":{"animationMs":85}}"#, look(MotionStyle::Spring, 2.0)),
            ("250 → 0.7 (desetiny)", r#"{"drawer":{"animationMs":250}}"#, look(MotionStyle::Spring, 0.7)),
            ("extrém 2000 → 0.5", r#"{"drawer":{"animationMs":2000}}"#, look(MotionStyle::Spring, 0.5)),
            ("extrém 10 → 2", r#"{"drawer":{"animationMs":10}}"#, look(MotionStyle::Spring, 2.0)),
            (
                "uložená rychlost má přednost",
                r#"{"drawer":{"animationMs":340},"appearance":{"motion":{"speed":1.2}}}"#,
                look(MotionStyle::Spring, 1.2),
            ),
            ("nečíselná hodnota", r#"{"drawer":{"animationMs":"rychle"}}"#, def.clone()),
        ];
        for (name, json, want) in cases {
            let raw: Value = serde_json::from_str(json).unwrap();
            let mut cfg = Config::deserialize(&raw).unwrap();
            upgrade_loaded(&raw, &mut cfg);
            assert_eq!(cfg.appearance.motion, want, "{name}");
            assert!(crate::config::validate(&cfg).is_ok(), "{name}: po převodu validní");
        }
    }

    #[test]
    fn old_key_is_read_through_load_and_never_written() {
        let dir = std::env::temp_dir().join(format!("ledge-motion-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"drawer":{"animationMs":0,"width":1200}}"#).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(cfg.appearance.motion.style, MotionStyle::None);
        assert_eq!(cfg.drawer.width, 1200);
        crate::config::save(&path, &cfg).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("animationMs"), "starý klíč se nezapisuje: {text}");
        // po uložení drží nový klíč totéž
        let again = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(again.appearance.motion.style, MotionStyle::None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
