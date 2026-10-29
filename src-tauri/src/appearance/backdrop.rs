//! Pozadí otevřeného šuplíku (kolo 25, `appearance.backdrop`): hotový nebo vlastní gradient (`id`),
//! volitelně živé pozadí nad ním (`live`: oheň, polární záře, láva, hvězdy, déšť, zrno). Nic nenastaveno
//! (= starý config, `is_default`) → pozadí jako dosud a pole se ani nezapisuje. Tabulka hotových gradientů
//! a kreslení živých pozadí je ve frontendu (`src/theme/backdrop.ts`, `backdropLive.ts`); Rust drží jen
//! tvar a ověřuje vlastní gradienty (2–5 zastávek `#rrggbb`, nejvýš 20 vlastních).

use super::is_hex;
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_CUSTOM: usize = 20;
pub const STOPS: (usize, usize) = (2, 5);
pub const LIVE: [&str; 6] = ["fire", "aurora", "lava", "stars", "rain", "grain"];
pub const FLAMES: [&str; 4] = ["classic", "blue", "green", "violet"];
pub const KINDS: [&str; 4] = ["linear", "radial", "conic", "mesh"];

/// Zastávka gradientu: barva a poloha 0–100 %.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default)]
pub struct GradStop {
    pub c: String,
    pub p: f32,
}

impl Default for GradStop {
    fn default() -> Self {
        Self { c: String::from("#000000"), p: 0.0 }
    }
}

/// Vlastní gradient uživatele (`id` = `my-…`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default)]
pub struct CustomGradient {
    pub id: String,
    pub name: String,
    /// `linear`, `radial`, `conic`, `mesh`.
    pub kind: String,
    /// Úhel ve stupních 0–360 (u `radial` se nepoužije).
    pub angle: f32,
    pub stops: Vec<GradStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BackdropConfig {
    /// Hotový (`midnight`, `sunset`, …) nebo vlastní (`my-…`) gradient; prázdné = bez gradientu (dnešní vzhled).
    pub id: String,
    /// Živé pozadí: `fire`, `aurora`, `lava`, `stars`, `rain`, `grain`; prázdné = žádné (výchozí).
    pub live: String,
    /// Barva plamene (jen `fire`): `classic`, `blue`, `green`, `violet`.
    pub flame: String,
    /// Síla živého pozadí 0.1–1.
    pub intensity: f32,
    /// Rychlost živého pozadí 0.3–2.
    pub speed: f32,
    /// Průsvitnost karet nad pozadím: `auto` (průsvitné jen když je pozadí zvolené), `solid`, `tint`, `glass`.
    pub cards: String,
    /// Vlastní gradienty, nejvýš 20.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<CustomGradient>,
}

impl Default for BackdropConfig {
    fn default() -> Self {
        Self { id: String::new(), live: String::new(), flame: String::from("classic"), intensity: 0.6, speed: 1.0, cards: String::from("auto"), custom: Vec::new() }
    }
}

impl BackdropConfig {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

fn id_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Ověření sekce (chyby česky); starý config (výchozí hodnoty) projde vždy.
pub fn validate(b: &BackdropConfig) -> Result<(), String> {
    if !b.id.is_empty() && !id_ok(&b.id) {
        return Err(tr("appearance.backdrop.id chce krátké jméno z malých písmen.", "appearance.backdrop.id needs a short lowercase name.").into());
    }
    if !b.live.is_empty() && !LIVE.contains(&b.live.as_str()) {
        return Err(tr("appearance.backdrop.live: fire, aurora, lava, stars, rain, grain (nebo prázdné).", "appearance.backdrop.live: fire, aurora, lava, stars, rain, grain (or empty).").into());
    }
    if !FLAMES.contains(&b.flame.as_str()) {
        return Err(tr("appearance.backdrop.flame: classic, blue, green, violet.", "appearance.backdrop.flame: classic, blue, green, violet.").into());
    }
    if !["auto", "solid", "tint", "glass"].contains(&b.cards.as_str()) {
        return Err(tr("appearance.backdrop.cards: auto, solid, tint, glass.", "appearance.backdrop.cards: auto, solid, tint, glass.").into());
    }
    if !(0.1..=1.0).contains(&b.intensity) || !(0.3..=2.0).contains(&b.speed) {
        return Err(tr("appearance.backdrop.intensity musí být 0.1–1 a speed 0.3–2.", "appearance.backdrop.intensity must be 0.1–1 and speed 0.3–2.").into());
    }
    if b.custom.len() > MAX_CUSTOM {
        return Err(tr("appearance.backdrop.custom: nejvýš 20 vlastních gradientů.", "appearance.backdrop.custom: at most 20 custom gradients.").into());
    }
    for g in &b.custom {
        let bad = !id_ok(&g.id)
            || !KINDS.contains(&g.kind.as_str())
            || !(0.0..=360.0).contains(&g.angle)
            || !(STOPS.0..=STOPS.1).contains(&g.stops.len())
            || g.stops.iter().any(|s| !is_hex(&s.c) || !(0.0..=100.0).contains(&s.p));
        if bad {
            return Err(tr(
                "appearance.backdrop.custom: gradient chce id, kind (linear, radial, conic, mesh), úhel 0–360 a 2–5 zastávek #rrggbb s polohou 0–100.",
                "appearance.backdrop.custom: a gradient needs an id, kind (linear, radial, conic, mesh), angle 0–360 and 2–5 stops #rrggbb at 0–100.",
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grad(f: impl FnOnce(&mut CustomGradient)) -> BackdropConfig {
        let mut g = CustomGradient {
            id: "my-test".into(),
            name: "Test".into(),
            kind: "linear".into(),
            angle: 90.0,
            stops: vec![GradStop { c: "#112233".into(), p: 0.0 }, GradStop { c: "#445566".into(), p: 100.0 }],
        };
        f(&mut g);
        BackdropConfig { custom: vec![g], ..Default::default() }
    }

    #[test]
    fn default_is_todays_look_and_not_written() {
        let b: BackdropConfig = serde_json::from_str("{}").unwrap();
        assert!(b.is_default());
        assert!(validate(&b).is_ok());
        let a = crate::appearance::AppearanceConfig::default();
        let j = serde_json::to_string(&a).unwrap();
        assert!(!j.contains("backdrop") && !j.contains("accent2") && !j.contains("accentFromBackdrop"), "{j}");
    }

    #[test]
    fn validation_table() {
        let cases: Vec<(&str, BackdropConfig, bool)> = vec![
            ("hotový", BackdropConfig { id: "sunset".into(), ..Default::default() }, true),
            ("živý oheň modrý", BackdropConfig { live: "fire".into(), flame: "blue".into(), ..Default::default() }, true),
            ("neznámé živé", BackdropConfig { live: "snow".into(), ..Default::default() }, false),
            ("neznámý plamen", BackdropConfig { flame: "pink".into(), ..Default::default() }, false),
            ("karty tint", BackdropConfig { cards: "tint".into(), ..Default::default() }, true),
            ("karty neznámé", BackdropConfig { cards: "foam".into(), ..Default::default() }, false),
            ("síla 0", BackdropConfig { intensity: 0.0, ..Default::default() }, false),
            ("rychlost 3", BackdropConfig { speed: 3.0, ..Default::default() }, false),
            ("id s mezerou", BackdropConfig { id: "a b".into(), ..Default::default() }, false),
            ("vlastní ok", grad(|_| {}), true),
            ("vlastní 1 zastávka", grad(|g| g.stops.truncate(1)), false),
            ("vlastní 6 zastávek", grad(|g| g.stops = (0..6).map(|i| GradStop { c: "#000000".into(), p: i as f32 * 10.0 }).collect()), false),
            ("vlastní špatná barva", grad(|g| g.stops[0].c = "red".into()), false),
            ("vlastní poloha 101", grad(|g| g.stops[1].p = 101.0), false),
            ("vlastní druh", grad(|g| g.kind = "spiral".into()), false),
            ("vlastní úhel", grad(|g| g.angle = 400.0), false),
        ];
        for (name, b, ok) in cases {
            assert_eq!(validate(&b).is_ok(), ok, "{name}");
        }
    }

    #[test]
    fn too_many_custom() {
        let one = grad(|_| {}).custom.remove(0);
        let b = BackdropConfig { custom: vec![one; MAX_CUSTOM + 1], ..Default::default() };
        assert!(validate(&b).is_err());
    }
}
