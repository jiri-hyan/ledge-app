//! Přihrádka na soubory (jako NotchNook): soubory přetažené z Průzkumníka na proužek se uloží jako
//! **odkazy** (cesty) — nic se nekopíruje ani nepřesouvá. Seznam je v `%APPDATA%\Ledge\shelf.json`
//! (strop `MAX_ITEMS`, nejnovější nahoře), v šuplíku karta „Přihrádka“ (`src/shelf/*`).
//!
//! Tady je čistá logika (dedup, strop, formát cest do schránky, serializace, překryv proužku, stav
//! „tah nad proužkem“) + úložiště. Win32: cíl přetažení `drop.rs`, přetažení ven `dragout.rs`,
//! schránka `clip.rs`. Mapa: `docs/codemap/shelf.md`.

pub mod clip;
pub mod dragout;
pub mod drop;

use crate::i18n::tr;
use crate::strip::{Rgba, Segment, StripModel};
use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::RECT;

/// Nejvýš položek v přihrádce; nejstarší nad strop odpadnou.
pub const MAX_ITEMS: usize = 50;
/// Nejkratší výška cílové zóny u hrany (logické px) — i když je proužek jen linka nebo skrytý.
pub const ZONE_MIN_H: i32 = 40;
/// Jak dlouho proužek po puštění ukazuje „Přihrádka: +N“ (ms).
pub const DONE_MS: u32 = 1500;
const FILE: &str = "shelf.json";

fn yes() -> bool {
    true
}

/// Sekce `shelf` v configu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ShelfConfig {
    /// Přihrádka: přetažení souborů na proužek je uloží (jen cesty) do karty „Přihrádka“. Výchozí true.
    #[serde(default = "yes")]
    pub enabled: bool,
}

impl Default for ShelfConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Jedna položka: absolutní cesta + kdy přibyla (ms od epochy).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub path: String,
    #[serde(default)]
    pub added_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Stored {
    #[serde(default)]
    items: Vec<Item>,
}

/// Výsledek přidání: kolik nových, kolik už tam bylo (posunuto nahoru), kolik odpadlo stropem.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Added {
    pub new: usize,
    pub moved: usize,
    pub dropped: usize,
}

/// Formát cest do schránky.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// prostá cesta (`C:\a b\c.txt`), víc cest = řádek na cestu
    Plain,
    /// zmínka pro Claude Code: `@C:\a\c.txt`, s mezerou `@"C:\a b\c.txt"`; víc cest na jednom řádku
    Claude,
}

/// Normalizace cesty: ořez mezer, `/` → `\`, bez koncového `\` (kromě kořene `C:\`). Prázdná = `None`.
pub fn normalize(p: &str) -> Option<String> {
    let s = p.trim().trim_matches('"').replace('/', "\\");
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut s = s.to_string();
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    Some(s)
}

/// Stejná cesta? Windows: bez ohledu na velikost písmen.
pub fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Přidá cesty na začátek (v pořadí puštění); co už v přihrádce je, se posune nahoru (bez kopie);
/// nad `max` odpadnou nejstarší (konec seznamu).
pub fn add(items: &mut Vec<Item>, paths: &[String], now_ms: u64, max: usize) -> Added {
    let mut out = Added::default();
    let mut fresh: Vec<Item> = Vec::new();
    for p in paths.iter().filter_map(|p| normalize(p)) {
        if fresh.iter().any(|i| same(&i.path, &p)) {
            continue;
        }
        match items.iter().position(|i| same(&i.path, &p)) {
            Some(at) => {
                items.remove(at);
                out.moved += 1;
            }
            None => out.new += 1,
        }
        fresh.push(Item { path: p, added_ms: now_ms });
    }
    fresh.append(items);
    if fresh.len() > max {
        out.dropped = fresh.len() - max;
        fresh.truncate(max);
    }
    *items = fresh;
    out
}

/// Odebere cestu; `true` = byla tam.
pub fn remove(items: &mut Vec<Item>, path: &str) -> bool {
    let before = items.len();
    items.retain(|i| !same(&i.path, path));
    items.len() != before
}

/// Zmínka pro Claude Code: `@cesta`, s bílým znakem v uvozovkách `@"cesta"`.
pub fn claude_ref(path: &str) -> String {
    if path.chars().any(char::is_whitespace) { format!("@\"{path}\"") } else { format!("@{path}") }
}

/// Text do schránky. Prostá: řádek na cestu (CRLF, jak ho čekají aplikace Windows); Claude: zmínky
/// na jednom řádku oddělené mezerou (rovnou do promptu).
pub fn clip_text(paths: &[String], f: Format) -> String {
    match f {
        Format::Plain => paths.join("\r\n"),
        Format::Claude => paths.iter().map(|p| claude_ref(p)).collect::<Vec<_>>().join(" "),
    }
}

/// Název a složka pro zobrazení (`C:\a\b.txt` → `b.txt`, `C:\a`; kořen `C:\` → `C:\`, „“).
pub fn split(path: &str) -> (String, String) {
    let t = path.trim_end_matches('\\');
    match t.rfind('\\') {
        Some(i) if i + 1 < t.len() => {
            let dir = &t[..i];
            let dir = if dir.len() == 2 && dir.ends_with(':') { format!("{dir}\\") } else { dir.to_string() };
            (t[i + 1..].to_string(), dir)
        }
        _ => (path.to_string(), String::new()),
    }
}

pub fn parse(json: &str) -> Vec<Item> {
    let mut v = serde_json::from_str::<Stored>(json).map(|s| s.items).unwrap_or_default();
    // ručně upravený soubor: prázdné / zdvojené cesty pryč, strop
    let mut out: Vec<Item> = Vec::new();
    for mut i in v.drain(..) {
        let Some(p) = normalize(&i.path) else { continue };
        if out.iter().any(|o| same(&o.path, &p)) {
            continue;
        }
        i.path = p;
        out.push(i);
    }
    out.truncate(MAX_ITEMS);
    out
}

pub fn serialize(items: &[Item]) -> String {
    serde_json::to_string_pretty(&serde_json::json!({ "items": items })).unwrap_or_default()
}

// ---------- úložiště ----------

struct Store {
    file: PathBuf,
    items: Vec<Item>,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn load_from(file: &Path) -> Vec<Item> {
    std::fs::read_to_string(file).map(|s| parse(&s)).unwrap_or_default()
}

fn save_to(file: &Path, items: &[Item]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serialize(items)).and_then(|_| std::fs::rename(&tmp, file)).map_err(|e| crate::errors::io(tr("Přihrádku nejde uložit", "Can't save the shelf"), &e))
}

/// Práce se seznamem (načte se líně při prvním použití). `f` vrací `true` = změna → uložit + událost.
fn with<R>(f: impl FnOnce(&mut Vec<Item>) -> (R, bool)) -> Result<R, String> {
    let mut g = STORE.lock();
    let s = g.get_or_insert_with(|| {
        let file = crate::config::config_dir().join(FILE);
        let items = load_from(&file);
        Store { file, items }
    });
    let (r, changed) = f(&mut s.items);
    if changed {
        save_to(&s.file, &s.items)?;
        drop(g);
        notify();
    }
    Ok(r)
}

fn notify() {
    use tauri::Emitter;
    let _ = crate::state::core().app.emit_to(crate::drawer::LABEL, "shelf", ());
}

pub fn items() -> Vec<Item> {
    with(|v| (v.clone(), false)).unwrap_or_default()
}

/// Puštěné soubory (cíl přetažení na proužku).
pub fn add_paths(paths: &[String]) -> Result<Added, String> {
    let now = crate::state::now_ms().max(0) as u64;
    with(|v| {
        let a = add(v, paths, now, MAX_ITEMS);
        (a, a.new + a.moved > 0)
    })
}

pub fn remove_path(path: &str) -> Result<bool, String> {
    with(|v| {
        let r = remove(v, path);
        (r, r)
    })
}

pub fn clear() -> Result<(), String> {
    with(|v| {
        let had = !v.is_empty();
        v.clear();
        ((), had)
    })
}

/// Je cesta v přihrádce? (příkazy z UI pracují jen s tím, co tam opravdu je)
pub fn contains(path: &str) -> Option<String> {
    items().into_iter().find(|i| same(&i.path, path)).map(|i| i.path)
}

// ---------- proužek ----------

/// Co proužek ukazuje kvůli přihrádce.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Hint {
    #[default]
    None,
    /// soubory se táhnou nad proužkem — „Pustit do přihrádky“
    Over,
    /// právě puštěno N souborů (na `DONE_MS`)
    Done(usize),
}

/// Překryv modelu proužku (kopie pro kreslení): při tahu nad proužkem ho ukáže a zvýrazní jako cíl,
/// po puštění krátce „Přihrádka: +N“. Mrtvý Ledge (fullscreen, herní režim) = nic.
pub fn overlay(m: &mut StripModel, dead: bool) {
    if dead || m.monitor.right <= m.monitor.left {
        return;
    }
    let seg = match m.shelf {
        Hint::None => return,
        Hint::Over => Segment { text: tr("Pustit do přihrádky", "Drop on the shelf").into(), color: Rgba::TEXT, dot: Some(Rgba::TEAL), strong: true, pill: true },
        Hint::Done(n) => Segment { text: trf!("Přihrádka: +{n}", "Shelf: +{n}"), color: Rgba::TEXT, dot: Some(Rgba::GREEN), strong: false, pill: true },
    };
    m.visible = true;
    m.line_only = false;
    m.hot = m.shelf == Hint::Over;
    m.ambient = Some(if m.shelf == Hint::Over { Rgba::TEAL } else { Rgba::GREEN });
    m.segments = vec![seg];
}

/// Cílová zóna u hrany (px): šířka zóny proužku, výška aspoň `ZONE_MIN_H` logických px.
pub fn zone(zl: i32, zr: i32, zb: i32, top: i32, dpi: i32) -> RECT {
    let min_h = ZONE_MIN_H * dpi / 96;
    RECT { left: zl, top, right: zr, bottom: zb.max(top + min_h) }
}

fn inside(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

fn union(a: &RECT, b: &RECT) -> RECT {
    RECT { left: a.left.min(b.left), top: a.top.min(b.top), right: a.right.max(b.right), bottom: a.bottom.max(b.bottom) }
}

/// „Táhne se něco na proužek?“ (vlákno `hover`, tik 16 ms u hrany). Zóna se aktivuje, jen když kurzor
/// do ní VJEL s drženým tlačítkem (tah začal jinde) — klik na taby prohlížeče pod proužkem ji
/// nespustí. Během aktivace drží zónu z okamžiku vjezdu (sjednocenou s aktuální), aby se cíl
/// nepřesouval pod kurzorem, když proužek změní šířku.
#[derive(Debug, Default)]
pub struct Arm {
    prev_in: bool,
    carried: bool,
    held: Option<RECT>,
}

impl Arm {
    /// `zone` = dnešní zóna (`None` = přihrádka teď nesmí: vypnutá, otevřený šuplík, mrtvý Ledge).
    /// `button` se volá jen s kurzorem v zóně (GetAsyncKeyState jen u hrany). Vrací aktivní zónu.
    pub fn step(&mut self, zone: Option<RECT>, x: i32, y: i32, button: impl FnOnce() -> bool) -> Option<RECT> {
        let Some(z) = zone else {
            *self = Arm::default();
            return None;
        };
        let area = self.held.map_or(z, |h| union(&h, &z));
        if !inside(&area, x, y) {
            *self = Arm::default();
            return None;
        }
        let down = button();
        if !self.prev_in {
            self.carried = down;
        } else if !down {
            self.carried = false;
        }
        self.prev_in = true;
        if self.carried {
            self.held = Some(area);
        } else {
            self.held = None;
        }
        self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(paths: &[&str]) -> Vec<Item> {
        paths.iter().map(|p| Item { path: p.to_string(), added_ms: 1 }).collect()
    }
    fn paths(v: &[Item]) -> Vec<&str> {
        v.iter().map(|i| i.path.as_str()).collect()
    }
    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn normalize_table() {
        let cases: &[(&str, Option<&str>)] = &[
            (r"C:\a\b.txt", Some(r"C:\a\b.txt")),
            ("  C:/a/b  ", Some(r"C:\a\b")),
            (r"C:\a\dir\\", Some(r"C:\a\dir")),
            (r"C:\", Some(r"C:\")),
            (r#""C:\a b\c""#, Some(r"C:\a b\c")),
            (r"\\server\share\x", Some(r"\\server\share\x")),
            ("   ", None),
            ("", None),
        ];
        for (inp, want) in cases {
            assert_eq!(normalize(inp).as_deref(), *want, "{inp:?}");
        }
    }

    #[test]
    fn add_dedup_order_and_cap() {
        // (přihrádka před, puštěno, strop, po, (nové, posunuté, odpadlé))
        type Case<'a> = (Vec<&'a str>, Vec<&'a str>, usize, Vec<&'a str>, (usize, usize, usize));
        let cases: Vec<Case> = vec![
            (vec![], vec![r"C:\a", r"C:\b"], 50, vec![r"C:\a", r"C:\b"], (2, 0, 0)),
            (vec![r"C:\x"], vec![r"C:\a"], 50, vec![r"C:\a", r"C:\x"], (1, 0, 0)),
            // už tam je (jiná velikost písmen) → nahoru, bez kopie
            (vec![r"C:\x", r"C:\A"], vec![r"c:\a"], 50, vec![r"c:\a", r"C:\x"], (0, 1, 0)),
            // dvakrát v jednom puštění
            (vec![], vec![r"C:\a", r"C:\A\"], 50, vec![r"C:\a"], (1, 0, 0)),
            // strop: nejstarší (konec) odpadne
            (vec![r"C:\1", r"C:\2", r"C:\3"], vec![r"C:\n"], 3, vec![r"C:\n", r"C:\1", r"C:\2"], (1, 0, 1)),
            (vec![r"C:\1"], vec!["  ", ""], 3, vec![r"C:\1"], (0, 0, 0)),
        ];
        for (before, dropped, max, after, (n, m, d)) in cases {
            let mut v = items(&before);
            let a = add(&mut v, &s(&dropped), 7, max);
            assert_eq!(paths(&v), after, "{before:?} + {dropped:?}");
            assert_eq!((a.new, a.moved, a.dropped), (n, m, d), "{before:?} + {dropped:?}");
        }
        let mut v = vec![];
        add(&mut v, &s(&[r"C:\a"]), 42, 5);
        assert_eq!(v[0].added_ms, 42);
    }

    #[test]
    fn remove_case_insensitive() {
        let mut v = items(&[r"C:\a", r"C:\B"]);
        assert!(remove(&mut v, r"c:\b"));
        assert!(!remove(&mut v, r"C:\zzz"));
        assert_eq!(paths(&v), vec![r"C:\a"]);
    }

    #[test]
    fn clip_formats() {
        let cases: &[(&[&str], Format, &str)] = &[
            (&[r"C:\a\b.txt"], Format::Plain, r"C:\a\b.txt"),
            (&[r"C:\a b\c.txt"], Format::Plain, r"C:\a b\c.txt"),
            (&[r"C:\a", r"D:\b c"], Format::Plain, "C:\\a\r\nD:\\b c"),
            (&[r"C:\a\b.txt"], Format::Claude, r"@C:\a\b.txt"),
            (&[r"C:\a b\c.txt"], Format::Claude, r#"@"C:\a b\c.txt""#),
            (&[r"C:\Program Files (x86)\x"], Format::Claude, r#"@"C:\Program Files (x86)\x""#),
            (&[r"C:\a", r"D:\b c"], Format::Claude, r#"@C:\a @"D:\b c""#),
            (&[], Format::Plain, ""),
            (&[], Format::Claude, ""),
        ];
        for (p, f, want) in cases {
            assert_eq!(clip_text(&s(p), *f), *want, "{p:?} {f:?}");
        }
    }

    #[test]
    fn split_table() {
        let cases = [
            (r"C:\a\b.txt", ("b.txt", r"C:\a")),
            (r"C:\b.txt", ("b.txt", r"C:\")),
            (r"C:\a\dir\", ("dir", r"C:\a")),
            (r"\\srv\share\f", ("f", r"\\srv\share")),
            (r"C:\", (r"C:\", "")),
            ("bez", ("bez", "")),
        ];
        for (p, (n, d)) in cases {
            assert_eq!(split(p), (n.to_string(), d.to_string()), "{p}");
        }
    }

    #[test]
    fn serialization_roundtrip_and_tolerance() {
        let v = vec![Item { path: r"C:\a b\c.txt".into(), added_ms: 5 }];
        let j = serialize(&v);
        assert!(j.contains("addedMs"), "{j}");
        assert_eq!(parse(&j), v);
        // poškozený / cizí soubor = prázdná přihrádka, ne pád
        assert!(parse("nesmysl").is_empty());
        assert!(parse("{}").is_empty());
        // ruční úprava: prázdné a zdvojené pryč, chybějící čas = 0
        let p = parse(r#"{"items":[{"path":"C:\\a"},{"path":"c:\\A\\"},{"path":"  "},{"path":"C:/b"}]}"#);
        assert_eq!(paths(&p), vec![r"C:\a", r"C:\b"]);
        assert_eq!(p[0].added_ms, 0);
        let many: Vec<Item> = (0..80).map(|i| Item { path: format!(r"C:\f{i}"), added_ms: 0 }).collect();
        assert_eq!(parse(&serialize(&many)).len(), MAX_ITEMS);
    }

    #[test]
    fn store_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sub").join(FILE);
        assert!(load_from(&file).is_empty(), "chybějící soubor = prázdná přihrádka");
        let v = items(&[r"C:\a", r"C:\b"]);
        save_to(&file, &v).unwrap();
        assert_eq!(load_from(&file), v);
        assert!(!file.with_extension("json.tmp").exists());
    }

    #[test]
    fn config_default_on() {
        let cfg: crate::config::Config = serde_json::from_str("{}").unwrap();
        assert!(cfg.shelf.enabled, "chybějící sekce = zapnuto");
        let cfg: crate::config::Config = serde_json::from_str(r#"{"shelf":{}}"#).unwrap();
        assert!(cfg.shelf.enabled);
        let cfg: crate::config::Config = serde_json::from_str(r#"{"shelf":{"enabled":false}}"#).unwrap();
        assert!(!cfg.shelf.enabled);
        assert!(crate::config::schema_json().contains("\"shelf\""));
    }

    #[test]
    fn overlay_table() {
        let mon = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
        let base = || StripModel { visible: false, line_only: true, monitor: mon, ..Default::default() };
        // bez náznaku se nic nemění
        let mut m = base();
        overlay(&mut m, false);
        assert_eq!(m, base());
        // tah nad proužkem: ukázat, zvýraznit, jeden segment
        let mut m = StripModel { shelf: Hint::Over, ..base() };
        overlay(&mut m, false);
        assert!(m.visible && !m.line_only && m.hot);
        assert_eq!(m.segments.len(), 1);
        assert_eq!(m.ambient, Some(Rgba::TEAL));
        // po puštění: počet, bez nažhavení
        let mut m = StripModel { shelf: Hint::Done(3), ..base() };
        crate::i18n::with_lang(crate::uiconf::Lang::Cs, || overlay(&mut m, false));
        assert!(m.visible && !m.hot);
        assert_eq!(m.segments[0].text, "Přihrádka: +3");
        // mrtvý Ledge / bez monitoru = nic
        let mut m = StripModel { shelf: Hint::Over, ..base() };
        overlay(&mut m, true);
        assert!(!m.visible);
        let mut m = StripModel { shelf: Hint::Over, monitor: RECT::default(), ..base() };
        overlay(&mut m, false);
        assert!(!m.visible);
    }

    #[test]
    fn zone_min_height() {
        assert_eq!(zone(10, 20, 4, 0, 96), RECT { left: 10, top: 0, right: 20, bottom: 40 });
        assert_eq!(zone(10, 20, 4, 0, 144), RECT { left: 10, top: 0, right: 20, bottom: 60 });
        assert_eq!(zone(10, 20, 70, 0, 96).bottom, 70);
        assert_eq!(zone(10, 20, -1076, -1080, 96).bottom, -1040, "monitor nad hlavním");
    }

    #[test]
    fn arm_table() {
        let z = Some(RECT { left: 100, top: 0, right: 300, bottom: 40 });
        let inz = (200, 10);
        let out = (200, 200);
        // (kroky: (zóna povolena, poloha, tlačítko), očekávaná aktivace po každém kroku)
        type Step = (bool, (i32, i32), bool, bool);
        let cases: Vec<(&str, Vec<Step>)> = vec![
            ("tah zvenku do zóny", vec![(true, out, true, false), (true, inz, true, true), (true, inz, true, true)]),
            ("puštění = konec", vec![(true, inz, true, true), (true, inz, false, false), (true, inz, true, false)]),
            ("klik v zóně (taby) nic", vec![(true, inz, false, false), (true, inz, true, false), (true, inz, true, false)]),
            ("vjezd bez tlačítka nic", vec![(true, out, false, false), (true, inz, false, false), (true, inz, true, false)]),
            ("odjezd ze zóny", vec![(true, inz, true, true), (true, out, true, false), (true, inz, true, true)]),
            ("zakázáno (šuplík, mrtvý)", vec![(true, inz, true, true), (false, inz, true, false), (true, inz, true, true)]),
        ];
        for (name, steps) in cases {
            let mut a = Arm::default();
            for (i, (on, (x, y), btn, want)) in steps.into_iter().enumerate() {
                let got = a.step(if on { z } else { None }, x, y, || btn).is_some();
                assert_eq!(got, want, "{name}: krok {i}");
            }
        }
        // tlačítko se mimo zónu nečte (GetAsyncKeyState jen u hrany)
        let mut a = Arm::default();
        a.step(z, out.0, out.1, || panic!("tlačítko mimo zónu"));
        a.step(None, inz.0, inz.1, || panic!("tlačítko se zakázanou zónou"));
    }

    #[test]
    fn arm_holds_zone_while_strip_changes() {
        let wide = RECT { left: 100, top: 0, right: 300, bottom: 40 };
        let narrow = RECT { left: 180, top: 0, right: 220, bottom: 40 };
        let mut a = Arm::default();
        a.step(Some(wide), 500, 10, || true);
        assert_eq!(a.step(Some(wide), 120, 10, || true), Some(wide));
        // proužek se zúžil, kurzor je pořád v původní zóně → drží
        assert_eq!(a.step(Some(narrow), 120, 10, || true), Some(wide));
    }
}
