//! Rozložení (`layout`), zavřený proužek (`strip`) a vzhled (`appearance`) přes MCP. Validace je ta
//! samá jako u editoru rozložení a Nastavení (`layout::validate` uvnitř `config::validate_field`, volá
//! `tools`); tady čtení s katalogy povolených hodnot a změny. Design styl je balíček hodnot, jehož
//! tabulka žije ve frontendu (`src/theme/looks.ts` `STYLES`) — `STYLES` níž je její zrcadlo (test).

use super::schema;
use crate::appearance::AppearanceConfig;
use crate::config::Config;
use crate::i18n::tr;
use crate::layout::{self, LayoutConfig};
use crate::stripconf::{StripConfig, StripItem};
use serde_json::{Value, json};

// ---------- rozložení ----------

pub fn layout_get(cfg: &Config) -> Value {
    let root = schema::of::<LayoutConfig>();
    json!({
        "layout": cfg.layout,
        "catalog": {
            "cards": layout::CARDS,
            "optInCards": layout::OPT_IN,
            "dynamicCards": "widget:<file> (a script widget from the widgets folder)",
            "parts": layout::PARTS.iter().map(|(p, c)| json!({ "part": p, "card": c })).collect::<Vec<_>>(),
            "builtinTabs": [layout::WORK, layout::PERSONAL],
            "maxTabs": layout::MAX_PAGES,
            "maxHeightRows": layout::MAX_H,
            "maxRowLimit": layout::MAX_LIMIT,
            "values": schema::catalog(&root),
        },
    })
}

/// Nové rozložení ze vstupu `{ layout }` (celé, ne změna). Chybí `upgrades` = převezme se (jinak by se
/// jednorázové úpravy pustily znovu a vrátily, co uživatel odebral).
pub fn layout_parse(cfg: &Config, args: &Value) -> Result<Config, String> {
    let input = args.get("layout").ok_or_else(|| tr("Chybí layout (celé rozložení jako z layout_get).", "layout is missing (the whole layout as from layout_get)."))?;
    let unknown = unknown_cards(input);
    let mut l: LayoutConfig = match schema::parse(input, "layout") {
        Ok(l) if unknown.is_empty() => l,
        Ok(_) => return Err(unknown_cards_msg(&unknown)),
        // serde se zastaví u první chyby; neznámé karty z celého vstupu přidáme, ať model opraví vše najednou
        Err(e) if !unknown.is_empty() => return Err(format!("{e} {}", unknown_cards_msg(&unknown))),
        Err(e) => return Err(e),
    };
    if input.get("upgrades").is_none() {
        l.upgrades = cfg.layout.upgrades.clone();
    }
    let mut next = cfg.clone();
    next.layout = l;
    Ok(next)
}

/// Id karet ve vstupu `{pages:[{cards:[{id}]}]}`, které nejsou v katalogu (`layout::CARDS`) ani dynamické
/// (`widget:<soubor>`) — všechny, bez duplicit, v pořadí ve vstupu.
pub fn unknown_cards(layout: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in layout.get("pages").and_then(Value::as_array).into_iter().flatten().flat_map(|p| p.get("cards").and_then(Value::as_array).into_iter().flatten()).filter_map(|c| c.get("id").and_then(Value::as_str)) {
        if !layout::CARDS.contains(&id) && !layout::is_dynamic(id) && !out.iter().any(|x| x == id) {
            out.push(id.to_string());
        }
    }
    out
}

fn unknown_cards_msg(unknown: &[String]) -> String {
    let list = unknown.join(", ");
    let allowed = format!("{}, widget:<{}>", layout::CARDS.join(", "), tr("soubor", "file"));
    if unknown.len() == 1 {
        trf!("Neznámá karta: {list}. Povolené: {allowed}.", "Unknown card: {list}. Allowed: {allowed}.")
    } else {
        trf!("Neznámé karty: {list}. Povolené: {allowed}.", "Unknown cards: {list}. Allowed: {allowed}.")
    }
}

/// Lidsky: co bude na záložkách a co se proti dnešku změní.
pub fn layout_summary(old: &LayoutConfig, new: &LayoutConfig) -> String {
    let size = |c: &layout::LayoutCard| serde_json::to_value(c.size).ok().and_then(|v| v.as_str().map(str::to_uppercase)).unwrap_or_default();
    let mut lines: Vec<String> = new
        .pages
        .iter()
        .map(|p| {
            let cards: Vec<String> = p.cards.iter().map(|c| format!("{} {}", c.id, size(c))).collect();
            format!("{}: {}", layout::title(p), if cards.is_empty() { "—".into() } else { cards.join(", ") })
        })
        .collect();
    let added: Vec<&str> = new.pages.iter().flat_map(|p| &p.cards).map(|c| c.id.as_str()).filter(|id| !layout::placed(old, id)).collect();
    let gone: Vec<&str> = old.pages.iter().flat_map(|p| &p.cards).map(|c| c.id.as_str()).filter(|id| !layout::placed(new, id)).collect();
    if !added.is_empty() {
        lines.push(trf!("Nově v šuplíku: {}", "New in the drawer: {}", added.join(", ")));
    }
    if !gone.is_empty() {
        lines.push(trf!("Skryté (na žádné záložce): {}", "Hidden (on no tab): {}", gone.join(", ")));
    }
    lines.join("\n")
}

// ---------- proužek ----------

pub fn strip_get(cfg: &Config) -> Value {
    let root = schema::of::<StripConfig>();
    let items: Vec<String> = StripItem::BUILTIN.iter().map(StripItem::key).collect();
    json!({
        "strip": cfg.strip,
        "catalog": { "items": items, "widgetItem": "widget:<file>", "values": schema::catalog(&root), "customMaxChars": crate::stripconf::CUSTOM_MAX },
    })
}

/// Změna proužku (sloučení; `itemsRev` je vnitřní a měnit nejde).
pub fn strip_set(cfg: &Config, args: &Value) -> Result<Config, String> {
    if args.get("itemsRev").is_some() {
        return Err(tr("itemsRev je vnitřní značka Ledge, neměň ji.", "itemsRev is Ledge's internal marker, don't change it.").into());
    }
    let mut next = cfg.clone();
    next.strip = schema::patch(&cfg.strip, args, "strip")?;
    Ok(next)
}

// ---------- vzhled ----------

/// Design styl: hodnoty, které zapíše výběr stylu (zrcadlo `STYLES` v `src/theme/looks.ts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub id: &'static str,
    pub theme: &'static str,
    pub density: &'static str,
    pub font_scale: f32,
    pub radius: &'static str,
    pub opacity: f32,
    pub strip: &'static str,
    pub strip_size: &'static str,
    pub font: &'static str,
    pub material: &'static str,
    pub layout: &'static str,
    pub width: &'static str,
    pub blur: bool,
}

#[rustfmt::skip]
pub const STYLES: [Style; 13] = [
    Style { id: "classic", theme: "dark", density: "normal", font_scale: 1.0, radius: "normal", opacity: 1.0, strip: "pill", strip_size: "normal", font: "system", material: "solid", layout: "classic", width: "custom", blur: false },
    Style { id: "glass", theme: "midnight", density: "normal", font_scale: 1.0, radius: "round", opacity: 0.82, strip: "pill", strip_size: "normal", font: "system", material: "glass", layout: "float", width: "medium", blur: false },
    Style { id: "terminal", theme: "terminal", density: "compact", font_scale: 1.0, radius: "sharp", opacity: 1.0, strip: "term", strip_size: "normal", font: "mono", material: "frame", layout: "band", width: "full", blur: false },
    Style { id: "paper", theme: "paper", density: "comfy", font_scale: 1.05, radius: "normal", opacity: 1.0, strip: "tag", strip_size: "normal", font: "serif", material: "paper", layout: "sheet", width: "narrow", blur: false },
    Style { id: "neon", theme: "neon", density: "normal", font_scale: 1.0, radius: "round", opacity: 0.94, strip: "glow", strip_size: "normal", font: "system", material: "glow", layout: "panorama", width: "wide", blur: false },
    Style { id: "brutal", theme: "brutal", density: "normal", font_scale: 1.0, radius: "sharp", opacity: 1.0, strip: "block", strip_size: "normal", font: "grotesk", material: "hard", layout: "mosaic", width: "medium", blur: false },
    Style { id: "pastel", theme: "pastel", density: "comfy", font_scale: 1.0, radius: "round", opacity: 1.0, strip: "drop", strip_size: "normal", font: "system", material: "soft", layout: "tiles", width: "medium", blur: false },
    Style { id: "aurora", theme: "aurora", density: "comfy", font_scale: 1.0, radius: "round", opacity: 0.9, strip: "pill", strip_size: "normal", font: "system", material: "glass", layout: "panorama", width: "wide", blur: false },
    Style { id: "mono", theme: "mono", density: "comfy", font_scale: 1.0, radius: "sharp", opacity: 1.0, strip: "line", strip_size: "small", font: "system", material: "solid", layout: "sheet", width: "medium", blur: false },
    Style { id: "retro", theme: "retro", density: "normal", font_scale: 1.0, radius: "sharp", opacity: 1.0, strip: "block", strip_size: "normal", font: "mono", material: "hard", layout: "mosaic", width: "medium", blur: false },
    Style { id: "fluent", theme: "dark", density: "normal", font_scale: 1.0, radius: "normal", opacity: 0.88, strip: "pill", strip_size: "normal", font: "system", material: "acrylic", layout: "panel", width: "medium", blur: true },
    Style { id: "oled", theme: "oled", density: "normal", font_scale: 1.0, radius: "normal", opacity: 1.0, strip: "line", strip_size: "normal", font: "system", material: "solid", layout: "classic", width: "custom", blur: false },
    Style { id: "warm", theme: "warm", density: "normal", font_scale: 1.0, radius: "round", opacity: 0.95, strip: "pill", strip_size: "normal", font: "system", material: "tinted", layout: "mosaic", width: "medium", blur: false },
];

pub fn appearance_get(cfg: &Config) -> Value {
    let root = schema::of::<AppearanceConfig>();
    let mut values = schema::catalog(&root);
    values.remove("style");
    json!({
        "appearance": cfg.appearance,
        "blur": cfg.drawer.glass == crate::config::Glass::Acrylic,
        "catalog": {
            "styles": STYLES.iter().map(|s| s.id).collect::<Vec<_>>(),
            "values": values,
            "accent": "#rrggbb or \"\" (theme default)",
            "fontScale": "0.85–1.25",
            "glass.opacity": "0.25–1",
            "motion.speed": "0.5–2",
            "strip.color": "\"\", \"accent\" or #rrggbb",
        },
    })
}

/// Vybere styl jako galerie v Nastavení (`withStyle` + `configWithStyle` ve frontendu).
fn with_style(cfg: &Config, s: &Style) -> Result<Config, String> {
    let change = json!({
        "style": s.id, "theme": s.theme, "accent": "", "density": s.density, "fontScale": s.font_scale, "radius": s.radius,
        "glass": { "opacity": s.opacity }, "strip": { "style": s.strip, "size": s.strip_size },
        "font": s.font, "material": s.material, "layout": s.layout, "width": s.width,
    });
    let mut next = cfg.clone();
    next.appearance = schema::patch(&cfg.appearance, &change, "appearance")?;
    next.drawer.glass = if s.blur { crate::config::Glass::Acrylic } else { crate::config::Glass::Solid };
    Ok(next)
}

/// `{ style?, blur?, …pole appearance }`: nejdřív styl (celý balíček), pak jednotlivá pole navrch.
pub fn appearance_set(cfg: &Config, args: &Value) -> Result<Config, String> {
    let Some(m) = args.as_object() else { return Err(tr("Čekám objekt se změnami vzhledu.", "Expected an object with appearance changes.").into()) };
    let mut next = cfg.clone();
    if let Some(id) = m.get("style") {
        let id = id.as_str().unwrap_or_default();
        let ids: Vec<&str> = STYLES.iter().map(|s| s.id).collect();
        let s = STYLES.iter().find(|s| s.id == id).ok_or_else(|| trf!("Styl „{id}“ neznám (jde: {}).", "Unknown style “{id}” (available: {}).", ids.join(", ")))?;
        next = with_style(&next, s)?;
    }
    if let Some(b) = m.get("blur") {
        let b = b.as_bool().ok_or_else(|| tr("blur je true / false.", "blur is true / false."))?;
        next.drawer.glass = if b { crate::config::Glass::Acrylic } else { crate::config::Glass::Solid };
    }
    let mut rest = m.clone();
    rest.remove("style");
    rest.remove("blur");
    if !rest.is_empty() {
        next.appearance = schema::patch(&next.appearance, &Value::Object(rest), "appearance")?;
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    /// `parts: { theme: "dark", fontScale: 1, … }` → mapa klíč → hodnota (bez uvozovek).
    fn parts(text: &str) -> BTreeMap<String, String> {
        text.split(',').filter_map(|kv| kv.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().trim_matches('"').to_string())).collect()
    }

    #[test]
    fn unknown_cards_are_all_listed() {
        let cfg = Config::default();
        let msg = |v: Value| layout_parse(&cfg, &json!({ "layout": v })).err().unwrap_or_default();
        let two = json!({ "pages": [{ "id": "prace", "cards": [{ "id": "foo" }, { "id": "projects" }, { "id": "bar" }, { "id": "foo" }] }] });
        let m = msg(two);
        assert!(m.starts_with("Neznámé karty: foo, bar. Povolené: projects, running,"), "{m}");
        assert!(m.contains("widget:<soubor>"), "{m}");
        // jedna neznámá karta + chyba serde jinde: obojí v jedné větě
        let both = json!({ "pages": [{ "id": "prace", "cards": [{ "id": "foo", "size": "gigantic" }, { "id": "baz" }] }] });
        let m = msg(both);
        assert!(m.contains("size") || m.contains("gigantic") || m.contains("variant"), "serde chyba chybí: {m}");
        assert!(m.contains("Neznámé karty: foo, baz."), "{m}");
        // dynamická a známé karty projdou; bez neznámých se chyby serde nemění
        let ok = json!({ "pages": [{ "id": "prace", "cards": [{ "id": "projects" }, { "id": "widget:cpu.ps1" }] }] });
        assert!(layout_parse(&cfg, &json!({ "layout": ok })).is_ok());
        assert!(msg(json!({ "pages": [{ "id": "prace", "cards": [{ "id": "clock", "size": "gigantic" }] }] })).starts_with("layout: "));
        assert_eq!(unknown_cards(&json!({ "pages": [{ "cards": [{ "id": "a" }, { "nic": 1 }] }, { "cards": [{ "id": "a" }, { "id": "notes" }] }] })), ["a"]);
        assert!(unknown_cards(&json!({})).is_empty());
    }

    #[test]
    fn styles_mirror_frontend_table() {
        let ts = include_str!("../../../src/theme/looks.ts");
        let start = ts.find("export const STYLES").unwrap();
        let block = &ts[start..ts[start..].find("\n];").map(|e| start + e).unwrap()];
        let mut seen = Vec::new();
        for chunk in block.split("id: ").skip(1) {
            let id = chunk.split([',', '\n']).next().unwrap().trim().trim_matches('"');
            let id = if id == "CLASSIC" { "classic" } else { id };
            let p = chunk.split("parts: {").nth(1).unwrap().split('}').next().unwrap();
            let ts_parts = parts(p);
            let s = STYLES.iter().find(|s| s.id == id).unwrap_or_else(|| panic!("styl {id} chybí v Rust STYLES"));
            let want: BTreeMap<String, String> = [
                ("theme", s.theme.to_string()),
                ("density", s.density.into()),
                ("fontScale", s.font_scale.to_string()),
                ("radius", s.radius.into()),
                ("opacity", s.opacity.to_string()),
                ("strip", s.strip.into()),
                ("stripSize", s.strip_size.into()),
                ("font", s.font.into()),
                ("material", s.material.into()),
                ("layout", s.layout.into()),
                ("width", s.width.into()),
                ("blur", s.blur.to_string()),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
            assert_eq!(ts_parts, want, "styl {id}");
            seen.push(id.to_string());
        }
        assert_eq!(seen, STYLES.iter().map(|s| s.id.to_string()).collect::<Vec<_>>(), "pořadí a počet stylů");
        // každý styl projde validací a typy configu
        for s in &STYLES {
            let c = with_style(&Config::default(), s).unwrap();
            assert!(crate::config::validate(&c).is_ok(), "{}", s.id);
            assert_eq!(c.appearance.style, s.id);
        }
    }
}
