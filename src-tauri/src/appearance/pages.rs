//! Vzhled záložek (`appearance.pages`) a vlastní motivy (`appearance.customThemes`, výběr
//! `appearance.customTheme`). Frontend (`src/theme/pages.ts`, `customThemes.ts`) z nich skládá vzhled
//! aktivní záložky; Rust z nich bere jen validaci configu a paletu nativního proužku (`palette_for`).
//! Vlastní motiv = vestavěný motiv `base` (vrstvy karet a linek) + celá sada upravitelných barev
//! `#rrggbb` (`EDITABLE`, zrcadlo `CUSTOM_KEYS` v `themes.ts`/`customThemes.ts`). Text musí mít na pozadí
//! kontrast ≥ 4.5:1 (editor v Nastavení uložení s nižším kontrastem zablokuje, Rust je poslední pojistka).

use super::geom::{contrast, parse_hex, palette, Palette};
use super::{is_hex, look::style_id_ok, AppearanceConfig, Theme};
use crate::strip::Rgba;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Nejvýš tolik vlastních motivů.
pub const MAX_CUSTOM_THEMES: usize = 20;
/// Nejvýš tolik záložek s vlastním vzhledem (jako `layout::MAX_PAGES` s rezervou).
pub const MAX_PAGE_LOOKS: usize = 64;
/// Barvy, které vlastní motiv ukládá (všechny povinné). Zrcadlo `CUSTOM_KEYS` v `src/theme/customThemes.ts`.
pub const EDITABLE: [&str; 11] = ["bg", "raised", "t1", "t2", "t3", "accent", "accent-2", "green", "amber", "red", "blue"];
/// Texty (a barvy stavů), které musí na pozadí mít kontrast ≥ 4.5:1.
pub const TEXT_KEYS: [&str; 7] = ["t1", "t2", "t3", "green", "amber", "red", "blue"];
/// Hranice čitelného textu (WCAG AA).
pub const MIN_CONTRAST: f32 = 4.5;
/// Svítivost pozadí, nad kterou je motiv světlý (průsečík černého a bílého textu).
const LIGHT_LUM: f32 = 0.18;

/// Vzhled jedné záložky: co je prázdné, drží se celkový vzhled.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct PageLook {
    /// Design styl záložky (jméno jako `appearance.style`); prázdné = jako celek.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub style: String,
    /// Motiv záložky: vestavěný (`dark`, `nord`, …) nebo `id` vlastního motivu; prázdné = jako celek.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub theme: String,
    /// Akcent `#rrggbb`; prázdné = jako celek.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub accent: String,
}

/// Vlastní motiv uložený v configu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomTheme {
    /// Krátké jméno `a–z`, `0–9`, `-` (1–24 znaků), nesmí být jméno vestavěného motivu.
    pub id: String,
    /// Zobrazené jméno (1–32 znaků).
    pub name: String,
    /// Vestavěný motiv, z něhož vrstvy karet a linek vycházejí (u opačného tónu pozadí se vezme `dark` / `light`).
    pub base: Theme,
    /// Barvy `#rrggbb` podle klíče: `bg`, `raised`, `t1`, `t2`, `t3`, `accent`, `accent-2`, `green`, `amber`, `red`, `blue`.
    pub colors: BTreeMap<String, String>,
}

impl CustomTheme {
    fn color(&self, key: &str) -> Option<Rgba> {
        parse_hex(self.colors.get(key)?)
    }

    /// Světlý motiv (tmavý text na světlém podkladu) podle svítivosti pozadí — zrcadlo `isLightBg` v TS.
    pub fn light(&self) -> bool {
        self.color("bg").is_some_and(|c| super::geom::lum(c) > LIGHT_LUM)
    }
}

/// Vestavěný motiv podle jména z configu (`dark`, `nord`, …).
pub fn builtin_theme(id: &str) -> Option<Theme> {
    serde_json::from_value(serde_json::Value::String(id.to_string())).ok()
}

/// Jméno vlastního motivu: 1–24 znaků `a–z 0–9 -`, začíná písmenem nebo číslicí.
pub fn custom_id_ok(s: &str) -> bool {
    (1..=24).contains(&s.len()) && !s.starts_with('-') && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Vlastní motiv vybraný pro celý vzhled (`appearance.customTheme`), pokud existuje.
pub fn active_custom(a: &AppearanceConfig) -> Option<&CustomTheme> {
    if a.custom_theme.is_empty() {
        return None;
    }
    a.custom_themes.iter().find(|c| c.id == a.custom_theme)
}

/// Chyby jednoho vlastního motivu (prázdné = v pořádku). Kontrast textu je proti pozadí a `raised`.
pub fn theme_problems(c: &CustomTheme) -> Vec<String> {
    let mut out = Vec::new();
    for k in EDITABLE {
        match c.colors.get(k) {
            None => out.push(trf!("chybí barva „{k}“", "missing color “{k}”")),
            Some(v) if !is_hex(v) => out.push(trf!("barva „{k}“ ({v}) chce tvar #rrggbb", "color “{k}” ({v}) needs the form #rrggbb")),
            _ => {}
        }
    }
    for k in c.colors.keys() {
        if !EDITABLE.contains(&k.as_str()) {
            out.push(trf!("neznámý klíč barvy „{k}“", "unknown color key “{k}”"));
        }
    }
    if !out.is_empty() {
        return out;
    }
    let bg = c.color("bg").unwrap_or(Rgba::DIM);
    let raised = c.color("raised").unwrap_or(bg);
    for k in TEXT_KEYS {
        let fg = c.color(k).unwrap_or(Rgba::DIM);
        for (surface, on) in [(bg, "bg"), (raised, "raised")] {
            let r = contrast(fg, surface);
            if r < MIN_CONTRAST {
                out.push(trf!("barva „{k}“ má na „{on}“ kontrast {r:.1}:1 (chce ≥ 4.5:1)", "color “{k}” has contrast {r:.1}:1 on “{on}” (needs ≥ 4.5:1)"));
            }
        }
    }
    out
}

/// Paleta nativního proužku: vestavěná podle `theme`, u vybraného vlastního motivu z jeho pozadí a akcentů.
pub fn palette_for(a: &AppearanceConfig) -> Palette {
    let Some(c) = active_custom(a) else { return palette(a.theme) };
    let light = c.light();
    // vrstvy (okraje) z vestavěného motivu stejného tónu; opačný tón = kanonický tmavý / světlý
    let base = if palette(c.base).light == light { palette(c.base) } else { palette(if light { Theme::Light } else { Theme::Dark }) };
    let bg = c.color("bg").unwrap_or(base.top);
    let mix = |k: f32| Rgba((bg.0 * k).min(1.0), (bg.1 * k).min(1.0), (bg.2 * k).min(1.0), 1.0);
    let (top, bottom) = if light { (bg, mix(0.94)) } else { (mix(0.6), mix(1.25)) };
    let a1 = c.color("accent").unwrap_or(base.accent[0]);
    let a2 = c.color("accent-2").unwrap_or(base.accent[1]);
    Palette { top, bottom, light, accent: [a1, a2, a2], ..base }
}

/// Validace `pages`, `customThemes` a `customTheme` (chyby česky / anglicky podle jazyka UI).
pub fn validate(a: &AppearanceConfig) -> Result<(), String> {
    if a.custom_themes.len() > MAX_CUSTOM_THEMES {
        return Err(trf!("appearance.customThemes smí mít nejvýš {MAX_CUSTOM_THEMES} motivů.", "appearance.customThemes may hold at most {MAX_CUSTOM_THEMES} themes."));
    }
    for (i, c) in a.custom_themes.iter().enumerate() {
        if !custom_id_ok(&c.id) || builtin_theme(&c.id).is_some() {
            return Err(trf!("appearance.customThemes[{i}].id „{}“ chce krátké jméno (a–z, 0–9, -), které není vestavěný motiv.", "appearance.customThemes[{i}].id “{}” needs a short name (a–z, 0–9, -) that is not a built-in theme.", c.id));
        }
        if a.custom_themes[..i].iter().any(|d| d.id == c.id) {
            return Err(trf!("appearance.customThemes: jméno „{}“ je dvakrát.", "appearance.customThemes: the id “{}” appears twice.", c.id));
        }
        let name = c.name.trim();
        if name.is_empty() || name.chars().count() > 32 {
            return Err(trf!("appearance.customThemes[{i}].name chce 1–32 znaků.", "appearance.customThemes[{i}].name needs 1–32 characters."));
        }
        let probs = theme_problems(c);
        if !probs.is_empty() {
            return Err(trf!("appearance.customThemes „{}“: {}.", "appearance.customThemes “{}”: {}.", c.id, probs.join("; ")));
        }
    }
    let known = |id: &str| builtin_theme(id).is_some() || a.custom_themes.iter().any(|c| c.id == id);
    if !a.custom_theme.is_empty() && !a.custom_themes.iter().any(|c| c.id == a.custom_theme) {
        return Err(trf!("appearance.customTheme „{}“ není mezi customThemes.", "appearance.customTheme “{}” is not among customThemes.", a.custom_theme));
    }
    if a.pages.len() > MAX_PAGE_LOOKS {
        return Err(trf!("appearance.pages smí mít nejvýš {MAX_PAGE_LOOKS} záložek.", "appearance.pages may hold at most {MAX_PAGE_LOOKS} tabs."));
    }
    for (id, p) in &a.pages {
        if id.is_empty() || id.chars().count() > 64 {
            return Err(tr_page_id());
        }
        if !p.style.is_empty() && !style_id_ok(&p.style) {
            return Err(trf!("appearance.pages.{id}.style „{}“ chce krátké jméno z malých písmen (např. classic, neon).", "appearance.pages.{id}.style “{}” needs a short lowercase name (e.g. classic, neon).", p.style));
        }
        if !p.theme.is_empty() && !known(&p.theme) {
            return Err(trf!("appearance.pages.{id}.theme „{}“ není vestavěný ani vlastní motiv.", "appearance.pages.{id}.theme “{}” is neither a built-in nor a custom theme.", p.theme));
        }
        if !p.accent.is_empty() && !is_hex(&p.accent) {
            return Err(trf!("appearance.pages.{id}.accent „{}“ chce tvar #rrggbb (nebo prázdné).", "appearance.pages.{id}.accent “{}” needs the form #rrggbb (or empty).", p.accent));
        }
    }
    Ok(())
}

fn tr_page_id() -> String {
    crate::i18n::tr("appearance.pages: jméno záložky chce 1–64 znaků.", "appearance.pages: a tab id needs 1–64 characters.").into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(id: &str, bg: &str, fg: &str) -> CustomTheme {
        let mut colors = BTreeMap::new();
        for (k, v) in [
            ("bg", bg), ("raised", bg), ("t1", fg), ("t2", fg), ("t3", fg), ("accent", "#a78bfa"), ("accent-2", "#60a5fa"),
            ("green", fg), ("amber", fg), ("red", fg), ("blue", fg),
        ] {
            colors.insert(k.to_string(), v.to_string());
        }
        CustomTheme { id: id.into(), name: "Můj".into(), base: Theme::Dark, colors }
    }
    fn with(c: CustomTheme) -> AppearanceConfig {
        AppearanceConfig { custom_themes: vec![c], ..Default::default() }
    }

    #[test]
    fn old_config_has_no_pages_or_themes() {
        let a: AppearanceConfig = serde_json::from_str(r#"{"theme":"nord"}"#).unwrap();
        assert!(a.pages.is_empty() && a.custom_themes.is_empty() && a.custom_theme.is_empty());
        let s = serde_json::to_string(&a).unwrap();
        assert!(!s.contains("pages") && !s.contains("customTheme"), "prázdné se nezapisují: {s}");
        assert!(super::super::validate(&a).is_ok());
    }

    #[test]
    fn pages_roundtrip_and_validation_table() {
        let src = r##"{"customThemes":[{"id":"my-one","name":"Moje","base":"dark","colors":{"bg":"#101018","raised":"#262a37","t1":"#f4f5f9","t2":"#aeb2c2","t3":"#9a9fb0","accent":"#a78bfa","accent-2":"#60a5fa","green":"#4ade80","amber":"#fbbf24","red":"#f87171","blue":"#60a5fa"}}],
            "pages":{"work":{"style":"terminal","theme":"my-one","accent":"#22c55e"},"personal":{"theme":"nord"}}}"##;
        let a: AppearanceConfig = serde_json::from_str(src).unwrap();
        assert_eq!(a.pages["work"].style, "terminal");
        assert_eq!(a.pages["personal"], PageLook { theme: "nord".into(), ..Default::default() });
        assert!(super::super::validate(&a).is_ok(), "{:?}", super::super::validate(&a));
        let again: AppearanceConfig = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq!(again, a);

        type Case = (&'static str, Box<dyn Fn(&mut AppearanceConfig)>, bool);
        let cases: Vec<Case> = vec![
            ("neznámý motiv", Box::new(|a| a.pages.get_mut("work").unwrap().theme = "nic".into()), false),
            ("vestavěný motiv", Box::new(|a| a.pages.get_mut("work").unwrap().theme = "neon".into()), true),
            ("styl s velkým písmenem", Box::new(|a| a.pages.get_mut("work").unwrap().style = "Neon".into()), false),
            ("akcent mimo tvar", Box::new(|a| a.pages.get_mut("work").unwrap().accent = "red".into()), false),
            ("prázdné záložky jsou jako celek", Box::new(|a| a.pages.insert("x".into(), PageLook::default()).map(|_| ()).unwrap_or(())), true),
            ("prázdné jméno záložky", Box::new(|a| { a.pages.insert(String::new(), PageLook::default()); }), false),
            ("vybraný neexistující motiv", Box::new(|a| a.custom_theme = "zadny".into()), false),
            ("vybraný existující motiv", Box::new(|a| a.custom_theme = "my-one".into()), true),
            ("jméno jako vestavěný", Box::new(|a| a.custom_themes[0].id = "nord".into()), false),
            ("jméno s velkým písmenem", Box::new(|a| a.custom_themes[0].id = "My".into()), false),
            ("prázdné jméno motivu", Box::new(|a| a.custom_themes[0].name = "  ".into()), false),
            ("chybí barva", Box::new(|a| { a.custom_themes[0].colors.remove("t2"); }), false),
            ("cizí klíč", Box::new(|a| { a.custom_themes[0].colors.insert("card".into(), "#ffffff".into()); }), false),
            ("barva mimo tvar", Box::new(|a| { a.custom_themes[0].colors.insert("bg".into(), "rgb(1,2,3)".into()); }), false),
            ("nízký kontrast textu", Box::new(|a| { a.custom_themes[0].colors.insert("t3".into(), "#202030".into()); }), false),
        ];
        for (name, f, want) in cases {
            let mut b = a.clone();
            f(&mut b);
            assert_eq!(super::super::validate(&b).is_ok(), want, "{name}: {:?}", super::super::validate(&b));
        }
    }

    #[test]
    fn theme_count_and_duplicates_are_limited() {
        let mut a = with(theme("my-0", "#101018", "#f4f5f9"));
        for i in 1..=MAX_CUSTOM_THEMES {
            a.custom_themes.push(theme(&format!("my-{i}"), "#101018", "#f4f5f9"));
        }
        assert_eq!(a.custom_themes.len(), MAX_CUSTOM_THEMES + 1);
        assert!(super::super::validate(&a).is_err(), "víc než {MAX_CUSTOM_THEMES}");
        a.custom_themes.truncate(MAX_CUSTOM_THEMES);
        assert!(super::super::validate(&a).is_ok());
        a.custom_themes[3].id = "my-0".into();
        assert!(super::super::validate(&a).is_err(), "dvě stejná jména");
    }

    #[test]
    fn contrast_table_matches_frontend() {
        // stejná tabulka je v `theme.test.mjs` (test „vlastní motiv: kontrast“): bílá na tmavém, tmavá na světlém,
        // šedá na šedé a okraj 4.5:1 (#767676 na bílé = 4.54, #777777 = 4.48)
        for (fg, bg, ok) in [
            ("#f4f5f9", "#101018", true),
            ("#111111", "#ffffff", true),
            ("#808080", "#707070", false),
            ("#767676", "#ffffff", true),
            ("#777777", "#ffffff", false),
        ] {
            let r = contrast(parse_hex(fg).unwrap(), parse_hex(bg).unwrap());
            assert_eq!(r >= MIN_CONTRAST, ok, "{fg} na {bg} = {r:.2}");
        }
    }

    #[test]
    fn light_follows_background_luminance() {
        for (bg, light) in [("#101018", false), ("#262a37", false), ("#707070", false), ("#7a7a7a", true), ("#f8f9fc", true), ("#ffffff", true)] {
            assert_eq!(theme("my-x", bg, "#000000").light(), light, "{bg}");
        }
    }

    #[test]
    fn palette_follows_custom_theme_and_stays_readable() {
        let mut a = AppearanceConfig::default();
        assert_eq!(palette_for(&a), palette(Theme::Dark), "bez vlastního motivu = vestavěný");
        for (bg, fg) in [("#101018", "#f4f5f9"), ("#1b2b1b", "#e8ffe8"), ("#fbf7ee", "#1a1a1a"), ("#ffffff", "#000000")] {
            a.custom_themes = vec![theme("my-x", bg, fg)];
            a.custom_theme = "my-x".into();
            let p = palette_for(&a);
            assert_eq!(p.light, a.custom_themes[0].light(), "{bg}");
            assert_eq!(p.accent[0], parse_hex("#a78bfa").unwrap());
            // text proužku (readable přes ink) má na obou koncích přechodu čitelný kontrast
            let t = super::super::geom::readable(Rgba::TEXT, &p);
            let k = super::super::geom::ink(t, p.light);
            assert!(contrast(k, p.top) >= 4.5 && contrast(k, p.bottom) >= 4.5, "{bg}");
        }
        a.custom_theme = "neexistuje".into();
        assert_eq!(palette_for(&a), palette(a.theme), "neznámý výběr = vestavěný");
    }

    #[test]
    fn schema_lists_pages_and_custom_themes() {
        let schema = crate::config::schema_json();
        for needle in ["customThemes", "customTheme", "\"pages\"", "PageLook"] {
            assert!(schema.contains(needle), "{needle} ve schématu");
        }
    }
}
