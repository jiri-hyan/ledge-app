//! Vzhled jednotlivých zástupců a karet (vlna 8 P): co si uživatel změní na JEDNÉ dlaždici projektu nebo
//! akce (`projects[].look`, `actions[].look` — ikona emoji / systémová ikona / vlastní obrázek, barva
//! podkladu, tvar, vlastní popisek, schovaný popisek), na JEDNÉ kartě záložky (`layout.pages[].cards[].look`
//! — akcent, pozadí, skrytá hlavička, hustota, varianta zobrazení) a složky dlaždic v kartě Projekty
//! (`layout…tiles.folders`). Všechno je volitelné se serde default: chybí = dnešní vzhled, starý config
//! se načte beze změny a nic nového se do něj nepíše, dokud si uživatel něco nezmění.
//!
//! Vlastní obrázek se při výběru převede přes WIC (`iconimg::to_png`, ≤ 128 px; SVG beze změny) do cache
//! `%LOCALAPPDATA%\Ledge\looks\<klíč>.png` a config drží jen klíč (hash obsahu) — originál se smí přesunout
//! nebo smazat. Chybějící soubor v cache = ikona podle Auto (žádná vymyšlená náhrada).

use crate::config::Config;
use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Předvolby barvy podkladu zástupce (zrcadlo `TILE_COLORS` v `src/tilelook/model.ts`). Vlastní barva
/// `#rrggbb` jde taky; kresba (systémová ikona, písmena) je bílá, nebo tmavá — co má větší kontrast.
/// Kreslí UI (`inkDark`); tady jen zrcadlo pro test kontrastu.
#[cfg(test)]
pub const TILE_COLORS: [&str; 12] = [
    "#64748b", "#1f2937", "#dc2626", "#ea580c", "#f59e0b", "#16a34a", "#0d9488", "#0891b2", "#2563eb", "#4f46e5", "#7c3aed", "#db2777",
];
/// Akcent karty: barvy motivu (mění se s motivem) nebo vlastní `#rrggbb` (zrcadlo `CARD_ACCENTS` v TS).
pub const CARD_ACCENTS: [&str; 7] = ["accent", "blue", "green", "amber", "red", "violet", "teal"];
/// Nejdelší vlastní popisek dlaždice (znaků).
pub const MAX_LABEL: usize = 40;
const MAX_EMOJI: usize = 8;
const MAX_FOLDER_NAME: usize = 32;
const MAX_FOLDERS: usize = 32;
const MAX_FOLDER_ITEMS: usize = 64;
/// Největší obrázek, který se při výběru čte (fotka z telefonu; výsledek je ≤ 128 px).
const MAX_IMAGE: u64 = 12 << 20;
const MAX_SVG: u64 = 512 << 10;

/// Ikona zástupce místo automatické (logo projektu / favicon / ikona souboru).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum LookIcon {
    /// Emoji (nebo krátký text, nejvýš 8 znaků).
    Emoji(String),
    /// Systémová ikona Windows (Segoe Fluent Icons): kód znaku hex `E80F` (rozsah E000–F8FF).
    Glyph(String),
    /// Vlastní obrázek: klíč souboru v cache `looks` (16 hex znaků) — vznikne výběrem v šuplíku.
    Image(String),
}

/// Tvar ikony zástupce. Chybí = zaoblený čtverec (dnešní).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TileShape {
    #[default]
    Rounded,
    Circle,
    Square,
    /// průsvitné „sklo“ v barvě podkladu (bez rozmazání — software rendering WebView2)
    Glass,
}

/// Vzhled jednoho zástupce (projekt nebo akce). Všechna pole volitelná; prázdné = dnešní dlaždice.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TileLook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<LookIcon>,
    /// Barva podkladu ikony `#rrggbb` (předvolby `TILE_COLORS` nebo vlastní). Chybí = barva projektu / akce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<TileShape>,
    /// Vlastní popisek pod ikonou (nejvýš 40 znaků) místo názvu projektu / akce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Bez popisku pod ikonou (název zůstane v nápovědě a pro čtečky).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hide_label: bool,
}

/// Pozadí karty. Chybí = podle design stylu (dnešní).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CardBg {
    /// jemný tón akcentu karty přes pozadí stylu
    Tint,
    /// plné, neprůhledné
    Solid,
    /// průhledné (jen obsah)
    Clear,
}

/// Hustota karty. Chybí = podle stylu.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CardDensity {
    Compact,
    Airy,
}

/// Varianta zobrazení obsahu (karty Projekty, Repa, Poznámky, Rychlé akce). Chybí = výchozí karty.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CardView {
    List,
    Tiles,
    /// velká dlaždice + menší kolem (kolo 18; kreslí ji Projekty, Repa a Poznámky)
    Bento,
    Compact,
}

/// Plocha karty 2026 (kolo 22). Chybí = podle design stylu (dnešní). Tokeny `--k-*` dává materiál stylu.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CardSurface {
    /// sklo jen na rámu a hlavičce, pod textem plná stabilní plocha (čitelnost)
    Glass,
    /// plná vrstva s hloubkou (základ + kontrast)
    Layer,
}

/// Vzhled jedné karty na záložce (`layout.pages[].cards[].look`). Prázdné = podle design stylu.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct CardLook {
    /// Akcent karty: `accent` (akcent vzhledu), `blue`, `green`, `amber`, `red`, `violet`, `teal` (barvy
    /// motivu) nebo vlastní `#rrggbb`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<CardBg>,
    /// Bez hlavičky (název, počet); ⋯ zůstane vidět při najetí.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hide_head: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<CardDensity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<CardView>,
    /// Plocha karty: `glass` (sklo na rámu, plná plocha pod textem) / `layer` (plná vrstva). Chybí = podle stylu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<CardSurface>,
    /// Jemné zrnění plochy (~3 %, statický vzor bez obrázků zvenku).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub grain: bool,
    /// „Číslo jako hrdina“: velké hlavní číslo karty s popiskem a trendem (jen karty s hlavním číslem —
    /// `claude`, `weather`; u jiných se ignoruje). Bez skutečného čísla karta zůstane jako dřív.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hero: bool,
}

/// Složka zástupců v kartě Projekty (přetažení dlaždice na jinou, jako na telefonu).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TileFolder {
    /// Id složky (`a-z`, `0-9`, `-`; nejvýš 24 znaků).
    pub id: String,
    /// Název složky (nejvýš 32 znaků; prázdný = „Složka“).
    pub name: String,
    /// Id projektů ve složce v pořadí. Neexistující id se ignorují; prázdná složka zmizí.
    pub items: Vec<String>,
}

fn id_ok(id: &str) -> bool {
    (1..=24).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Klíč obrázku v cache: 16 malých hex znaků (hash obsahu).
pub fn key_ok(k: &str) -> bool {
    k.len() == 16 && k.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// Kód systémové ikony: 4 hex znaky v soukromém rozsahu E000–F8FF (Segoe Fluent Icons / MDL2).
pub fn glyph_ok(g: &str) -> bool {
    g.len() == 4 && u32::from_str_radix(g, 16).is_ok_and(|c| (0xE000..=0xF8FF).contains(&c))
}

fn text_ok(s: &str, max: usize) -> bool {
    s.chars().count() <= max && !s.chars().any(char::is_control)
}

/// Kresba na podkladu `hex` má být černá (`Some(true)`) nebo bílá (`Some(false)`) — co má větší kontrast
/// (vždy ≥ 4.5:1). Zrcadlo `inkDark` v TS (kreslí UI) — tady pro test kontrastu. Neplatná barva = `None`.
#[cfg(test)]
pub fn ink_dark(hex: &str) -> Option<bool> {
    use crate::appearance::geom::{contrast, parse_hex};
    use crate::strip::Rgba;
    let c = parse_hex(hex)?;
    Some(contrast(c, Rgba(0.0, 0.0, 0.0, 1.0)) > contrast(c, Rgba(1.0, 1.0, 1.0, 1.0)))
}

/// Validace vzhledu zástupce; `at` = cesta k poli (`projects.0.look`).
pub fn validate_tile(l: &TileLook, at: &str) -> Result<(), FieldError> {
    match &l.icon {
        Some(LookIcon::Emoji(e)) if e.trim().is_empty() || !text_ok(e, MAX_EMOJI) => {
            return Err(fe(format!("{at}.icon"), tr("Emoji ikony je jeden znak nebo emoji (nejvýš 8 znaků).", "An icon emoji is one character or emoji (at most 8 characters).")));
        }
        Some(LookIcon::Glyph(g)) if !glyph_ok(g) => {
            return Err(fe(format!("{at}.icon"), trf!("Systémová ikona „{g}“: kód E000–F8FF (4 hex znaky).", "System icon “{g}”: code E000–F8FF (4 hex characters).")));
        }
        Some(LookIcon::Image(k)) if !key_ok(k) => {
            return Err(fe(format!("{at}.icon"), tr("Vlastní obrázek vyber v šuplíku (Upravit vzhled…) — klíč je 16 hex znaků.", "Pick a custom image in the drawer (Edit look…) — the key is 16 hex characters.")));
        }
        _ => {}
    }
    if let Some(c) = &l.color
        && !crate::appearance::is_hex(c)
    {
        return Err(fe(format!("{at}.color"), tr("Barva podkladu musí být #rrggbb.", "The background color must be #rrggbb.")));
    }
    if let Some(s) = &l.label
        && !text_ok(s, MAX_LABEL)
    {
        return Err(fe(format!("{at}.label"), trf!("Vlastní popisek má nejvýš {MAX_LABEL} znaků.", "A custom label has at most {MAX_LABEL} characters.")));
    }
    Ok(())
}

/// Validace vzhledu karty; `at` = `layout.pages.0.cards.1.look`.
pub fn validate_card(l: &CardLook, at: &str) -> Result<(), FieldError> {
    if let Some(a) = &l.accent
        && !CARD_ACCENTS.contains(&a.as_str())
        && !crate::appearance::is_hex(a)
    {
        return Err(fe(format!("{at}.accent"), trf!("Akcent karty: {} nebo #rrggbb.", "Card accent: {} or #rrggbb.", CARD_ACCENTS.join(", "))));
    }
    Ok(())
}

/// Validace složek karty Projekty; `at` = `layout.pages.0.cards.0.tiles.folders`.
pub fn validate_folders(f: &[TileFolder], at: &str) -> Result<(), FieldError> {
    if f.len() > MAX_FOLDERS {
        return Err(fe(at, trf!("Nejvýš {MAX_FOLDERS} složek na kartě.", "At most {MAX_FOLDERS} folders on a card.")));
    }
    let mut ids = std::collections::HashSet::new();
    let mut inside = std::collections::HashSet::new();
    for (k, x) in f.iter().enumerate() {
        if !id_ok(&x.id) || !ids.insert(x.id.as_str()) {
            return Err(fe(format!("{at}.{k}.id"), trf!("Složka „{}“: id jen a-z, 0-9 a pomlčka (nejvýš 24), každé jednou.", "Folder “{}”: id only a-z, 0-9 and dash (at most 24), each once.", x.id)));
        }
        if !text_ok(&x.name, MAX_FOLDER_NAME) {
            return Err(fe(format!("{at}.{k}.name"), trf!("Název složky má nejvýš {MAX_FOLDER_NAME} znaků.", "A folder name has at most {MAX_FOLDER_NAME} characters.")));
        }
        if x.items.len() > MAX_FOLDER_ITEMS {
            return Err(fe(format!("{at}.{k}.items"), trf!("Ve složce je nejvýš {MAX_FOLDER_ITEMS} projektů.", "A folder holds at most {MAX_FOLDER_ITEMS} projects.")));
        }
        for it in &x.items {
            if !inside.insert(it.as_str()) {
                return Err(fe(format!("{at}.{k}.items"), trf!("Projekt „{it}“ je ve dvou složkách (nebo dvakrát).", "Project “{it}” is in two folders (or twice).")));
            }
        }
    }
    Ok(())
}

/// Celá validace vzhledů v configu (volá `config::validate_field`).
pub fn validate(cfg: &Config) -> Result<(), FieldError> {
    for (i, p) in cfg.projects.iter().enumerate() {
        if let Some(l) = &p.look {
            validate_tile(l, &format!("projects.{i}.look"))?;
        }
    }
    for (i, a) in cfg.actions.iter().enumerate() {
        if let Some(l) = &a.look {
            validate_tile(l, &format!("actions.{i}.look"))?;
        }
    }
    for (i, pg) in cfg.layout.pages.iter().enumerate() {
        for (j, c) in pg.cards.iter().enumerate() {
            if let Some(l) = &c.look {
                validate_card(l, &format!("layout.pages.{i}.cards.{j}.look"))?;
            }
            if let Some(t) = &c.tiles {
                validate_folders(&t.folders, &format!("layout.pages.{i}.cards.{j}.tiles.folders"))?;
            }
        }
    }
    Ok(())
}

// ---------- vlastní obrázky (cache) ----------

/// `%LOCALAPPDATA%\Ledge\looks` (u přepsaného configu vedle něj — testy).
pub fn image_dir() -> PathBuf {
    crate::actionrun::local_dir().join("looks")
}

/// Vybraný soubor → zmenšený obrázek v cache `dir` → klíč (hash obsahu). Rastry přes WIC (PNG ≤ 128 px),
/// SVG beze změny (≤ 512 KB). Čte disk a COM — volat mimo UI vlákno.
pub fn import_image(dir: &Path, src: &Path) -> Result<String, String> {
    let bad = || tr("Tenhle soubor nejde použít jako obrázek (png, jpg, ico, webp, bmp, gif nebo svg).", "This file can't be used as an image (png, jpg, ico, webp, bmp, gif or svg).").to_string();
    let meta = std::fs::metadata(src).map_err(|e| crate::errors::io(tr("Soubor nejde přečíst", "Can't read the file"), &e))?;
    let svg = src.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if !meta.is_file() || meta.len() > if svg { MAX_SVG } else { MAX_IMAGE } {
        return Err(tr("Obrázek je moc velký (fotka nejvýš 12 MB, SVG 512 KB).", "The image is too large (photo at most 12 MB, SVG 512 KB).").into());
    }
    let bytes = std::fs::read(src).map_err(|e| crate::errors::io(tr("Soubor nejde přečíst", "Can't read the file"), &e))?;
    let (ext, out) = if svg {
        let text = std::str::from_utf8(&bytes).map_err(|_| bad())?;
        if !text.contains("<svg") {
            return Err(bad());
        }
        ("svg", bytes)
    } else {
        ("png", crate::iconimg::to_png(&bytes).ok_or_else(bad)?)
    };
    let key = format!("{:016x}", crate::icons::fnv1a(&out));
    std::fs::create_dir_all(dir).map_err(|e| crate::errors::io(tr("Obrázek nejde uložit", "Can't save the image"), &e))?;
    let path = dir.join(format!("{key}.{ext}"));
    if !path.exists() {
        std::fs::write(&path, &out).map_err(|e| crate::errors::io(tr("Obrázek nejde uložit", "Can't save the image"), &e))?;
    }
    Ok(key)
}

/// Data URL obrázku z cache; neznámý / chybějící klíč = `None` (UI ukáže Auto).
pub fn image_url(dir: &Path, key: &str) -> Option<String> {
    if !key_ok(key) {
        return None;
    }
    for (ext, mime) in [("png", "image/png"), ("svg", "image/svg+xml")] {
        if let Ok(b) = std::fs::read(dir.join(format!("{key}.{ext}"))) {
            return Some(format!("data:{mime};base64,{}", crate::icons::b64(&b)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn empty_look_writes_nothing() {
        assert_eq!(serde_json::to_value(TileLook::default()).unwrap(), json!({}));
        assert_eq!(serde_json::to_value(CardLook::default()).unwrap(), json!({}));
        let full = json!({"icon":{"kind":"glyph","value":"E80F"},"color":"#2563eb","shape":"circle","label":"Domů","hideLabel":true});
        let l: TileLook = serde_json::from_value(full.clone()).unwrap();
        assert_eq!(l.icon, Some(LookIcon::Glyph("E80F".into())));
        assert_eq!(serde_json::to_value(&l).unwrap(), full, "tam a zpět");
        let c: CardLook = serde_json::from_value(json!({"accent":"teal","bg":"tint","hideHead":true,"density":"airy","view":"list"})).unwrap();
        assert_eq!((c.bg, c.density, c.view, c.hide_head), (Some(CardBg::Tint), Some(CardDensity::Airy), Some(CardView::List), true));
        assert!(serde_json::from_value::<TileLook>(json!({"shape":"star"})).is_err());
        // kolo 22: plocha, zrnění, hrdina — tam a zpět, nepravda se nepíše
        let k = json!({"surface":"glass","grain":true,"hero":true});
        let c: CardLook = serde_json::from_value(k.clone()).unwrap();
        assert_eq!((c.surface, c.grain, c.hero), (Some(CardSurface::Glass), true, true));
        assert_eq!(serde_json::to_value(&c).unwrap(), k);
        let off: CardLook = serde_json::from_value(json!({"grain":false,"hero":false,"surface":"layer"})).unwrap();
        assert_eq!(serde_json::to_value(&off).unwrap(), json!({"surface":"layer"}));
        assert!(serde_json::from_value::<CardLook>(json!({"surface":"metal"})).is_err());
    }

    #[test]
    fn tile_validation_table() {
        let ok = |v: Value| validate_tile(&serde_json::from_value(v).unwrap(), "projects.0.look").err().and_then(|e| e.field);
        let cases: Vec<(Value, Option<&str>)> = vec![
            (json!({}), None),
            (json!({"icon":{"kind":"emoji","value":"🎮"}}), None),
            (json!({"icon":{"kind":"emoji","value":"👨‍👩‍👧"}}), None),
            (json!({"icon":{"kind":"emoji","value":" "}}), Some("projects.0.look.icon")),
            (json!({"icon":{"kind":"emoji","value":"dlouhý text"}}), Some("projects.0.look.icon")),
            (json!({"icon":{"kind":"glyph","value":"E80F"}}), None),
            (json!({"icon":{"kind":"glyph","value":"0041"}}), Some("projects.0.look.icon")),
            (json!({"icon":{"kind":"glyph","value":"E80"}}), Some("projects.0.look.icon")),
            (json!({"icon":{"kind":"image","value":"0123456789abcdef"}}), None),
            (json!({"icon":{"kind":"image","value":"C:\\x.png"}}), Some("projects.0.look.icon")),
            (json!({"color":"#0d9488"}), None),
            (json!({"color":"teal"}), Some("projects.0.look.color")),
            (json!({"label":"x".repeat(40)}), None),
            (json!({"label":"x".repeat(41)}), Some("projects.0.look.label")),
            (json!({"label":"a\nb"}), Some("projects.0.look.label")),
        ];
        for (v, want) in cases {
            assert_eq!(ok(v.clone()).as_deref(), want, "{v}");
        }
    }

    #[test]
    fn card_and_folder_validation() {
        let card = |a: &str| validate_card(&CardLook { accent: Some(a.into()), ..Default::default() }, "x").is_ok();
        assert!(card("teal") && card("accent") && card("#123abc"));
        assert!(!card("pink") && !card("#12"));
        let f = |id: &str, items: &[&str]| TileFolder { id: id.into(), name: "Hry".into(), items: items.iter().map(|s| s.to_string()).collect() };
        assert!(validate_folders(&[f("slozka", &["a", "b"]), f("slozka-2", &["c"])], "f").is_ok());
        assert!(validate_folders(&[f("slozka", &[])], "f").is_ok(), "prázdnou ignoruje UI, načtení nesmí spadnout");
        assert_eq!(validate_folders(&[f("Slozka", &["a"])], "f").unwrap_err().field.as_deref(), Some("f.0.id"));
        assert_eq!(validate_folders(&[f("a", &["x"]), f("a", &["y"])], "f").unwrap_err().field.as_deref(), Some("f.1.id"));
        assert_eq!(validate_folders(&[f("a", &["x"]), f("b", &["x"])], "f").unwrap_err().field.as_deref(), Some("f.1.items"));
    }

    #[test]
    fn ink_is_readable_on_every_color() {
        use crate::appearance::geom::{contrast, parse_hex};
        use crate::strip::Rgba;
        let dark = Rgba(0.0, 0.0, 0.0, 1.0);
        let white = Rgba(1.0, 1.0, 1.0, 1.0);
        let check = |hex: &str| {
            let c = parse_hex(hex).unwrap();
            let ink = if ink_dark(hex).unwrap() { dark } else { white };
            assert!(contrast(c, ink) >= 4.5, "{hex}: {}", contrast(c, ink));
        };
        TILE_COLORS.iter().for_each(|h| check(h));
        // i libovolná vlastní barva (mřížka přes celé RGB)
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(17) {
                for b in (0..=255).step_by(17) {
                    check(&format!("#{r:02x}{g:02x}{b:02x}"));
                }
            }
        }
        assert_eq!(ink_dark("#f59e0b"), Some(true));
        assert_eq!(ink_dark("#1f2937"), Some(false));
        assert_eq!(ink_dark("modrá"), None);
    }

    #[test]
    fn config_without_looks_loads_and_saves_unchanged() {
        // skutečný tvar configu vlny 7 (projekt, akce, rozložení s tiles) — bez `look` / `folders`
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let old = r##"{"projects":[{"id":"harbor","name":"Harbor","kind":"dev","path":"C:\\x","run":"npm run dev","accent":"#2563eb"}],
          "actions":[{"id":"mapy","label":"Mapy","kind":"url","target":"https://mapy.cz"}],
          "layout":{"pages":[{"id":"prace","name":"","cards":[{"id":"projects","size":"xl","tiles":{"size":"small","label":true,"git":false,"hidden":[]}},{"id":"claude","size":"m"}]}],"hiddenParts":[],"upgrades":["personalProjects"]}}"##;
        std::fs::write(&path, old).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert!(cfg.projects[0].look.is_none() && cfg.actions[0].look.is_none());
        assert!(cfg.layout.pages[0].cards.iter().all(|c| c.look.is_none()));
        crate::config::save(&path, &cfg).unwrap();
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(saved["projects"][0].get("look").is_none() && saved["actions"][0].get("look").is_none(), "vzhled se do starého configu nepíše");
        let cards = saved["layout"]["pages"][0]["cards"].as_array().unwrap();
        assert!(cards.iter().all(|c| c.get("look").is_none()));
        assert_eq!(cards[0]["tiles"], json!({"size":"small","label":true,"git":false,"hidden":[]}), "bez složek se `folders` nepíše");
        let again = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(again.projects, cfg.projects);
        assert_eq!(again.layout, cfg.layout);

        // se vzhledy: load → save → load beze ztráty
        let mut cfg = again;
        cfg.projects[0].look = Some(TileLook { icon: Some(LookIcon::Emoji("⚓".into())), shape: Some(TileShape::Circle), ..Default::default() });
        cfg.actions[0].look = Some(TileLook { color: Some("#16a34a".into()), hide_label: true, ..Default::default() });
        cfg.layout.pages[0].cards[1].look = Some(CardLook { accent: Some("teal".into()), bg: Some(CardBg::Tint), view: Some(CardView::Compact), ..Default::default() });
        cfg.layout.pages[0].cards[0].tiles.as_mut().unwrap().folders = vec![TileFolder { id: "slozka".into(), name: "Weby".into(), items: vec!["harbor".into()] }];
        crate::config::save(&path, &cfg).unwrap();
        let back = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(back.projects[0].look, cfg.projects[0].look);
        assert_eq!(back.actions[0].look, cfg.actions[0].look);
        assert_eq!(back.layout, cfg.layout);
        // chybná hodnota = config se neuloží (cesta k poli pro Nastavení)
        cfg.projects[0].look = Some(TileLook { color: Some("red".into()), ..Default::default() });
        assert_eq!(crate::config::validate_field(&cfg).unwrap_err().field.as_deref(), Some("projects.0.look.color"));
    }

    #[test]
    fn schema_mentions_looks() {
        let schema = crate::config::schema_json();
        for needle in ["\"look\"", "\"folders\"", "\"hideLabel\"", "\"glass\"", "\"tint\"", "\"surface\"", "\"grain\"", "\"hero\""] {
            assert!(schema.contains(needle), "{needle} ve schématu");
        }
    }

    #[test]
    fn image_cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("looks");
        let png = dir.path().join("a.png");
        std::fs::write(&png, crate::iconimg::png_from_bgra(300, 200, &[10, 20, 30, 255].repeat(300 * 200))).unwrap();
        let key = import_image(&cache, &png).unwrap();
        assert!(key_ok(&key), "{key}");
        assert_eq!(import_image(&cache, &png).unwrap(), key, "stejný obsah = stejný klíč");
        let url = image_url(&cache, &key).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
        // originál smazaný — obrázek zůstává v cache
        std::fs::remove_file(&png).unwrap();
        assert!(image_url(&cache, &key).is_some());
        let svg = dir.path().join("i.svg");
        std::fs::write(&svg, "<svg xmlns='http://www.w3.org/2000/svg'/>").unwrap();
        let k2 = import_image(&cache, &svg).unwrap();
        assert!(image_url(&cache, &k2).unwrap().starts_with("data:image/svg+xml;base64,"));
        let txt = dir.path().join("x.txt");
        std::fs::write(&txt, "ahoj").unwrap();
        assert!(import_image(&cache, &txt).is_err());
        assert!(import_image(&cache, &dir.path().join("chybi.png")).is_err());
        assert_eq!(image_url(&cache, "0000000000000000"), None);
        assert_eq!(image_url(&cache, "../config"), None, "klíč nikdy není cesta");
    }
}
