//! Lístky na plochu: existující poznámka Ledge v malém okně bez rámu přes ostatní okna (QA webů,
//! checklist). Žádný druhý svět poznámek — obsah čte a ukládá UI lístku přes `note_read` / `note_save`
//! / `note_toggle_task` s otiskem, změny zvenku hlásí `win::on_notes_changed` (volá `reload_notes`).
//!
//! Tady jsou čisté věci (tabulkové testy): stav lístku (`Pin`, `PinPrefs`, `PinPatch`), soubor
//! `note-pins.json` v `%LOCALAPPDATA%\Ledge` (`load` / `save`), label okna (`label_for`) a geometrie
//! (`place_new`, `restore`). Okna samotná: `win.rs`, příkazy: `commands/pins.rs`, mapa
//! `docs/codemap/notes.md`.
//!
//! Geometrie: pozice = vnější levý horní roh ve FYZICKÝCH px (virtuální plocha, tím je dán i monitor),
//! velikost = vnitřní v LOGICKÝCH px — po přesunu na monitor s jiným DPI zůstane lístek stejně velký.

pub(crate) mod win;

use crate::webpanel::Area;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Okna lístků mají label `note-<klíč>` (klíč = otisk cesty poznámky, `key`).
pub const LABEL_PREFIX: &str = "note-";
/// Nejmenší lístek (logické px) — lišta + aspoň dva řádky textu.
pub const MIN_W: f64 = 160.0;
pub const MIN_H: f64 = 90.0;
pub const MAX_W: f64 = 4000.0;
pub const MAX_H: f64 = 3000.0;
/// Výška lišty (logické px) — sbalený lístek je jen lišta.
pub const BAR_H: f64 = 28.0;
pub const DEFAULT_W: f64 = 300.0;
pub const DEFAULT_H: f64 = 240.0;
/// Průhlednost textu nejníž sem (%) — text musí zůstat čitelný.
pub const TEXT_MIN: u8 = 50;
pub const FILE: &str = "note-pins.json";
/// Okraj od kraje plochy a posun každého dalšího nového lístku (logické px).
const MARGIN: f64 = 16.0;
const CASCADE: f64 = 28.0;
/// Kolik lišty musí ležet na pracovní ploše, aby šel lístek chytit (fyzické px).
const GRAB_W: i32 = 48;
const GRAB_H: i32 = 12;

/// Cesta pro porovnání: malá písmena, zpětná lomítka, bez lomítka na konci.
fn normalize(path: &str) -> String {
    path.trim().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Klíč lístku z cesty poznámky (32 hex — v labelu okna smí být jen `[a-zA-Z0-9-/:_]`).
pub fn key(path: &str) -> String {
    crate::notes::edit::hash(normalize(path).as_bytes())
}

pub fn label_for(path: &str) -> String {
    format!("{LABEL_PREFIX}{}", key(path))
}

pub fn same_note(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

/// Styl lístku (galerie v ⋯ → Vzhled). `Classic` = původní tmavé sklo s posuvníky průhlednosti.
/// Neznámé id (např. z novějšího Ledge) = `Classic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PinStyle {
    #[default]
    Classic,
    Paper,
    Glass,
    Stripe,
    Minimal,
    Dark,
}

impl PinStyle {
    pub const ALL: [PinStyle; 6] = [Self::Classic, Self::Paper, Self::Glass, Self::Stripe, Self::Minimal, Self::Dark];

    pub fn id(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Paper => "paper",
            Self::Glass => "glass",
            Self::Stripe => "stripe",
            Self::Minimal => "minimal",
            Self::Dark => "dark",
        }
    }

    /// Ze jména v configu (`notes.pins.style`); neznámé = `None`.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id.trim())
    }
}

impl Serialize for PinStyle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for PinStyle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let id = String::deserialize(d)?;
        Ok(Self::from_id(&id).unwrap_or_default())
    }
}

/// Volby lístku, které vidí a mění UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinPrefs {
    /// Vždy navrchu (jinak normální okno — schová se za aktivní)
    pub on_top: bool,
    /// Zamknout na místě: nejde posunout ani měnit velikost
    pub locked: bool,
    /// Průchozí: myš jde oknem na stránku pod ním, lístek je jen k čtení
    pub through: bool,
    /// neprůhlednost pozadí 0–100 %
    pub bg: u8,
    /// neprůhlednost textu `TEXT_MIN`–100 %
    pub text: u8,
    /// sbaleno na lištu (dvojklik na lištu)
    pub collapsed: bool,
    /// styl (vzhled) lístku
    pub style: PinStyle,
}

impl Default for PinPrefs {
    fn default() -> Self {
        Self { on_top: true, locked: false, through: false, bg: 88, text: 100, collapsed: false, style: PinStyle::Classic }
    }
}

impl PinPrefs {
    pub fn sanitized(mut self) -> Self {
        self.bg = self.bg.min(100);
        self.text = self.text.clamp(TEXT_MIN, 100);
        self
    }

    pub fn look(&self) -> PinLook {
        PinLook { bg: self.bg, text: self.text, style: self.style }
    }

    pub fn with_look(self, l: PinLook) -> Self {
        Self { bg: l.bg, text: l.text, style: l.style, ..self }.sanitized()
    }
}

/// Vzhled, který si nový lístek převezme (naposledy použitý, nebo výchozí z Nastavení `notes.pins`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinLook {
    pub bg: u8,
    pub text: u8,
    pub style: PinStyle,
}

impl Default for PinLook {
    fn default() -> Self {
        PinPrefs::default().look()
    }
}

/// Vzhled nového lístku: naposledy použitý (když si ho Ledge má pamatovat a nějaký je), jinak výchozí.
pub fn new_look(last: Option<PinLook>, remember: bool, default: PinLook) -> PinLook {
    let l = if remember { last.unwrap_or(default) } else { default };
    PinPrefs::default().with_look(l).look()
}

/// Změna voleb z UI (chybějící pole = beze změny).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinPatch {
    pub on_top: Option<bool>,
    pub locked: Option<bool>,
    pub through: Option<bool>,
    pub bg: Option<u8>,
    pub text: Option<u8>,
    pub collapsed: Option<bool>,
    pub style: Option<PinStyle>,
}

impl PinPatch {
    pub fn apply(&self, p: PinPrefs) -> PinPrefs {
        PinPrefs {
            on_top: self.on_top.unwrap_or(p.on_top),
            locked: self.locked.unwrap_or(p.locked),
            through: self.through.unwrap_or(p.through),
            bg: self.bg.unwrap_or(p.bg),
            text: self.text.unwrap_or(p.text),
            collapsed: self.collapsed.unwrap_or(p.collapsed),
            style: self.style.unwrap_or(p.style),
        }
        .sanitized()
    }

    /// Mění vzhled (pozadí, text, styl)? Pak se zapamatuje jako „naposledy použitý“.
    pub fn touches_look(&self) -> bool {
        self.bg.is_some() || self.text.is_some() || self.style.is_some()
    }
}

/// Lístek (řádek v `note-pins.json`). Po odepnutí řádek zůstane (`open: false`) — znovupřipnutí ho
/// otevře na stejném místě, ve stejné velikosti a vzhledu. Jen tento počítač (`%LOCALAPPDATA%`), nikdy
/// do `.md` ani do syncu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Pin {
    /// absolutní cesta poznámky (jak ji zná UI)
    pub path: String,
    /// vnější pozice (fyzické px — určuje i monitor); `None` = ještě neumístěný
    pub x: Option<i32>,
    pub y: Option<i32>,
    /// vnitřní velikost rozbaleného lístku (logické px)
    pub w: f64,
    pub h: f64,
    #[serde(flatten)]
    pub prefs: PinPrefs,
    /// připnutý na ploše (starý soubor bez pole = připnutý); `false` = odepnutý, pamatuje se poloha a vzhled
    pub open: bool,
    /// schovaný do data (ms epochy): okno je zavřené, v ten čas se samo znovu otevře
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_until: Option<i64>,
    /// kdy se odepnul (ms) — z odepnutých se pamatuje jen `MAX_CLOSED` naposledy odepnutých
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_ms: Option<i64>,
}

impl Default for Pin {
    fn default() -> Self {
        Self { path: String::new(), x: None, y: None, w: DEFAULT_W, h: DEFAULT_H, prefs: PinPrefs::default(), open: true, hidden_until: None, closed_ms: None }
    }
}

impl Pin {
    pub fn new(path: &str) -> Self {
        Self { path: path.to_string(), ..Self::default() }
    }

    fn sanitized(mut self) -> Self {
        let size = |v: f64, lo: f64, hi: f64, def: f64| if v.is_finite() { v.clamp(lo, hi) } else { def };
        self.w = size(self.w, MIN_W, MAX_W, DEFAULT_W);
        self.h = size(self.h, MIN_H, MAX_H, DEFAULT_H);
        if self.x.is_none() || self.y.is_none() {
            (self.x, self.y) = (None, None);
        }
        if !self.open {
            self.hidden_until = None;
        }
        self.prefs = self.prefs.sanitized();
        self
    }

    /// Má být teď na ploše (připnutý a ne schovaný do budoucna)?
    pub fn visible_at(&self, now_ms: i64) -> bool {
        self.open && self.hidden_until.is_none_or(|t| t <= now_ms)
    }
}

/// Kolik odepnutých lístků si Ledge pamatuje (polohu a vzhled) — soubor neroste donekonečna.
pub const MAX_CLOSED: usize = 200;

/// Celý soubor stavu lístků.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinsFile {
    /// nápověda „jak z průchozího režimu ven“ už se ukázala
    pub hint_seen: bool,
    /// naposledy použitý vzhled (pozadí, text, styl) — převezme ho nový lístek
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<PinLook>,
    pub pins: Vec<Pin>,
}

impl PinsFile {
    pub fn by_label(&self, label: &str) -> Option<&Pin> {
        self.pins.iter().find(|p| label_for(&p.path) == label)
    }

    pub fn by_label_mut(&mut self, label: &str) -> Option<&mut Pin> {
        self.pins.iter_mut().find(|p| label_for(&p.path) == label)
    }

    /// Lístek poznámky `path` — existující (i odepnutý, s pamětí polohy), jinak nový se vzhledem `look`.
    pub fn ensure(&mut self, path: &str, look: PinLook) -> &mut Pin {
        if let Some(i) = self.pins.iter().position(|p| same_note(&p.path, path)) {
            return &mut self.pins[i];
        }
        let mut p = Pin::new(path);
        p.prefs = p.prefs.with_look(look);
        self.pins.push(p);
        self.pins.last_mut().expect("právě vložený")
    }

    /// Připnout (znovu): odepnutý / schovaný lístek se vrátí se zapamatovanou polohou a vzhledem.
    pub fn reopen(&mut self, path: &str, look: PinLook) -> &mut Pin {
        let p = self.ensure(path, look);
        if !p.open {
            // průchozí lístek po znovupřipnutí jde ovládat (jinak by na něj nešlo kliknout)
            p.prefs.through = false;
        }
        p.open = true;
        p.hidden_until = None;
        p.closed_ms = None;
        p
    }

    /// Odepnout (poznámka zůstane, poloha a vzhled se pamatují). Vrací true, když byl připnutý.
    pub fn close(&mut self, path: &str, now_ms: i64) -> bool {
        let Some(p) = self.pins.iter_mut().find(|p| same_note(&p.path, path)) else { return false };
        let was = p.open;
        p.open = false;
        p.hidden_until = None;
        p.closed_ms = Some(now_ms);
        self.prune_closed();
        was
    }

    /// Z odepnutých nechá jen `MAX_CLOSED` naposledy odepnutých (připnuté zůstanou vždy).
    fn prune_closed(&mut self) {
        let mut closed: Vec<(i64, usize)> = self.pins.iter().enumerate().filter(|(_, p)| !p.open).map(|(i, p)| (p.closed_ms.unwrap_or(0), i)).collect();
        if closed.len() <= MAX_CLOSED {
            return;
        }
        closed.sort_unstable_by(|a, b| b.cmp(a));
        let drop: Vec<usize> = closed[MAX_CLOSED..].iter().map(|(_, i)| *i).collect();
        let mut i = 0;
        self.pins.retain(|_| {
            let keep = !drop.contains(&i);
            i += 1;
            keep
        });
    }

    /// Schovat do data: okno zmizí, v `until_ms` se samo vrátí. Vrací true, když je lístek připnutý.
    pub fn hide(&mut self, path: &str, until_ms: i64) -> bool {
        let Some(p) = self.pins.iter_mut().find(|p| same_note(&p.path, path) && p.open) else { return false };
        p.hidden_until = Some(until_ms);
        true
    }

    /// Cesty lístků, které mají být teď na ploše (po startu, po konci hry).
    pub fn visible(&self, now_ms: i64) -> Vec<String> {
        self.pins.iter().filter(|p| p.visible_at(now_ms)).map(|p| p.path.clone()).collect()
    }

    /// Schované lístky, jejichž čas nastal.
    pub fn due_hidden(&self, now_ms: i64) -> Vec<String> {
        self.pins.iter().filter(|p| p.open && p.hidden_until.is_some_and(|t| t <= now_ms)).map(|p| p.path.clone()).collect()
    }

    /// Nejbližší čas, kdy se má schovaný lístek vrátit (jeden termín pro vlákno `emit`).
    pub fn next_unhide(&self, now_ms: i64) -> Option<i64> {
        self.pins.iter().filter(|p| p.open).filter_map(|p| p.hidden_until).filter(|&t| t > now_ms).min()
    }
}

/// Stav lístků (chybějící / poškozený soubor = žádné; hodnoty mimo rozsah se srovnají, duplicity pryč).
pub fn load(path: &Path) -> PinsFile {
    let mut f: PinsFile = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let mut seen: Vec<String> = Vec::new();
    f.pins = f
        .pins
        .into_iter()
        .filter(|p| !p.path.trim().is_empty())
        .filter(|p| {
            let n = normalize(&p.path);
            let new = !seen.contains(&n);
            seen.push(n);
            new
        })
        .map(Pin::sanitized)
        .collect();
    f
}

/// Zápis celého souboru (tmp + přejmenování — nikdy půlka souboru).
pub fn save(path: &Path, f: &PinsFile) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(f).unwrap_or_default())?;
    std::fs::rename(&tmp, path)
}

// ---------- geometrie ----------

/// Monitor pro umístění: pracovní plocha (fyzické px) a měřítko DPI (1.0 = 96 dpi).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mon {
    pub work: Area,
    pub scale: f64,
}

/// Kam okno dát: vnější pozice a vnitřní velikost ve fyzických px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

fn px(v: f64, scale: f64) -> i32 {
    (v * scale).round() as i32
}

/// Fyzická velikost lístku na monitoru (sbalený = jen lišta).
fn phys_size(p: &Pin, scale: f64) -> (i32, i32) {
    let h = if p.prefs.collapsed { BAR_H } else { p.h };
    (px(p.w, scale).max(1), px(h, scale).max(1))
}

/// Vejde se do plochy: zmenšit na plochu (s okrajem), pak posunout celé dovnitř.
fn fit(x: i32, y: i32, w: i32, h: i32, work: Area, margin: i32) -> Place {
    let w = w.min((work.r - work.l - 2 * margin).max(1));
    let h = h.min((work.b - work.t - 2 * margin).max(1));
    let x = x.clamp(work.l, (work.r - w).max(work.l));
    let y = y.clamp(work.t, (work.b - h).max(work.t));
    Place { x, y, w: w as u32, h: h as u32 }
}

/// Nový lístek: vpravo nahoře na pracovní ploše (pod proužkem `top_gap` fyzicky), každý další
/// (`index` = kolik lístků už je otevřených) o kus níž a vlevo, ať se nepřekryjí přesně.
pub fn place_new(p: &Pin, mon: &Mon, top_gap: i32, index: usize) -> Place {
    let (w, h) = phys_size(p, mon.scale);
    let m = px(MARGIN, mon.scale);
    let step = px(CASCADE, mon.scale) * (index % 8) as i32;
    let x = mon.work.r - m - w - step;
    let y = mon.work.t + top_gap.max(m) + step;
    fit(x, y, w, h, mon.work, m)
}

/// Index monitoru, na kterém leží lišta lístku (nejvíc překryvu), `None` = lišta mimo všechny
/// (odpojený monitor, změněné rozlišení) nebo jí na ploše leží míň než `GRAB_W` × `GRAB_H` px.
pub fn monitor_of(x: i32, y: i32, w: i32, bar: i32, mons: &[Mon]) -> Option<usize> {
    let (l, t, r, b) = (x, y, x.saturating_add(w), y.saturating_add(bar));
    mons.iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let ow = r.min(m.work.r) - l.max(m.work.l);
            let oh = b.min(m.work.b) - t.max(m.work.t);
            (ow >= GRAB_W && oh >= GRAB_H).then_some((i, ow as i64 * oh as i64))
        })
        .max_by_key(|(_, a)| *a)
        .map(|(i, _)| i)
}

/// Obnova po restartu Ledge: uložená pozice na monitoru, kde lišta leží (velikost × DPI toho
/// monitoru, posunuté celé dovnitř jeho plochy); neumístěný nebo mimo obrazovku → `place_new` na
/// `fallback` (monitor proužku). Měřítko pro odhad šířky lišty bere `fallback` (pozice je fyzická).
pub fn restore(p: &Pin, mons: &[Mon], fallback: &Mon, top_gap: i32, index: usize) -> Place {
    if let (Some(x), Some(y)) = (p.x, p.y) {
        let guess = phys_size(p, fallback.scale).0;
        if let Some(m) = monitor_of(x, y, guess, px(BAR_H, fallback.scale), mons).map(|i| mons[i]) {
            let (w, h) = phys_size(p, m.scale);
            return fit(x, y, w, h, m.work, 0);
        }
    }
    place_new(p, fallback, top_gap, index)
}

/// Co zakrývá obrazovku z pohledu lístků (počítá vlákno `hover` ze `screen::fullscreen_state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FsKind {
    /// nic přes celou obrazovku
    None,
    /// D3D exkluzivní fullscreen / prezentace
    Exclusive,
    /// přes monitor leží okno prohlížeče (F11)
    CoverBrowser,
    /// přes monitor leží okno Ledge / lístku
    CoverOwn,
    /// přes monitor leží jiné okno (hra v okně bez rámu, video)
    CoverOther,
}

/// Prohlížeče, nad kterými lístky zůstanou i ve fullscreenu (QA webů přes F11). Název exe bez přípony.
const BROWSERS: [&str; 8] = ["chrome", "msedge", "firefox", "brave", "opera", "vivaldi", "arc", "zen"];

pub(crate) fn is_browser_exe(exe: &str) -> bool {
    let name = exe.rsplit(['\\', '/']).next().unwrap_or(exe);
    let stem = name.rsplit_once('.').filter(|(_, ext)| ext.eq_ignore_ascii_case("exe")).map_or(name, |(s, _)| s);
    BROWSERS.iter().any(|b| stem.eq_ignore_ascii_case(b))
}

/// Co leží přes celý monitor: okno Ledge / lístku (`own`), prohlížeč, nebo něco jiného.
pub(crate) fn cover_kind(own: bool, exe: &str) -> FsKind {
    if own {
        FsKind::CoverOwn
    } else if is_browser_exe(exe) {
        FsKind::CoverBrowser
    } else {
        FsKind::CoverOther
    }
}

/// Mají lístky „Vždy navrchu“ ustoupit? Hře (herní režim, D3D, borderless okno jiné aplikace) ano,
/// prohlížeči ve F11 a vlastnímu oknu ne.
pub(crate) fn pins_yield(game_mode: bool, fs: FsKind) -> bool {
    game_mode || matches!(fs, FsKind::Exclusive | FsKind::CoverOther)
}

/// Co udělat při změně fullscreenu / herního režimu (`win::sync_dead`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeadPlan {
    /// přepnout oknům „vždy navrchu“ (jen při skutečné změně)
    pub retop: bool,
    /// spustit odloženou obnovu po startu — i BEZ změny stavu (start ve hře: `DEAD` začíná na false)
    pub restore: bool,
}

pub(crate) fn dead_plan(was_dead: bool, dead: bool, restore_pending: bool) -> DeadPlan {
    DeadPlan { retop: was_dead != dead, restore: !dead && restore_pending }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yield_table() {
        use FsKind::*;
        // (herní režim, fullscreen) → ustoupí?
        let cases = [
            (false, None, false),
            (false, Exclusive, true),
            (false, CoverBrowser, false),
            (false, CoverOwn, false),
            (false, CoverOther, true),
            (true, None, true),
            (true, Exclusive, true),
            (true, CoverBrowser, true),
            (true, CoverOwn, true),
            (true, CoverOther, true),
        ];
        for (gm, fs, want) in cases {
            assert_eq!(pins_yield(gm, fs), want, "herní {gm}, {fs:?}");
        }
    }

    #[test]
    fn dead_plan_table() {
        // (dřív ustupovaly, teď ustupují, čeká obnova) → (přepnout navrchu, obnovit)
        let cases = [
            (false, false, false, false, false),
            // start v herním režimu: obnova čekala, `DEAD` se nezměnil (false → false) — dřív se nespustila
            (false, false, true, false, true),
            (true, false, true, true, true),
            (true, false, false, true, false),
            (false, true, true, true, false),
            (true, true, true, false, false),
        ];
        for (was, dead, pending, retop, restore) in cases {
            assert_eq!(dead_plan(was, dead, pending), DeadPlan { retop, restore }, "{was} {dead} {pending}");
        }
    }

    #[test]
    fn browser_and_cover_classification() {
        for exe in ["chrome.exe", "Chrome.EXE", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe", "arc.exe", "zen.exe", "chrome", "C:\\x\\Opera.exe"] {
            assert!(is_browser_exe(exe), "{exe}");
        }
        for exe in ["", "game.exe", "chromedriver.exe", "notchrome.exe", "chrome.dll", "vlc.exe"] {
            assert!(!is_browser_exe(exe), "{exe}");
        }
        assert_eq!(cover_kind(true, "chrome.exe"), FsKind::CoverOwn);
        assert_eq!(cover_kind(false, "msedge.exe"), FsKind::CoverBrowser);
        assert_eq!(cover_kind(false, "game.exe"), FsKind::CoverOther);
        assert_eq!(cover_kind(false, ""), FsKind::CoverOther);
    }

    const FHD: Area = Area { l: 0, t: 0, r: 1920, b: 1040 };
    const RIGHT: Area = Area { l: 1920, t: 0, r: 4480, b: 1400 };

    fn mons() -> Vec<Mon> {
        vec![Mon { work: FHD, scale: 1.0 }, Mon { work: RIGHT, scale: 1.5 }]
    }

    fn at(x: i32, y: i32) -> Pin {
        Pin { x: Some(x), y: Some(y), ..Pin::new(r"C:\n\a.md") }
    }

    #[test]
    fn label_is_stable_and_case_insensitive() {
        let a = label_for(r"C:\Users\x\Ledge\notes\_obecne\a.md");
        assert!(a.starts_with(LABEL_PREFIX) && a.len() == LABEL_PREFIX.len() + 32, "{a}");
        assert!(a[LABEL_PREFIX.len()..].chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, label_for("c:/users/X/ledge/notes/_obecne/A.md"));
        assert_ne!(a, label_for(r"C:\Users\x\Ledge\notes\_obecne\b.md"));
        assert!(same_note(r"C:\a\b.md", "c:/A/B.md/"));
        // nikdy se nepotká s oknem s webem ani se šuplíkem
        assert!(!a.starts_with(crate::webpanel::LABEL_PREFIX) && a != crate::drawer::LABEL);
    }

    #[test]
    fn patch_and_limits() {
        let p = PinPrefs::default();
        assert!(p.on_top && !p.locked && !p.through && !p.collapsed);
        let cases: &[(PinPatch, PinPrefs)] = &[
            (PinPatch::default(), p),
            (PinPatch { locked: Some(true), ..Default::default() }, PinPrefs { locked: true, ..p }),
            (PinPatch { through: Some(true), on_top: Some(false), ..Default::default() }, PinPrefs { through: true, on_top: false, ..p }),
            // text nikdy pod hranici čitelnosti, pozadí nejvýš 100 %
            (PinPatch { text: Some(0), bg: Some(250), ..Default::default() }, PinPrefs { text: TEXT_MIN, bg: 100, ..p }),
            (PinPatch { bg: Some(0), ..Default::default() }, PinPrefs { bg: 0, ..p }),
        ];
        for (patch, want) in cases {
            assert_eq!(patch.apply(p), *want, "{patch:?}");
        }
        // UI posílá jen změněné pole
        let patch: PinPatch = serde_json::from_str(r#"{"collapsed":true}"#).unwrap();
        assert_eq!(patch.apply(p), PinPrefs { collapsed: true, ..p });
        assert!(!patch.touches_look());
        // styl: id malými písmeny, vzhled se pamatuje
        let patch: PinPatch = serde_json::from_str(r#"{"style":"paper"}"#).unwrap();
        assert_eq!(patch.apply(p).style, PinStyle::Paper);
        assert!(patch.touches_look() && PinPatch { bg: Some(1), ..Default::default() }.touches_look());
    }

    #[test]
    fn style_ids_and_unknown() {
        for s in PinStyle::ALL {
            assert_eq!(PinStyle::from_id(s.id()), Some(s));
            assert_eq!(serde_json::to_string(&s).unwrap(), format!("\"{}\"", s.id()));
        }
        assert_eq!(PinStyle::from_id("neon"), None);
        // neznámý styl ze souboru (novější Ledge) = původní vzhled, nic se neztratí
        let p: PinPrefs = serde_json::from_str(r#"{"style":"neon","bg":40}"#).unwrap();
        assert_eq!((p.style, p.bg), (PinStyle::Classic, 40));
        assert_eq!(PinPrefs::default().style, PinStyle::Classic);
    }

    #[test]
    fn new_look_table() {
        let def = PinLook::default();
        let custom = PinLook { bg: 40, text: 70, style: PinStyle::Glass };
        let last = PinLook { bg: 20, text: 90, style: PinStyle::Dark };
        let cases: &[(Option<PinLook>, bool, PinLook, PinLook, &str)] = &[
            (None, true, def, def, "nic naposledy → výchozí (dnešní vzhled)"),
            (None, true, custom, custom, "nic naposledy → výchozí z Nastavení"),
            (Some(last), true, custom, last, "naposledy použitý vyhrává"),
            (Some(last), false, custom, custom, "pamatování vypnuté → vždy z Nastavení"),
            (Some(PinLook { bg: 200, text: 0, style: PinStyle::Paper }), true, def, PinLook { bg: 100, text: TEXT_MIN, style: PinStyle::Paper }, "meze"),
        ];
        for (l, remember, d, want, why) in cases {
            assert_eq!(new_look(*l, *remember, *d), *want, "{why}");
        }
    }

    #[test]
    fn file_serde_old_and_broken() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join(FILE);
        assert_eq!(load(&path), PinsFile::default());
        // plochý tvar, chybějící pole = výchozí, nesmysly srovnané, duplicita a prázdná cesta pryč
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"pins":[{"path":"C:\\n\\a.md","x":10,"y":20,"w":5,"h":99999,"text":3,"through":true},
                        {"path":"c:/n/A.md","x":1,"y":1},{"path":""},{"path":"C:\\n\\b.md","x":7}],"neznamy":1}"#,
        )
        .unwrap();
        let f = load(&path);
        assert!(!f.hint_seen);
        assert_eq!(f.pins.len(), 2);
        let a = &f.pins[0];
        assert_eq!((a.x, a.y, a.w, a.h), (Some(10), Some(20), MIN_W, MAX_H));
        assert_eq!(a.prefs, PinPrefs { through: true, text: TEXT_MIN, ..PinPrefs::default() });
        // jen x bez y = neumístěný
        assert_eq!((f.pins[1].x, f.pins[1].y), (None, None));
        // zápis a zpět beze ztráty
        assert!(f.pins.iter().all(|p| p.open), "starý soubor bez `open` = připnuté");
        let mut g = f.clone();
        g.hint_seen = true;
        g.last = Some(PinLook { bg: 30, text: 80, style: PinStyle::Stripe });
        g.ensure(r"C:\n\c.md", PinLook::default()).prefs.locked = true;
        g.close(r"C:\n\b.md", 5);
        g.hide(r"C:\n\a.md", 99);
        save(&path, &g).unwrap();
        assert_eq!(load(&path), g);
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"hintSeen\": true") && raw.contains("\"onTop\"") && !raw.contains("\"prefs\""), "{raw}");
        std::fs::write(&path, "{rozbité").unwrap();
        assert_eq!(load(&path), PinsFile::default());
    }

    #[test]
    fn ensure_close_reopen_remembers() {
        let look = PinLook { bg: 40, text: 70, style: PinStyle::Paper };
        let mut f = PinsFile::default();
        f.ensure(r"C:\n\a.md", look).w = 400.0;
        f.ensure("c:/n/A.md", PinLook::default()).h = 300.0;
        assert_eq!(f.pins.len(), 1);
        assert_eq!((f.pins[0].w, f.pins[0].h), (400.0, 300.0));
        assert_eq!(f.pins[0].prefs.look(), look, "nový lístek dostane vzhled, existující si nechá svůj");
        assert!(f.by_label(&label_for(r"C:\N\a.md")).is_some());
        f.pins[0].x = Some(2100);
        f.pins[0].y = Some(50);
        f.pins[0].prefs.through = true;
        // odepnutí: řádek zůstane i s polohou, velikostí a vzhledem
        assert!(f.close("C:/n/a.md", 10));
        assert!(!f.close("C:/n/a.md", 11), "druhé odepnutí nic");
        assert!(!f.close("C:/n/jiny.md", 11));
        let p = &f.pins[0];
        assert_eq!((p.open, p.x, p.y, p.w, p.h, p.closed_ms), (false, Some(2100), Some(50), 400.0, 300.0, Some(11)));
        assert!(f.visible(0).is_empty());
        // znovupřipnutí: stejné místo a vzhled, průchozí vypnutý (jinak by na něj nešlo kliknout)
        let p = f.reopen(r"C:\N\A.md", PinLook::default()).clone();
        assert_eq!((p.open, p.x, p.y, p.w, p.prefs.style, p.prefs.through, p.closed_ms), (true, Some(2100), Some(50), 400.0, PinStyle::Paper, false, None));
        assert_eq!(f.pins.len(), 1);
    }

    #[test]
    fn hide_until_and_wake() {
        let mut f = PinsFile::default();
        f.reopen("C:/n/a.md", PinLook::default());
        f.reopen("C:/n/b.md", PinLook::default());
        f.reopen("C:/n/c.md", PinLook::default());
        f.close("C:/n/c.md", 1);
        assert!(f.hide("C:/n/a.md", 1_000));
        assert!(f.hide("C:/n/b.md", 500));
        assert!(!f.hide("C:/n/c.md", 300), "odepnutý se neschovává");
        assert!(!f.hide("C:/n/x.md", 300));
        // (teď, viditelné, k vynoření, další termín)
        type Case<'a> = (i64, &'a [&'a str], &'a [&'a str], Option<i64>);
        let cases: &[Case] = &[
            (0, &[], &[], Some(500)),
            (499, &[], &[], Some(500)),
            (500, &["C:/n/b.md"], &["C:/n/b.md"], Some(1_000)),
            (2_000, &["C:/n/a.md", "C:/n/b.md"], &["C:/n/a.md", "C:/n/b.md"], None),
        ];
        for (now, vis, due, next) in cases {
            assert_eq!(f.visible(*now), *vis, "visible @{now}");
            assert_eq!(f.due_hidden(*now), *due, "due @{now}");
            assert_eq!(f.next_unhide(*now), *next, "next @{now}");
        }
        // znovu otevřít = schování zrušené
        f.reopen("C:/n/a.md", PinLook::default());
        assert_eq!(f.next_unhide(0), Some(500));
        // odepnutí schování zruší
        f.close("C:/n/b.md", 3);
        assert_eq!(f.next_unhide(0), None);
    }

    #[test]
    fn closed_are_capped_open_kept() {
        let mut f = PinsFile::default();
        f.reopen("C:/n/stale-pripnuty.md", PinLook::default());
        for i in 0..(MAX_CLOSED + 5) {
            let p = format!("C:/n/{i}.md");
            f.reopen(&p, PinLook::default());
            f.close(&p, i as i64);
        }
        assert_eq!(f.pins.iter().filter(|p| !p.open).count(), MAX_CLOSED);
        assert!(f.pins.iter().any(|p| p.open), "připnutý nikdy nevypadne");
        // vypadly nejdřív odepnuté
        assert!(!f.pins.iter().any(|p| p.path == "C:/n/0.md") && f.pins.iter().any(|p| p.path == format!("C:/n/{}.md", MAX_CLOSED + 4)));
    }

    #[test]
    fn new_pins_top_right_cascade() {
        let m = Mon { work: FHD, scale: 1.0 };
        let p = Pin::new("x");
        assert_eq!(place_new(&p, &m, 40, 0), Place { x: 1920 - 16 - 300, y: 40, w: 300, h: 240 });
        let second = place_new(&p, &m, 40, 1);
        assert_eq!((second.x, second.y), (1920 - 16 - 300 - 28, 68));
        // DPI 150 %: velikost i okraje × 1,5
        let hi = Mon { work: RIGHT, scale: 1.5 };
        assert_eq!(place_new(&p, &hi, 0, 0), Place { x: 4480 - 24 - 450, y: 24, w: 450, h: 360 });
        // sbalený = jen lišta
        let c = Pin { prefs: PinPrefs { collapsed: true, ..PinPrefs::default() }, ..p.clone() };
        assert_eq!(place_new(&c, &m, 40, 0).h, BAR_H as u32);
        // obří lístek na malé ploše → zmenší se dovnitř
        let big = Pin { w: 3000.0, h: 2000.0, ..p };
        let g = place_new(&big, &Mon { work: Area { l: 0, t: 0, r: 800, b: 600 }, scale: 1.0 }, 0, 0);
        assert!(g.x >= 0 && g.y >= 0 && g.x + g.w as i32 <= 800 && g.y + g.h as i32 <= 600, "{g:?}");
    }

    #[test]
    fn restore_on_screen_and_off_screen() {
        let ms = mons();
        let fb = ms[0];
        let cases: &[(Pin, Place, &str)] = &[
            (at(100, 100), Place { x: 100, y: 100, w: 300, h: 240 }, "na prvním monitoru beze změny"),
            (at(2000, 50), Place { x: 2000, y: 50, w: 450, h: 360 }, "druhý monitor: logická velikost × jeho DPI"),
            (at(1700, 900), Place { x: 1620, y: 800, w: 300, h: 240 }, "přesah přes okraj → celé dovnitř"),
            (at(9000, 100), place_new(&Pin::new("x"), &fb, 40, 2), "odpojený monitor → nový na monitoru proužku"),
            (at(-280, 10), place_new(&Pin::new("x"), &fb, 40, 2), "jen 20 px lišty na ploše → nejde chytit"),
            (at(100, 1035), place_new(&Pin::new("x"), &fb, 40, 2), "lišta pod plochou (hlavní panel)"),
            (Pin::new("x"), place_new(&Pin::new("x"), &fb, 40, 2), "neumístěný"),
        ];
        for (p, want, why) in cases {
            assert_eq!(restore(p, &ms, &fb, 40, 2), *want, "{why}");
        }
        // lišta přes dva monitory → ten s větším překryvem
        assert_eq!(monitor_of(1800, 10, 300, 28, &ms), Some(1));
        assert_eq!(monitor_of(1700, 10, 300, 28, &ms), Some(0));
        assert_eq!(monitor_of(0, 0, 300, 28, &[]), None);
    }
}
