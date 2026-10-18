//! Vzhled Ledge (`config.appearance`): motiv, akcent, hustota, písmo, rádiusy, krytí pozadí šuplíku,
//! styl a rychlost animací, vzhled a pozice proužku. Výchozí hodnoty = vzhled před
//! zavedením sekce (starý config bez `appearance` vypadá stejně). Frontend aplikuje motiv přes CSS
//! proměnné (`src/theme/*`), Rust z toho bere jen proužek (`geom`) a pozici šuplíku.
//! Šířka šuplíku a rozmazání zůstávají v `drawer.width` / `drawer.glass` (nezdvojovat). Pohyb šuplíku
//! (i délka vysunutí okna) se řídí JEN `motion` — starý `drawer.animationMs` převádí `motion::upgrade_loaded`.
//! Vlna 6: `style` = hotový balíček (barvy, tvar, písmo, materiál, hustota, proužek, rozložení a šířka);
//! tabulka stylů je jen ve frontendu (`src/theme/styles.ts`) — výběr stylu zapíše jeho hodnoty do
//! polí níž, ta zůstávají přebitelná („Upravit detaily“). Rust z nových polí potřebuje jen šířku
//! (`width`, `geom::drawer_width_for`) a tvar proužku (`stripdeco`). Starý config = styl `classic`.

pub mod geom;
pub mod look;
pub mod motion;
pub mod stripdeco;

pub use look::{FontLook, Layout, Material, WidthPreset};

use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppearanceConfig {
    /// Design styl (balíček vzhledu): `classic` (výchozí = vzhled před vlnou 6), `glass`, `terminal`,
    /// `paper`, `neon`, `brutal`, `pastel`. Jen jméno — hodnoty stylu jsou v polích níž (výběr stylu je
    /// do nich zapíše, „Obnovit styl“ je vrátí). Neznámé jméno frontend bere jako `classic`.
    pub style: String,
    /// Barevný motiv: `dark` (výchozí), `midnight`, `graphite`, `nord`, `light`, `contrast`.
    pub theme: Theme,
    /// Barva akcentu `#rrggbb` (podpisová linka, záře, fokus). Prázdné = výchozí fialovo-modrá Ledge.
    pub accent: String,
    /// Hustota mezer: `compact`, `normal`, `comfy`.
    pub density: Density,
    /// Velikost písma (násobek, 0.85–1.25).
    pub font_scale: f32,
    /// Zaoblení rohů: `sharp`, `normal`, `round`.
    pub radius: Radius,
    /// Krytí pozadí šuplíku. Rozmazání plochy za šuplíkem je `drawer.glass` („acrylic“).
    pub glass: GlassLook,
    /// Styl a rychlost animací.
    pub motion: MotionLook,
    /// Vzhled nativního proužku nahoře. Obsah a krytí proužku jsou v `strip`.
    pub strip: StripLook,
    // Dřívější `cards` (pořadí karet Práce) převádí `layout::upgrade_loaded` do `layout` — jedno místo.
    /// Písmo (jen systémová písma Windows): `system` (Segoe UI Variable), `mono` (Cascadia Mono),
    /// `serif` (patkové nadpisy Sitka/Georgia), `grotesk` (Bahnschrift).
    pub font: FontLook,
    /// Materiál ploch: `solid` (výchozí), `glass`, `paper`, `glow`, `hard`, `soft`, `frame`.
    pub material: Material,
    /// Rozložení hlavní stránky: `classic` (výchozí), `float`, `band`, `sheet`, `panorama`, `mosaic`, `tiles`.
    pub layout: Layout,
    /// Předvolba šířky šuplíku: `custom` (výchozí = `drawer.width` v px), `narrow`, `medium`, `wide`, `full`.
    pub width: WidthPreset,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            style: String::from(look::CLASSIC),
            theme: Theme::Dark,
            accent: String::new(),
            density: Density::Normal,
            font_scale: 1.0,
            radius: Radius::Normal,
            glass: GlassLook::default(),
            motion: MotionLook::default(),
            strip: StripLook::default(),
            font: FontLook::default(),
            material: Material::default(),
            layout: Layout::default(),
            width: WidthPreset::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    /// tmavě modrošedý (dnešní vzhled)
    #[default]
    Dark,
    /// hluboká noční modř
    Midnight,
    /// neutrální grafit bez modrého nádechu
    Graphite,
    /// studená severská paleta
    Nord,
    /// světlý
    Light,
    /// vysoký kontrast (černá, bílá, výrazné okraje)
    Contrast,
    /// zelená na černé (styl Terminál)
    Terminal,
    /// teplý světlý papír (styl Papír)
    Paper,
    /// tmavě fialová se zářícími akcenty (styl Neon)
    Neon,
    /// bílá, černé okraje, výrazné barvy (styl Brutal)
    Brutal,
    /// světlé pastelové šedomodré (styl Pastel)
    Pastel,
}

#[cfg(test)]
impl Theme {
    /// Všechny motivy (testy kontrastu a zrcadel TS).
    pub const ALL: [Theme; 11] = [
        Theme::Dark,
        Theme::Midnight,
        Theme::Graphite,
        Theme::Nord,
        Theme::Light,
        Theme::Contrast,
        Theme::Terminal,
        Theme::Paper,
        Theme::Neon,
        Theme::Brutal,
        Theme::Pastel,
    ];
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Density {
    Compact,
    #[default]
    Normal,
    Comfy,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Radius {
    Sharp,
    #[default]
    Normal,
    Round,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GlassLook {
    /// Krytí pozadí šuplíku jako násobek krytí motivu (0.25–1; 1 = výchozí). Méně = víc prosvítá plocha.
    pub opacity: f32,
}

impl Default for GlassLook {
    fn default() -> Self {
        Self { opacity: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct MotionLook {
    /// `spring` = pružina s překmitem (výchozí), `smooth` = plynule bez překmitu, `snappy` = svižně,
    /// `bounce` = dopadne a odskočí, `fade` = rozplynutí, `notch` = rozbalení z proužku (nejdřív do
    /// šířky, pak dolů), `none` = bez animací (okamžitě). Systémové „omezit pohyb“ se chová jako `none`
    /// (frontend i vysunutí okna v Rustu, `motion::anim_ms_now`).
    pub style: MotionStyle,
    /// Rychlost animací (násobek, 0.5–2; 2 = dvakrát rychleji).
    pub speed: f32,
}

impl Default for MotionLook {
    fn default() -> Self {
        Self { style: MotionStyle::Spring, speed: 1.0 }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MotionStyle {
    #[default]
    Spring,
    Smooth,
    Snappy,
    Bounce,
    Fade,
    Notch,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct StripLook {
    /// `pill` = zaoblený proužek s textem (výchozí), `line` = plochý pruh s hranatými rohy,
    /// `dot` = jen barevné tečky stavů (bez textu), `term` = ostrý terminálový řádek (neproporcionální
    /// písmo), `tag` = papírový štítek, `glow` = neonová linka, `block` = brutální blok s tvrdým stínem,
    /// `drop` = měkká kapka. Tvar a ozdoby: `stripdeco`.
    pub style: StripStyle,
    /// Velikost: `small`, `normal`, `large`.
    pub size: StripSize,
    /// Barva podpisové linky: prázdné = přechod akcentu (motivu nebo vlastního, jako v šuplíku),
    /// `accent` = jednobarevně v akcentu, nebo vlastní `#rrggbb`.
    pub color: String,
    /// Kde u horní hrany Ledge sedí (proužek i šuplík): `center`, `left`, `right`.
    pub position: StripPosition,
}

impl Default for StripLook {
    fn default() -> Self {
        Self { style: StripStyle::Pill, size: StripSize::Normal, color: String::new(), position: StripPosition::Center }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StripStyle {
    #[default]
    Pill,
    Line,
    Dot,
    Term,
    Tag,
    Glow,
    Block,
    Drop,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StripSize {
    Small,
    #[default]
    Normal,
    Large,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StripPosition {
    #[default]
    Center,
    Left,
    Right,
}

/// Rozsah šířky šuplíku (logické px) pro panel a ořez za běhu (`geom::drawer_width`). `drawer.width`
/// se záměrně nevaliduje (starý config s jinou šířkou se nesmí přestat načítat) — jen se ořízne.
pub const DRAWER_WIDTH: (u32, u32) = (720, 2400);

/// `#rrggbb`.
pub fn is_hex(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validace sekce (chyby česky). Nevalidní config se neaplikuje — proto jen nová pole.
pub fn validate(a: &AppearanceConfig) -> Result<(), String> {
    if !look::style_id_ok(&a.style) {
        return Err(trf!("appearance.style „{}“ chce krátké jméno z malých písmen (např. classic, neon).", "appearance.style “{}” needs a short lowercase name (e.g. classic, neon).", a.style));
    }
    if !a.accent.is_empty() && !is_hex(&a.accent) {
        return Err(trf!("appearance.accent „{}“ chce tvar #rrggbb (nebo prázdné = výchozí).", "appearance.accent “{}” needs the form #rrggbb (or empty = default).", a.accent));
    }
    if !(0.85..=1.25).contains(&a.font_scale) {
        return Err(tr("appearance.fontScale musí být 0.85–1.25.", "appearance.fontScale must be 0.85–1.25.").into());
    }
    if !(0.25..=1.0).contains(&a.glass.opacity) {
        return Err(tr("appearance.glass.opacity musí být 0.25–1 (níž by text na ploše nešel číst).", "appearance.glass.opacity must be 0.25–1 (lower, text on the surface would be unreadable).").into());
    }
    if !(0.5..=2.0).contains(&a.motion.speed) {
        return Err(tr("appearance.motion.speed musí být 0.5–2.", "appearance.motion.speed must be 0.5–2.").into());
    }
    let c = &a.strip.color;
    if !c.is_empty() && c != "accent" && !is_hex(c) {
        return Err(trf!("appearance.strip.color „{c}“ chce prázdné, „accent“ nebo #rrggbb.", "appearance.strip.color “{c}” needs empty, “accent” or #rrggbb."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(f: impl FnOnce(&mut AppearanceConfig)) -> Result<(), String> {
        let mut a = AppearanceConfig::default();
        f(&mut a);
        validate(&a)
    }

    #[test]
    fn empty_object_is_todays_look() {
        let a: AppearanceConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(a, AppearanceConfig::default());
        assert_eq!(a.theme, Theme::Dark);
        assert!(a.accent.is_empty());
        assert_eq!((a.font_scale, a.glass.opacity, a.motion.speed), (1.0, 1.0, 1.0));
        assert_eq!(a.strip, StripLook::default());
        assert!(validate(&a).is_ok());
    }

    #[test]
    fn partial_json_keeps_other_defaults() {
        let a: AppearanceConfig =
            serde_json::from_str(r#"{"theme":"nord","motion":{"style":"none"},"strip":{"position":"left"}}"#).unwrap();
        assert_eq!(a.theme, Theme::Nord);
        assert_eq!(a.motion, MotionLook { style: MotionStyle::None, speed: 1.0 });
        assert_eq!(a.strip.position, StripPosition::Left);
        assert_eq!(a.strip.style, StripStyle::Pill);
    }

    #[test]
    fn validation_table() {
        type Case = (&'static str, Box<dyn FnOnce(&mut AppearanceConfig)>, bool);
        let cases: Vec<Case> = vec![
            ("akcent hex", Box::new(|a| a.accent = "#22c55e".into()), true),
            ("akcent zkrácený", Box::new(|a| a.accent = "#2c5".into()), false),
            ("akcent jméno", Box::new(|a| a.accent = "red".into()), false),
            ("písmo dolní mez", Box::new(|a| a.font_scale = 0.85), true),
            ("písmo pod mez", Box::new(|a| a.font_scale = 0.8), false),
            ("písmo nad mez", Box::new(|a| a.font_scale = 1.3), false),
            ("krytí 0.25", Box::new(|a| a.glass.opacity = 0.25), true),
            ("krytí 0", Box::new(|a| a.glass.opacity = 0.0), false),
            ("rychlost 2", Box::new(|a| a.motion.speed = 2.0), true),
            ("rychlost 3", Box::new(|a| a.motion.speed = 3.0), false),
            ("linka akcent", Box::new(|a| a.strip.color = "accent".into()), true),
            ("linka hex", Box::new(|a| a.strip.color = "#ffffff".into()), true),
            ("linka nesmysl", Box::new(|a| a.strip.color = "modrá".into()), false),
        ];
        for (name, f, want) in cases {
            assert_eq!(ok(f).is_ok(), want, "{name}");
        }
    }

    #[test]
    fn schema_and_old_config() {
        let schema = crate::config::schema_json();
        for needle in ["\"appearance\"", "fontScale", "midnight", "snappy", "\"dot\"", "material", "panorama", "notch", "grotesk"] {
            assert!(schema.contains(needle), "{needle} ve schématu");
        }
        // starý config bez sekce: výchozí vzhled a projde validací
        let cfg: crate::config::Config = serde_json::from_str(r#"{"drawer":{"width":1080}}"#).unwrap();
        assert_eq!(cfg.appearance, AppearanceConfig::default());
        assert!(crate::config::validate(&cfg).is_ok());
    }

    #[test]
    fn old_config_before_styles_loads_unchanged() {
        // skutečný tvar configu z vlny 5 (sekce appearance bez style/font/material/layout/width)
        let old = r##"{
          "drawer": { "width": 1300, "glass": "acrylic", "trigger": "hover" },
          "appearance": {
            "theme": "nord", "accent": "#22c55e", "density": "compact", "fontScale": 1.1, "radius": "round",
            "glass": { "opacity": 0.8 }, "motion": { "style": "smooth", "speed": 1.5 },
            "strip": { "style": "line", "size": "large", "color": "accent", "position": "left" },
            "cards": ["notes", "claude"]
          },
          "strip": { "opacity": 0.7 }
        }"##;
        let raw: serde_json::Value = serde_json::from_str(old).unwrap();
        let mut cfg = crate::config::Config::deserialize(&raw).unwrap();
        motion::upgrade_loaded(&raw, &mut cfg);
        let a = &cfg.appearance;
        assert_eq!(a.style, "classic", "starý config = styl Klasický");
        assert_eq!((a.font, a.material, a.layout, a.width), (FontLook::System, Material::Solid, Layout::Classic, WidthPreset::Custom));
        // jeho vlastní hodnoty zůstávají (styl je jen jméno, nic nepřepisuje)
        assert_eq!((a.theme, a.density, a.radius, a.font_scale), (Theme::Nord, Density::Compact, Radius::Round, 1.1));
        assert_eq!((a.strip.style, a.strip.position, a.motion.style), (StripStyle::Line, StripPosition::Left, MotionStyle::Smooth));
        assert_eq!(geom::drawer_width_for(&cfg, 1920, 96), 1300.0, "šířka dál z drawer.width");
        assert!(crate::config::validate(&cfg).is_ok());
        // uložení a znovunačtení drží totéž (nová pole se zapíšou s výchozí hodnotou)
        let again: crate::config::Config = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(again.appearance, cfg.appearance);
    }

    #[test]
    fn style_fields_roundtrip_and_validate() {
        let a: AppearanceConfig = serde_json::from_str(
            r#"{"style":"terminal","theme":"terminal","font":"mono","material":"frame","layout":"band","width":"full","strip":{"style":"term"},"motion":{"style":"notch"}}"#,
        )
        .unwrap();
        assert_eq!((a.theme, a.font, a.material, a.layout, a.width), (Theme::Terminal, FontLook::Mono, Material::Frame, Layout::Band, WidthPreset::Full));
        assert_eq!((a.strip.style, a.motion.style), (StripStyle::Term, MotionStyle::Notch));
        assert!(validate(&a).is_ok());
        for bad in ["", "Neon", "neon styl", "x1"] {
            assert!(ok(|a| a.style = bad.into()).is_err(), "{bad:?}");
        }
        for t in ["\"paper\"", "\"neon\"", "\"brutal\"", "\"pastel\""] {
            assert!(serde_json::from_str::<Theme>(t).is_ok(), "{t}");
        }
        for m in ["\"bounce\"", "\"fade\"", "\"notch\""] {
            assert!(serde_json::from_str::<MotionStyle>(m).is_ok(), "{m}");
        }
        for s in ["\"tag\"", "\"glow\"", "\"block\"", "\"drop\""] {
            assert!(serde_json::from_str::<StripStyle>(s).is_ok(), "{s}");
        }
    }

    #[test]
    fn enums_serialize_lowercase() {
        let s = serde_json::to_string(&AppearanceConfig::default()).unwrap();
        for needle in ["\"theme\":\"dark\"", "\"density\":\"normal\"", "\"fontScale\":1.0", "\"style\":\"spring\"", "\"position\":\"center\"", "\"style\":\"pill\""] {
            assert!(s.contains(needle), "{needle} v {s}");
        }
    }
}
