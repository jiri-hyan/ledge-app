//! Rozložení šuplíku (`config.layout`): záložky (vestavěné Práce / Osobní + vlastní), na každé karty
//! v pořadí a s velikostí `s|m|l|xl`, a skryté části karet (limity a útrata v kartě Claude, hlasitost…).
//!
//! **Jeden model.** Co je v šuplíku, kde a jak velké, je JEN tady. Karta, která není na žádné záložce,
//! je „skrytá“ (katalog v šuplíku ji nabídne zpět). `drawer.sections` a `modes.personalEnabled` jsou od
//! téhle verze ODVOZENÉ ([`derive`] při načtení, uložení i aplikaci configu): dál je čtou moduly, které
//! podle nich zapínají stahování a odběry (počasí, kalendář, útrata, média, hlasitost, baterie), a starší
//! Ledge po downgradu. Ruční změna `drawer.sections` se přepíše — měň `layout`.
//!
//! Starý config bez `layout` se převede beze ztráty ([`from_legacy`]): viditelnost z `drawer.sections`,
//! pořadí karet Práce z `appearance.cards`, záložka Osobní jen při `modes.personalEnabled != false`,
//! dnešní pevné pořadí Osobní. Velikosti dřív nebyly — řada se rozdělí jako dnešní mřížka ([`row_sizes`]).
//!
//! Karta má šířku `size` a volitelně výšku `h` (1–4 řádky, chybí = podle obsahu; obsah pak scrolluje
//! uvnitř karty). Karta Projekty (zástupci) má navíc volby dlaždic `tiles` (velikost, popisek, git,
//! skryté projekty na téhle záložce). Jednorázové úpravy rozložení po aktualizaci ([`apply_upgrades`])
//! se značí v `upgrades` — co uživatel potom odebere, už se nevrací.

use crate::config::Config;
use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Vestavěné záložky (název podle jazyka, když `name` je prázdné).
pub const WORK: &str = "prace";
pub const PERSONAL: &str = "osobni";
/// Karty, které jde dát na záložku (zrcadlo `CARD_IDS` v `src/layout/model.ts`).
pub const CARDS: [&str; 13] = ["projects", "running", "claude", "repos", "notes", "actions", "shelf", "clock", "timer", "weather", "calendar", "links", "media"];
/// Části karet, které jdou skrýt zvlášť: (klíč, karta). Klíče = pole `drawer.sections`.
pub const PARTS: [(&str, &str); 7] = [
    ("claudeLimits", "claude"),
    ("claudeCosts", "claude"),
    ("claudeGit", "claude"),
    ("claudeSubagents", "claude"),
    ("nowPlaying", "media"),
    ("sound", "media"),
    ("battery", "clock"),
];
/// Nejvýš záložek (klávesy 1–9).
pub const MAX_PAGES: usize = 9;
const MAX_NAME: usize = 32;
const MAX_ICON: usize = 4;
/// Nejvyšší výška karty v řádcích (`h`).
pub const MAX_H: u8 = 4;
/// Jednorázová úprava: karta Projekty (zástupci) i na záložku Osobní (vlna 7).
pub const UP_PERSONAL_PROJECTS: &str = "personalProjects";

/// Šířka karty v mřížce `.deck` (`--deck-cols`, výchozí 4): `s` = 1 sloupec, `m` = 2, `l` = 3, `xl` = celá řada.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CardSize {
    S,
    #[default]
    M,
    L,
    Xl,
}

/// Velikost dlaždic v kartě Projekty: malé ikony, dlaždice (výchozí), velké.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TileSize {
    Small,
    #[default]
    Tile,
    Large,
}

/// Volby karty Projekty (zástupci) na jedné záložce.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TileOpts {
    /// `small` (malé ikony) / `tile` (dlaždice) / `large` (velké).
    pub size: TileSize,
    /// Název projektu pod ikonou.
    pub label: bool,
    /// Větev a stav gitu (neuloženo ●, nepushnuto ↑, pozadu ↓) pod názvem.
    pub git: bool,
    /// Id projektů, které na téhle záložce nejsou (prázdné = všechny; nový projekt se ukáže sám).
    pub hidden: Vec<String>,
}

impl Default for TileOpts {
    fn default() -> Self {
        Self { size: TileSize::Tile, label: true, git: true, hidden: Vec::new() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct LayoutCard {
    /// Id karty: `projects`, `running`, `claude`, `repos`, `notes`, `actions`, `shelf`, `clock`, `timer`,
    /// `weather`, `calendar`, `links`, `media`.
    pub id: String,
    /// `s` / `m` / `l` / `xl` (celá řada).
    pub size: CardSize,
    /// Výška v řádcích mřížky 1–4 (obsah scrolluje uvnitř karty); chybí = podle obsahu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u8>,
    /// Jen karta `projects`: velikost dlaždic, popisek, git, skryté projekty. Chybí = výchozí.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiles: Option<TileOpts>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct LayoutPage {
    /// Id záložky (`a-z`, `0-9`, `-`; nejvýš 24 znaků). `prace` a `osobni` jsou vestavěné.
    pub id: String,
    /// Název na záložce. U vestavěných smí být prázdný (= „Práce“ / „Osobní“ podle jazyka).
    pub name: String,
    /// Volitelná ikona (emoji) před názvem.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// Karty v pořadí zleva doprava, shora dolů.
    pub cards: Vec<LayoutCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct LayoutConfig {
    /// Záložky šuplíku v pořadí (1–9). Karta, která není na žádné, je skrytá a nic nestahuje.
    pub pages: Vec<LayoutPage>,
    /// Skryté části karet: `claudeLimits`, `claudeCosts`, `claudeGit`, `claudeSubagents` (karta Claude),
    /// `nowPlaying`, `sound` (karta Právě hraje), `battery` (karta Čas).
    pub hidden_parts: Vec<String>,
    /// Jednorázové úpravy rozložení, které už proběhly (`personalProjects`). Neměň ručně.
    /// Chybí = žádná (ne výchozí rozložení — to je má všechny).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upgrades: Vec<String>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        let mut l = from_legacy(&Value::Null);
        apply_upgrades(&mut l);
        l
    }
}

fn card(id: &str, size: CardSize) -> LayoutCard {
    LayoutCard { id: id.into(), size, ..Default::default() }
}

/// Velikosti karet jedné řady tak, aby ji vyplnily jako dřívější mřížka (4 sloupce): 1 karta = celá
/// řada, 2 = půlky, 3 = první širší, víc = po sloupci (zalomí se).
pub fn row_sizes(n: usize) -> Vec<CardSize> {
    match n {
        0 => vec![],
        1 => vec![CardSize::Xl],
        2 => vec![CardSize::M, CardSize::M],
        3 => vec![CardSize::M, CardSize::S, CardSize::S],
        _ => vec![CardSize::S; n],
    }
}

/// Karty jedné řady s velikostmi z [`row_sizes`].
fn row(ids: &[&str]) -> Vec<LayoutCard> {
    ids.iter().zip(row_sizes(ids.len())).map(|(id, s)| card(id, s)).collect()
}

/// Pořadí mřížky Práce ze starého `appearance.cards` (jako dřívější `cardOrder`): platná id bez
/// opakování, chybějící na konec ve výchozím pořadí.
fn legacy_work_order(v: Option<&Value>) -> Vec<&'static str> {
    const GRID: [&str; 4] = ["running", "claude", "repos", "notes"];
    let mut out: Vec<&'static str> = Vec::new();
    let listed = v.and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str);
    for id in listed.chain(GRID) {
        if let Some(k) = GRID.iter().find(|g| **g == id)
            && !out.contains(k)
        {
            out.push(k);
        }
    }
    out
}

/// Rozložení ze starého configu (syrový JSON; `Null` / `{}` = výchozí rozložení). Čistá funkce.
pub fn from_legacy(raw: &Value) -> LayoutConfig {
    let on = |k: &str| raw.pointer(&format!("/drawer/sections/{k}")).and_then(Value::as_bool) != Some(false);
    let personal = raw.pointer("/modes/personalEnabled").and_then(Value::as_bool) != Some(false);

    // Práce: projekty, akce, přihrádka přes celou šířku (dřív nad mřížkou), pak mřížka v pořadí `appearance.cards`
    let mut work: Vec<LayoutCard> = ["projects", "actions", "shelf"].into_iter().filter(|k| on(k)).map(|k| card(k, CardSize::Xl)).collect();
    let grid: Vec<&str> = legacy_work_order(raw.pointer("/appearance/cards")).into_iter().filter(|k| on(k)).collect();
    let narrow: Vec<&str> = grid.iter().copied().filter(|k| *k == "claude" || *k == "repos").collect();
    let mut sizes = row_sizes(narrow.len()).into_iter();
    for k in grid {
        let size = if k == "claude" || k == "repos" { sizes.next().unwrap_or_default() } else { CardSize::Xl };
        work.push(card(k, size));
    }
    let mut pages = vec![LayoutPage { id: WORK.into(), cards: work, ..Default::default() }];

    if personal {
        // dnešní pevné pořadí Osobní (MainPage): Čas, Časovač, Počasí, Kalendář, Právě hraje, Odkazy, Akce, Přihrádka, Poznámky
        let small: Vec<&str> = ["clock", "timer", "weather", "calendar"].into_iter().filter(|k| on(k)).collect();
        let media = on("nowPlaying") || on("sound");
        let mid: Vec<&str> = [("media", media), ("links", on("links"))].into_iter().filter(|(_, v)| *v).map(|(k, _)| k).collect();
        let mut cards = row(&small);
        cards.extend(row(&mid));
        cards.extend(["actions", "shelf", "notes"].into_iter().filter(|k| on(k)).map(|k| card(k, CardSize::Xl)));
        pages.push(LayoutPage { id: PERSONAL.into(), cards, ..Default::default() });
    }
    let hidden_parts = PARTS.iter().filter(|(p, _)| !on(p)).map(|(p, _)| p.to_string()).collect();
    LayoutConfig { pages, hidden_parts, upgrades: Vec::new() }
}

/// Jednorázové úpravy rozložení po aktualizaci (každá jen jednou — značka v `upgrades`; co uživatel
/// potom odebere, se nevrací). `personalProjects`: karta Projekty nahoru na Osobní, když tam nikdy
/// nebyla — jen když ji uživatel vůbec používá (je na nějaké záložce) a Osobní existuje. Čistá funkce.
pub fn apply_upgrades(l: &mut LayoutConfig) {
    if !l.upgrades.iter().any(|u| u == UP_PERSONAL_PROJECTS) {
        l.upgrades.push(UP_PERSONAL_PROJECTS.into());
        let used = placed(l, "projects");
        if let Some(p) = l.pages.iter_mut().find(|p| p.id == PERSONAL)
            && used
            && !p.cards.iter().any(|c| c.id == "projects")
        {
            p.cards.insert(0, card("projects", CardSize::Xl));
        }
    }
}

/// Po `config::load`: soubor bez `layout` → převod ze starého tvaru; pak jednorázové úpravy
/// ([`apply_upgrades`]). Nic se nezapisuje — změna přibude do souboru až s prvním uložením.
pub fn upgrade_loaded(raw: &Value, cfg: &mut Config) {
    if raw.get("layout").is_none() {
        cfg.layout = from_legacy(raw);
    }
    apply_upgrades(&mut cfg.layout);
}

/// Je karta na nějaké záložce?
pub fn placed(l: &LayoutConfig, id: &str) -> bool {
    l.pages.iter().any(|p| p.cards.iter().any(|c| c.id == id))
}

/// Odvozené pole z `layout`: `drawer.sections` (karta = je na nějaké záložce; část = karta je a část
/// není skrytá) a `modes.personalEnabled` (vždy zapnuto — záložky řídí `layout`). Idempotentní.
pub fn derive(cfg: &mut Config) {
    let l = &cfg.layout;
    let Ok(mut v) = serde_json::to_value(&cfg.drawer.sections) else { return };
    if let Some(m) = v.as_object_mut() {
        for (k, val) in m.iter_mut() {
            let on = if CARDS.contains(&k.as_str()) {
                placed(l, k)
            } else if let Some((_, parent)) = PARTS.iter().find(|(p, _)| p == k) {
                placed(l, parent) && !l.hidden_parts.iter().any(|h| h == k)
            } else {
                continue;
            };
            *val = Value::Bool(on);
        }
    }
    if let Ok(s) = serde_json::from_value(v) {
        cfg.drawer.sections = s;
    }
    cfg.modes.personal_enabled = true;
}

/// Záložka pro režim `mode` (id); neznámé id (smazaná záložka) = první.
pub fn page<'a>(l: &'a LayoutConfig, mode: &str) -> Option<&'a LayoutPage> {
    l.pages.iter().find(|p| p.id == mode).or_else(|| l.pages.first())
}

/// Id platné záložky pro `mode` (prázdné jen bez záložek — validace to nepustí).
pub fn page_id(l: &LayoutConfig, mode: &str) -> String {
    page(l, mode).map(|p| p.id.clone()).unwrap_or_default()
}

/// Je karta na záložce, která je teď vidět? (počasí a kalendář stahují jen tehdy)
pub fn page_has(l: &LayoutConfig, mode: &str, card: &str) -> bool {
    page(l, mode).is_some_and(|p| p.cards.iter().any(|c| c.id == card))
}

/// Název záložky pro proužek (ikona + název; vestavěná bez názvu podle jazyka).
pub fn title(p: &LayoutPage) -> String {
    let name = p.name.trim();
    let name = if !name.is_empty() {
        name.to_string()
    } else if p.id == WORK {
        tr("Práce", "Work").to_string()
    } else if p.id == PERSONAL {
        tr("Osobní", "Personal").to_string()
    } else {
        p.id.clone()
    };
    if p.icon.trim().is_empty() { name } else { format!("{} {name}", p.icon.trim()) }
}

fn id_ok(id: &str) -> bool {
    (1..=24).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Validace `layout` (cesta k poli → Nastavení ji ukáže u editoru).
pub fn validate(l: &LayoutConfig) -> Result<(), FieldError> {
    if l.pages.is_empty() {
        return Err(fe("layout.pages", tr("layout.pages: šuplík potřebuje aspoň jednu záložku.", "layout.pages: the drawer needs at least one tab.")));
    }
    if l.pages.len() > MAX_PAGES {
        return Err(fe("layout.pages", trf!("layout.pages: nejvýš {} záložek.", "layout.pages: at most {} tabs.", MAX_PAGES)));
    }
    let mut ids = std::collections::HashSet::new();
    for (i, p) in l.pages.iter().enumerate() {
        let at = |f: &str| format!("layout.pages.{i}.{f}");
        if !id_ok(&p.id) {
            return Err(fe(at("id"), trf!("layout.pages.{i}.id „{}“: jen a-z, 0-9 a pomlčka, nejvýš 24 znaků.", "layout.pages.{i}.id “{}”: only a-z, 0-9 and dash, at most 24 characters.", p.id)));
        }
        if !ids.insert(p.id.as_str()) {
            return Err(fe(at("id"), trf!("Záložka „{}“ je dvakrát.", "Tab “{}” is there twice.", p.id)));
        }
        if p.name.chars().count() > MAX_NAME {
            return Err(fe(at("name"), trf!("Název záložky má nejvýš {} znaků.", "A tab name has at most {} characters.", MAX_NAME)));
        }
        if p.id != WORK && p.id != PERSONAL && p.name.trim().is_empty() {
            return Err(fe(at("name"), tr("Vlastní záložka potřebuje název.", "A custom tab needs a name.")));
        }
        if p.icon.chars().count() > MAX_ICON {
            return Err(fe(at("icon"), tr("Ikona záložky je jeden znak nebo emoji.", "A tab icon is one character or emoji.")));
        }
        let mut seen = std::collections::HashSet::new();
        for (j, c) in p.cards.iter().enumerate() {
            if !CARDS.contains(&c.id.as_str()) {
                return Err(fe(format!("layout.pages.{i}.cards.{j}.id"), trf!("Karta „{}“ neexistuje (jde: {}).", "Card “{}” doesn't exist (available: {}).", c.id, CARDS.join(", "))));
            }
            if !seen.insert(c.id.as_str()) {
                return Err(fe(format!("layout.pages.{i}.cards.{j}.id"), trf!("Karta „{}“ je na záložce dvakrát.", "Card “{}” is on the tab twice.", c.id)));
            }
            if let Some(h) = c.h
                && !(1..=MAX_H).contains(&h)
            {
                return Err(fe(format!("layout.pages.{i}.cards.{j}.h"), trf!("Výška karty je 1–{} řádky (nebo chybí = podle obsahu).", "A card height is 1–{} rows (or absent = fit the content).", MAX_H)));
            }
        }
    }
    for (i, h) in l.hidden_parts.iter().enumerate() {
        if !PARTS.iter().any(|(p, _)| p == h) {
            let all: Vec<&str> = PARTS.iter().map(|(p, _)| *p).collect();
            return Err(fe(format!("layout.hiddenParts.{i}"), trf!("Část karty „{h}“ neexistuje (jde: {}).", "Card part “{h}” doesn't exist (available: {}).", all.join(", "))));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// (id, velikost) jedné záložky — čitelné porovnání v tabulkách
    fn shape(l: &LayoutConfig, page: &str) -> Vec<(String, CardSize)> {
        l.pages.iter().find(|p| p.id == page).map(|p| p.cards.iter().map(|c| (c.id.clone(), c.size)).collect()).unwrap_or_default()
    }
    fn ids(l: &LayoutConfig, page: &str) -> Vec<String> {
        shape(l, page).into_iter().map(|(id, _)| id).collect()
    }
    use CardSize::*;

    #[test]
    fn default_is_todays_drawer() {
        let d = LayoutConfig::default();
        let mut legacy = from_legacy(&json!({}));
        apply_upgrades(&mut legacy);
        assert_eq!(d, legacy);
        assert_eq!(d.upgrades, [UP_PERSONAL_PROJECTS]);
        assert_eq!(d.pages.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), [WORK, PERSONAL]);
        let s = |v: &[(&str, CardSize)]| v.iter().map(|(a, b)| (a.to_string(), *b)).collect::<Vec<_>>();
        assert_eq!(shape(&d, WORK), s(&[("projects", Xl), ("actions", Xl), ("shelf", Xl), ("running", Xl), ("claude", M), ("repos", M), ("notes", Xl)]));
        assert_eq!(
            shape(&d, PERSONAL),
            s(&[("projects", Xl), ("clock", S), ("timer", S), ("weather", S), ("calendar", S), ("media", M), ("links", M), ("actions", Xl), ("shelf", Xl), ("notes", Xl)])
        );
        assert!(d.pages.iter().flat_map(|p| &p.cards).all(|c| c.h.is_none() && c.tiles.is_none()), "výška podle obsahu, výchozí dlaždice");
        assert!(d.hidden_parts.is_empty());
        assert!(validate(&d).is_ok());
        // každá karta je ve výchozím rozložení (katalog je na začátku prázdný)
        for c in CARDS {
            assert!(placed(&d, c), "{c}");
        }
    }

    #[test]
    fn row_sizes_fill_a_row() {
        let w = |s: &CardSize| match s {
            S => 1,
            M => 2,
            L => 3,
            Xl => 4,
        };
        for n in 1..=4 {
            assert_eq!(row_sizes(n).iter().map(w).sum::<i32>(), 4, "{n}");
        }
        assert_eq!(row_sizes(6).len(), 6);
    }

    #[test]
    fn legacy_table() {
        // (popis, skutečný tvar starého configu, Práce, Osobní (None = záložka není), skryté části)
        type Case = (&'static str, Value, Vec<&'static str>, Option<Vec<&'static str>>, Vec<&'static str>);
        let cases: Vec<Case> = vec![
            ("prázdný", json!({}), vec!["projects", "actions", "shelf", "running", "claude", "repos", "notes"], Some(vec!["clock", "timer", "weather", "calendar", "media", "links", "actions", "shelf", "notes"]), vec![]),
            (
                "pořadí karet + skryté",
                json!({"drawer":{"width":1080,"sections":{"running":false,"weather":false,"claudeCosts":false,"projects":true}},"appearance":{"theme":"nord","cards":["notes","claude"]}}),
                vec!["projects", "actions", "shelf", "notes", "claude", "repos"],
                Some(vec!["clock", "timer", "calendar", "media", "links", "actions", "shelf", "notes"]),
                vec!["claudeCosts"],
            ),
            ("Osobní vypnutá", json!({"modes":{"personalEnabled":false},"drawer":{"sections":{"actions":false}}}), vec!["projects", "shelf", "running", "claude", "repos", "notes"], None, vec![]),
            (
                "jen hlasitost bez skladby",
                json!({"drawer":{"sections":{"nowPlaying":false,"battery":false}}}),
                vec!["projects", "actions", "shelf", "running", "claude", "repos", "notes"],
                Some(vec!["clock", "timer", "weather", "calendar", "media", "links", "actions", "shelf", "notes"]),
                vec!["nowPlaying", "battery"],
            ),
            (
                "bez médií",
                json!({"drawer":{"sections":{"nowPlaying":false,"sound":false,"links":false}}}),
                vec!["projects", "actions", "shelf", "running", "claude", "repos", "notes"],
                Some(vec!["clock", "timer", "weather", "calendar", "actions", "shelf", "notes"]),
                vec!["nowPlaying", "sound"],
            ),
            (
                "všechno skryté",
                json!({"drawer":{"sections":{"projects":false,"running":false,"claude":false,"notes":false,"repos":false,"actions":false,"shelf":false,"clock":false,"timer":false,"weather":false,"calendar":false,"links":false,"nowPlaying":false,"sound":false}}}),
                vec![],
                Some(vec![]),
                vec!["nowPlaying", "sound"],
            ),
            ("nesmysl v pořadí", json!({"appearance":{"cards":["repos","repos","pocasi",3]}}), vec!["projects", "actions", "shelf", "repos", "running", "claude", "notes"], Some(vec!["clock", "timer", "weather", "calendar", "media", "links", "actions", "shelf", "notes"]), vec![]),
        ];
        for (name, raw, work, personal, hidden) in cases {
            let l = from_legacy(&raw);
            assert_eq!(ids(&l, WORK), work, "{name}: Práce");
            match personal {
                Some(p) => assert_eq!(ids(&l, PERSONAL), p, "{name}: Osobní"),
                None => assert!(l.pages.iter().all(|p| p.id != PERSONAL), "{name}: bez Osobní"),
            }
            assert_eq!(l.hidden_parts, hidden, "{name}: části");
            assert!(validate(&l).is_ok(), "{name}: validní");
        }
    }

    #[test]
    fn legacy_sizes_follow_old_grid() {
        // jedna úzká karta (dřív mřížka „one“) = celá řada; tři malé Osobní = první širší
        let l = from_legacy(&json!({"drawer":{"sections":{"repos":false,"weather":false}}}));
        assert!(shape(&l, WORK).contains(&("claude".to_string(), Xl)));
        assert_eq!(shape(&l, PERSONAL)[..3], [("clock".to_string(), M), ("timer".to_string(), S), ("calendar".to_string(), S)]);
    }

    #[test]
    fn upgrade_only_without_layout() {
        let old = json!({"drawer":{"sections":{"claude":false}}});
        let mut cfg = Config::deserialize(&old).unwrap();
        upgrade_loaded(&old, &mut cfg);
        assert!(!ids(&cfg.layout, WORK).contains(&"claude".to_string()));
        // nový tvar se nepřepisuje (ani když `drawer.sections` říká něco jiného — odvozené pole)
        let new = json!({"layout":{"pages":[{"id":"hry","name":"Hry","icon":"🎮","cards":[{"id":"media","size":"xl"}]}]},"drawer":{"sections":{"media":false}}});
        let mut cfg = Config::deserialize(&new).unwrap();
        upgrade_loaded(&new, &mut cfg);
        assert_eq!(shape(&cfg.layout, "hry"), [("media".to_string(), Xl)]);
        assert_eq!(cfg.layout.pages.len(), 1);
    }

    #[test]
    fn derive_table() {
        // (popis, úprava rozložení, pole drawer.sections → očekávaná hodnota)
        type Case = (&'static str, Box<dyn Fn(&mut LayoutConfig)>, Vec<(&'static str, bool)>);
        let without = |id: &'static str| move |l: &mut LayoutConfig| l.pages.iter_mut().for_each(|p| p.cards.retain(|c| c.id != id));
        let cases: Vec<Case> = vec![
            ("výchozí", Box::new(|_| {}), vec![("weather", true), ("claudeCosts", true), ("nowPlaying", true), ("battery", true)]),
            ("bez počasí", Box::new(without("weather")), vec![("weather", false), ("calendar", true)]),
            ("akce jen z jedné záložky pryč", Box::new(|l: &mut LayoutConfig| l.pages[0].cards.retain(|c| c.id != "actions")), vec![("actions", true)]),
            ("bez Clauda = i jeho části", Box::new(without("claude")), vec![("claude", false), ("claudeCosts", false), ("claudeLimits", false)]),
            ("skrytá útrata", Box::new(|l: &mut LayoutConfig| l.hidden_parts = vec!["claudeCosts".into()]), vec![("claude", true), ("claudeCosts", false), ("claudeGit", true)]),
            ("bez médií", Box::new(without("media")), vec![("nowPlaying", false), ("sound", false)]),
            ("bez hodin = bez baterie", Box::new(without("clock")), vec![("clock", false), ("battery", false)]),
            ("karta na vlastní záložce", Box::new(|l: &mut LayoutConfig| {
                l.pages.retain(|p| p.id == WORK);
                l.pages.push(LayoutPage { id: "hry".into(), name: "Hry".into(), cards: vec![card("weather", S)], ..Default::default() });
            }), vec![("weather", true), ("clock", false)]),
        ];
        for (name, f, want) in cases {
            let mut cfg = Config::default();
            cfg.modes.personal_enabled = false;
            f(&mut cfg.layout);
            derive(&mut cfg);
            let v = serde_json::to_value(&cfg.drawer.sections).unwrap();
            for (k, on) in want {
                assert_eq!(v[k], json!(on), "{name}: {k}");
            }
            assert!(cfg.modes.personal_enabled, "{name}: Osobní řídí layout");
            let again = cfg.clone();
            derive(&mut cfg);
            assert_eq!(cfg, again, "{name}: idempotentní");
        }
    }

    #[test]
    fn every_section_field_is_derived() {
        // žádné pole drawer.sections nesmí zůstat „vlastní pravdou“ mimo layout
        let v = serde_json::to_value(crate::config::Sections::default()).unwrap();
        for k in v.as_object().unwrap().keys() {
            assert!(CARDS.contains(&k.as_str()) || PARTS.iter().any(|(p, _)| p == k), "{k} chybí v CARDS / PARTS");
        }
        for (_, parent) in PARTS {
            assert!(CARDS.contains(&parent), "{parent}");
        }
    }

    #[test]
    fn validation_table() {
        type Case = (&'static str, Box<dyn Fn(&mut LayoutConfig)>, Option<&'static str>);
        let custom = |name: &str, id: &str| LayoutPage { id: id.into(), name: name.into(), ..Default::default() };
        let cases: Vec<Case> = vec![
            ("výchozí", Box::new(|_| {}), None),
            ("bez záložek", Box::new(|l| l.pages.clear()), Some("layout.pages")),
            ("deset záložek", Box::new(move |l| (0..8).for_each(|i| l.pages.push(custom("X", &format!("x{i}"))))), Some("layout.pages")),
            ("devět záložek", Box::new(move |l| (0..7).for_each(|i| l.pages.push(custom("X", &format!("x{i}"))))), None),
            ("vlastní bez názvu", Box::new(move |l| l.pages.push(custom("  ", "hry"))), Some("layout.pages.2.name")),
            ("vestavěná bez názvu", Box::new(|l| l.pages[1].name.clear()), None),
            ("špatné id", Box::new(move |l| l.pages.push(custom("Hry", "Hry!"))), Some("layout.pages.2.id")),
            ("id dvakrát", Box::new(move |l| l.pages.push(custom("Práce 2", WORK))), Some("layout.pages.2.id")),
            ("dlouhý název", Box::new(|l| l.pages[0].name = "x".repeat(33)), Some("layout.pages.0.name")),
            ("emoji ikona", Box::new(|l| l.pages[0].icon = "🎮".into()), None),
            ("ikona text", Box::new(|l| l.pages[0].icon = "hry hry".into()), Some("layout.pages.0.icon")),
            ("neznámá karta", Box::new(|l| l.pages[0].cards.push(card("pocasi", S))), Some("layout.pages.0.cards.7.id")),
            ("karta dvakrát", Box::new(|l| l.pages[0].cards.push(card("claude", S))), Some("layout.pages.0.cards.7.id")),
            ("stejná karta na dvou záložkách", Box::new(|l| l.pages[1].cards.push(card("claude", L))), None),
            ("neznámá část", Box::new(|l| l.hidden_parts = vec!["git".into()]), Some("layout.hiddenParts.0")),
            ("výška 4", Box::new(|l| l.pages[0].cards[1].h = Some(4)), None),
            ("výška 0", Box::new(|l| l.pages[0].cards[1].h = Some(0)), Some("layout.pages.0.cards.1.h")),
            ("výška 5", Box::new(|l| l.pages[1].cards[0].h = Some(5)), Some("layout.pages.1.cards.0.h")),
        ];
        for (name, f, want) in cases {
            let mut l = LayoutConfig::default();
            f(&mut l);
            assert_eq!(validate(&l).err().and_then(|e| e.field).as_deref(), want, "{name}");
        }
    }

    #[test]
    fn upgrade_personal_projects_table() {
        // (popis, rozložení z configu 0.2.16 (už s `layout`, bez `upgrades`), Osobní po načtení)
        type Case = (&'static str, Value, Option<Vec<&'static str>>);
        let cases: Vec<Case> = vec![
            (
                "skutečný tvar 0.2.16",
                json!({"pages":[{"id":"prace","name":"","cards":[{"id":"projects","size":"xl"},{"id":"claude","size":"m"},{"id":"repos","size":"m"},{"id":"notes","size":"xl"}]},{"id":"osobni","name":"","cards":[{"id":"clock","size":"s"},{"id":"weather","size":"s"}]}],"hiddenParts":["claudeCosts"]}),
                Some(vec!["projects", "clock", "weather"]),
            ),
            (
                "Projekty už na Osobní jsou (jinde v pořadí)",
                json!({"pages":[{"id":"osobni","name":"","cards":[{"id":"clock","size":"s"},{"id":"projects","size":"m"}]}]}),
                Some(vec!["clock", "projects"]),
            ),
            (
                "Projekty uživatel nikde nemá",
                json!({"pages":[{"id":"prace","name":"","cards":[{"id":"claude","size":"m"}]},{"id":"osobni","name":"","cards":[{"id":"clock","size":"s"}]}]}),
                Some(vec!["clock"]),
            ),
            ("bez záložky Osobní", json!({"pages":[{"id":"prace","name":"","cards":[{"id":"projects","size":"xl"}]}]}), None),
            (
                "už proběhlo a uživatel je z Osobní odebral",
                json!({"pages":[{"id":"prace","name":"","cards":[{"id":"projects","size":"xl"}]},{"id":"osobni","name":"","cards":[{"id":"clock","size":"s"}]}],"upgrades":["personalProjects"]}),
                Some(vec!["clock"]),
            ),
        ];
        for (name, layout, personal) in cases {
            let raw = json!({ "layout": layout });
            let mut cfg = Config::deserialize(&raw).unwrap();
            upgrade_loaded(&raw, &mut cfg);
            match personal {
                Some(p) => assert_eq!(ids(&cfg.layout, PERSONAL), p, "{name}"),
                None => assert!(cfg.layout.pages.iter().all(|p| p.id != PERSONAL), "{name}"),
            }
            assert_eq!(cfg.layout.upgrades, [UP_PERSONAL_PROJECTS], "{name}: značka");
            let again = cfg.layout.clone();
            apply_upgrades(&mut cfg.layout);
            assert_eq!(cfg.layout, again, "{name}: jen jednou");
        }
        // starý config bez `layout` (před vlnou 6) dostane Projekty na Osobní taky; skryté projekty ne
        let old = json!({"drawer":{"sections":{"weather":false}}});
        let mut cfg = Config::deserialize(&old).unwrap();
        upgrade_loaded(&old, &mut cfg);
        assert_eq!(ids(&cfg.layout, PERSONAL)[..2], ["projects", "clock"]);
        let off = json!({"drawer":{"sections":{"projects":false}}});
        let mut cfg = Config::deserialize(&off).unwrap();
        upgrade_loaded(&off, &mut cfg);
        assert!(!placed(&cfg.layout, "projects"));
    }

    #[test]
    fn removed_from_personal_stays_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let old = r#"{"layout":{"pages":[{"id":"prace","name":"","cards":[{"id":"projects","size":"xl"}]},{"id":"osobni","name":"","cards":[{"id":"clock","size":"s"}]}],"hiddenParts":[]}}"#;
        std::fs::write(&path, old).unwrap();
        let mut cfg = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(ids(&cfg.layout, PERSONAL), ["projects", "clock"], "migrace");
        // uživatel kartu z Osobní odebere a uloží
        cfg.layout.pages[1].cards.retain(|c| c.id != "projects");
        crate::config::save(&path, &cfg).unwrap();
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["layout"]["upgrades"], json!(["personalProjects"]));
        let again = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(ids(&again.layout, PERSONAL), ["clock"], "odebranou už nevracet");
    }

    #[test]
    fn height_and_tiles_shape() {
        // (popis, karta v JSON, výška, volby dlaždic)
        let cases: Vec<(&str, Value, Option<u8>, Option<TileOpts>)> = vec![
            ("starý tvar", json!({"id":"claude","size":"m"}), None, None),
            ("výška", json!({"id":"claude","size":"m","h":3}), Some(3), None),
            (
                "dlaždice jen část",
                json!({"id":"projects","size":"xl","tiles":{"size":"small","git":false}}),
                None,
                Some(TileOpts { size: TileSize::Small, label: true, git: false, hidden: vec![] }),
            ),
            (
                "dlaždice celé",
                json!({"id":"projects","size":"xl","h":2,"tiles":{"size":"large","label":false,"git":true,"hidden":["harbor"]}}),
                Some(2),
                Some(TileOpts { size: TileSize::Large, label: false, git: true, hidden: vec!["harbor".into()] }),
            ),
        ];
        for (name, v, h, tiles) in cases {
            let c: LayoutCard = serde_json::from_value(v.clone()).unwrap();
            assert_eq!((c.h, c.tiles.clone()), (h, tiles), "{name}");
            let back = serde_json::to_value(&c).unwrap();
            assert_eq!(serde_json::from_value::<LayoutCard>(back.clone()).unwrap(), c, "{name}: tam a zpět");
            if name == "starý tvar" {
                assert_eq!(back, v, "bez výšky a voleb se nic nového nepíše");
            }
        }
        assert!(serde_json::from_value::<LayoutCard>(json!({"id":"projects","tiles":{"size":"huge"}})).is_err());
        let schema = crate::config::schema_json();
        for needle in ["\"tiles\"", "\"upgrades\"", "\"small\""] {
            assert!(schema.contains(needle), "{needle} ve schématu");
        }
    }


    #[test]
    fn page_lookup_falls_back_to_first() {
        let l = LayoutConfig::default();
        assert_eq!(page_id(&l, PERSONAL), PERSONAL);
        assert_eq!(page_id(&l, "smazana"), WORK);
        assert!(page_has(&l, PERSONAL, "weather"));
        assert!(!page_has(&l, WORK, "weather"));
        assert!(!page_has(&l, "smazana", "weather"), "neznámá = první záložka (Práce)");
        let p = LayoutPage { id: "hry".into(), name: " Hry ".into(), icon: "🎮".into(), ..Default::default() };
        assert_eq!(title(&p), "🎮 Hry");
        assert!(!title(&l.pages[0]).is_empty());
    }

    #[test]
    fn serde_shape_and_schema() {
        let v = serde_json::to_value(LayoutConfig::default()).unwrap();
        assert_eq!(v["pages"][0]["cards"][0], json!({"id":"projects","size":"xl"}));
        assert!(v["pages"][0].get("icon").is_none(), "prázdná ikona se nepíše");
        assert_eq!(v["hiddenParts"], json!([]));
        let schema = crate::config::schema_json();
        for needle in ["\"layout\"", "hiddenParts", "\"xl\""] {
            assert!(schema.contains(needle), "{needle} ve schématu");
        }
    }

    #[test]
    fn old_file_through_load_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let old = r#"{"modes":{"personalEnabled":true},"drawer":{"sections":{"weather":false,"claudeGit":false}},"appearance":{"cards":["repos"]}}"#;
        std::fs::write(&path, old).unwrap();
        let cfg = crate::config::load(&path).unwrap().unwrap();
        assert!(!ids(&cfg.layout, PERSONAL).contains(&"weather".to_string()));
        assert_eq!(ids(&cfg.layout, WORK)[3..], ["repos", "running", "claude", "notes"]);
        assert!(!cfg.drawer.sections.weather && !cfg.drawer.sections.claude_git && cfg.drawer.sections.claude);
        crate::config::save(&path, &cfg).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let saved: Value = serde_json::from_str(&text).unwrap();
        assert!(saved.get("layout").is_some(), "nový klíč: {text}");
        assert!(saved["appearance"].get("cards").is_none(), "staré pořadí se nepíše: {text}");
        let again = crate::config::load(&path).unwrap().unwrap();
        assert_eq!(again.layout, cfg.layout);
        assert_eq!(again.drawer.sections, cfg.drawer.sections);
    }
}
