//! Velikost otevřeného šuplíku podle uživatele (kolo 23, `appearance.size`): šířka a výška tažením za
//! okraje v režimu úprav (`src/layout/drawerSize.ts`) nebo čísly v Nastavení → Vzhled → Otevřený,
//! volitelně zvlášť pro každou záložku (`perPage`, výchozí vypnuto). Nic nenastaveno (= starý config) →
//! šířka z předvolby / `drawer.width`, výška podle obsahu — dnešní chování beze změny.
//! Čisté funkce + tabulkové testy; stav je jen id otevřené záložky (`set_page`, volá `set_mode`) a
//! šířka během tažení (`set_live`, bez zápisu configu). Používá `geom::drawer_width_for`,
//! `commands::drawer::drawer_size`, `threads::ui` (`UiCmd::Resize`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Mutex, RwLock};

/// Rozsah výšky (logické px): 1–5 řad karet. Šířka má rozsah `DRAWER_WIDTH`.
pub const HEIGHT: (u32, u32) = (270, 940);

/// Šířka a výška jedné záložky / celku v logických px; 0 = výchozí (šířka z předvolby, výška podle obsahu).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct PageSize {
    #[serde(skip_serializing_if = "is_zero")]
    pub width: u32,
    #[serde(skip_serializing_if = "is_zero")]
    pub height: u32,
}

/// `appearance.size`: velikost otevřeného šuplíku.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct SizeConfig {
    /// Šířka v px (720–2400, nejvýš 95 % monitoru); 0 = podle předvolby šířky / `drawer.width` (výchozí).
    #[serde(skip_serializing_if = "is_zero")]
    pub width: u32,
    /// Výška v px (270–940, nejvýš strop šuplíku); 0 = podle obsahu (výchozí). Obsah navíc se posouvá.
    #[serde(skip_serializing_if = "is_zero")]
    pub height: u32,
    /// Pamatovat velikost pro každou záložku (výchozí vypnuto = jedna velikost pro všechny).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub per_page: bool,
    /// Velikost záložek (klíč = id z `layout.pages`), platí jen s `perPage`; chybí / 0 = jako celek.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub pages: BTreeMap<String, PageSize>,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl SizeConfig {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Velikost pro záložku: s `perPage` její hodnoty, chybějící (0) doplní celek.
    pub fn for_page(&self, page: &str) -> PageSize {
        let own = if self.per_page { self.pages.get(page).copied().unwrap_or_default() } else { PageSize::default() };
        // ruční zásah do configu (`"height": 5`) se ořízne stejně jako ve frontendu (`normalizeSize`)
        clamp(PageSize {
            width: if own.width > 0 { own.width } else { self.width },
            height: if own.height > 0 { own.height } else { self.height },
        })
    }

    /// Zapsat velikost (už oříznutou `clamp`): s `perPage` do záložky, jinak do celku.
    pub fn set(&mut self, page: &str, s: PageSize) {
        if self.per_page {
            if s == PageSize::default() {
                self.pages.remove(page);
            } else {
                self.pages.insert(page.to_string(), s);
            }
        } else {
            self.width = s.width;
            self.height = s.height;
        }
    }
}

/// Přepnutí záložky UVNITŘ úprav (tažení / náhled velikosti): animovaně, jen s `perPage`, když se záložka
/// opravdu změnila a její velikost je jiná. `set_mode` dělá totéž pro skutečné přepnutí.
pub fn switch_animates(cfg: &SizeConfig, from: &str, to: &str) -> bool {
    cfg.per_page && from != to && cfg.for_page(from) != cfg.for_page(to)
}

/// Ořez na rozsahy (`DRAWER_WIDTH`, `HEIGHT`); 0 zůstává 0 (= výchozí).
pub fn clamp(s: PageSize) -> PageSize {
    let (wl, wh) = super::DRAWER_WIDTH;
    let c = |v: u32, lo: u32, hi: u32| if v == 0 { 0 } else { v.clamp(lo, hi) };
    PageSize { width: c(s.width, wl, wh), height: c(s.height, HEIGHT.0, HEIGHT.1) }
}

/// Pevná výška šuplíku v px po stropu monitoru (`max_h` z `drawerfit::limits`); `None` = podle obsahu.
/// Zaokrouhlení jako frontend (`--drawer-max-h` = `floor(maxH)`), ať se po přepnutí záložky nic nedorovnává.
pub fn fixed_height(s: PageSize, max_h: f64) -> Option<f64> {
    (s.height > 0).then(|| (s.height as f64).min(max_h.floor()))
}

/// Krok animace změny velikosti okna `[x, šířka, výška]` (fyzické px) v čase `t` 0–1, ease-out.
pub fn step(from: [i32; 3], to: [i32; 3], t: f64) -> [i32; 3] {
    let e = 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3);
    std::array::from_fn(|i| from[i] + ((to[i] - from[i]) as f64 * e).round() as i32)
}

static PAGE: RwLock<String> = RwLock::new(String::new());
static LIVE: Mutex<Option<u32>> = Mutex::new(None);

/// Otevřená záložka (zrcadlo `core().mode`; prázdné = výchozí `prace`).
pub fn set_page(id: &str) {
    if let Ok(mut p) = PAGE.write() {
        id.clone_into(&mut p);
    }
}

pub fn page() -> String {
    let p = PAGE.read().map(|p| p.clone()).unwrap_or_default();
    if p.is_empty() { crate::layout::WORK.to_string() } else { p }
}

/// Šířka během tažení (bez zápisu configu); `None` = konec tažení.
pub fn set_live(w: Option<u32>) {
    if let Ok(mut l) = LIVE.lock() {
        *l = w;
    }
}

pub fn live() -> Option<u32> {
    LIVE.lock().ok().and_then(|l| *l)
}

/// Živá šířka po `drawer_size(done = false)`: jen při skutečném tažení (`ResizeLive`, `animates = false`);
/// přepnutí záložky (animované) ji zahodí, ať nezůstane viset a nepřebije uloženou velikost záložky.
pub fn live_after(animates: bool, want_width: u32) -> Option<u32> {
    (!animates).then_some(want_width).filter(|w| *w > 0)
}

/// Šířka v px, kterou velikost přebíjí předvolbu (`None` = předvolba / `drawer.width`): tažení má přednost.
pub fn width_override(s: &SizeConfig, page: &str, live: Option<u32>) -> Option<u32> {
    live.or_else(|| Some(s.for_page(page).width).filter(|w| *w > 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ps(width: u32, height: u32) -> PageSize {
        PageSize { width, height }
    }

    #[test]
    fn old_config_has_default_size() {
        let s: SizeConfig = serde_json::from_str("{}").unwrap();
        assert!(s.is_default());
        assert_eq!(s.for_page("prace"), ps(0, 0), "nic = šířka z předvolby, výška podle obsahu");
        assert_eq!(serde_json::to_string(&s).unwrap(), "{}", "výchozí se nezapisuje");
        assert_eq!(width_override(&s, "prace", None), None);
    }

    /// Config před kolem 23 (bez `appearance.size`): stejná šířka jako dřív, výška podle obsahu (žádná pevná),
    /// uložení pole nepřidá — dokud uživatel netáhne, nic se nemění.
    #[test]
    fn config_before_round_23_keeps_size() {
        let old = r#"{ "drawer": { "width": 1300 }, "appearance": { "style": "classic", "width": "wide", "layout": "classic" },
            "layout": { "pages": [ { "id": "prace", "name": "", "cards": [] }, { "id": "app", "name": "App", "cards": [] } ] } }"#;
        let cfg: crate::config::Config = serde_json::from_str(old).unwrap();
        assert!(cfg.appearance.size.is_default());
        for page in ["prace", "app"] {
            let s = cfg.appearance.size.for_page(page);
            assert_eq!(fixed_height(s, 800.0), None, "{page}: výška dál podle obsahu");
            assert_eq!(width_override(&cfg.appearance.size, page, None), None);
        }
        assert_eq!(super::super::geom::drawer_width_for(&cfg, 1920, 96), 1440.0, "šířka dál z předvolby");
        let saved = serde_json::to_value(&cfg).unwrap();
        assert!(saved["appearance"].get("size").is_none(), "uložení pole size nepřidá");
    }

    #[test]
    fn for_page_table() {
        let mut pages = BTreeMap::new();
        pages.insert("prace".to_string(), ps(1440, 0));
        pages.insert("osobni".to_string(), ps(0, 600));
        let off = SizeConfig { width: 1200, height: 500, per_page: false, pages: pages.clone() };
        let on = SizeConfig { per_page: true, ..off.clone() };
        // (config, záložka, čekáno)
        let cases = [
            (&off, "prace", ps(1200, 500)), // bez volby záložky nic neznamenají
            (&on, "prace", ps(1440, 500)),  // vlastní šířka, výška z celku
            (&on, "osobni", ps(1200, 600)),
            (&on, "app", ps(1200, 500)), // záložka bez záznamu = celek
        ];
        for (s, page, want) in cases {
            assert_eq!(s.for_page(page), want, "{page} per_page={}", s.per_page);
        }
    }

    #[test]
    fn live_after_table() {
        // (animovaně = přepnutí záložky, šířka, čekáno)
        for (anim, w, want) in [(false, 1400, Some(1400)), (false, 0, None), (true, 1400, None), (true, 0, None)] {
            assert_eq!(live_after(anim, w), want, "anim={anim} w={w}");
        }
    }

    #[test]
    fn for_page_clamps_hand_edited() {
        let s = SizeConfig { width: 100, height: 5, ..Default::default() };
        assert_eq!(s.for_page("prace"), ps(super::super::DRAWER_WIDTH.0, HEIGHT.0));
        let s = SizeConfig { width: 99999, height: 99999, ..Default::default() };
        assert_eq!(s.for_page("prace"), ps(super::super::DRAWER_WIDTH.1, HEIGHT.1));
        let mut pages = BTreeMap::new();
        pages.insert("a".to_string(), ps(0, 7));
        let s = SizeConfig { per_page: true, pages, ..Default::default() };
        assert_eq!(s.for_page("a"), ps(0, HEIGHT.0));
        assert_eq!(fixed_height(s.for_page("a"), 2000.0), Some(f64::from(HEIGHT.0)));
    }

    #[test]
    fn switch_table() {
        let mut pages = BTreeMap::new();
        pages.insert("prace".to_string(), ps(1440, 0));
        pages.insert("smazana".to_string(), ps(0, 600));
        let s = SizeConfig { per_page: true, pages, ..Default::default() };
        assert!(switch_animates(&s, "prace", "smazana"));
        assert!(!switch_animates(&s, "prace", "prace"), "stejná záložka");
        assert!(!switch_animates(&s, "x", "y"), "obě výchozí");
        assert!(!switch_animates(&SizeConfig { per_page: false, ..s.clone() }, "prace", "smazana"), "bez perPage");
    }

    #[test]
    fn set_and_clamp_table() {
        // (vstup, ořez)
        let cases = [(ps(0, 0), ps(0, 0)), (ps(500, 100), ps(720, 270)), (ps(3000, 2000), ps(2400, 940)), (ps(1080, 600), ps(1080, 600))];
        for (i, want) in cases {
            assert_eq!(clamp(i), want, "{i:?}");
        }
        let mut s = SizeConfig::default();
        s.set("prace", ps(1300, 0));
        assert_eq!((s.width, s.height, s.pages.len()), (1300, 0, 0), "bez volby do celku");
        s.per_page = true;
        s.set("osobni", ps(900, 436));
        assert_eq!(s.for_page("osobni"), ps(900, 436));
        assert_eq!(s.for_page("prace"), ps(1300, 0));
        s.set("osobni", ps(0, 0));
        assert!(s.pages.is_empty(), "výchozí záložky = záznam pryč");
    }

    #[test]
    fn fixed_height_and_live() {
        assert_eq!(fixed_height(ps(0, 0), 800.0), None);
        assert_eq!(fixed_height(ps(0, 600), 800.0), Some(600.0));
        assert_eq!(fixed_height(ps(0, 940), 825.6), Some(825.0), "strop monitoru jako frontend (floor)");
        let s = SizeConfig { width: 1200, ..Default::default() };
        assert_eq!(width_override(&s, "prace", None), Some(1200));
        assert_eq!(width_override(&s, "prace", Some(900)), Some(900), "tažení má přednost");
    }

    #[test]
    fn step_eases_to_target() {
        let (a, b) = ([100, 1000, 500], [40, 1120, 700]);
        assert_eq!(step(a, b, 0.0), a);
        assert_eq!(step(a, b, 1.0), b);
        let mid = step(a, b, 0.5);
        assert!(mid[1] > 1060 && mid[1] < 1120, "ease-out: v půlce víc než půl cesty {mid:?}");
    }
}
