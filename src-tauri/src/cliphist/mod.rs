//! Historie schránky pro paletu (`Ctrl+K`, prefix `v:`) — soukromí napřed, **výchozí vypnuto**.
//!
//! - Sběr událostmi: vlákno `cliphist` s oknem jen pro zprávy (`HWND_MESSAGE`) a
//!   `AddClipboardFormatListener` → `WM_CLIPBOARDUPDATE` (`win.rs`). Běží JEN se zapnutou funkcí;
//!   vypnutí = listener pryč, okno zničené, historie z paměti smazaná.
//! - Co neukládat rozhoduje `filter.rs` (značky formátů správců hesel, zdrojová aplikace, prázdné,
//!   dlouhé, tvary tokenů) — čisté funkce, tabulkové testy.
//! - Úložiště: jen paměť (`History`), posledních `max` nepřipnutých; připnuté volitelně na disk
//!   (`%LOCALAPPDATA%\Ledge\clip-pins.json`, `savePins`). Zamčení stanice volitelně smaže nepřipnuté.
//! - Vložení zpět (`paste`) zapíše přes `shelf::clip::set_text` a označí zápis jako vlastní
//!   (`filter::Own`) — listener z něj nový záznam neudělá.
//!
//! Mapa: `docs/codemap/clipboard.md`.

pub mod apps;
pub mod filter;
mod win;
#[cfg(test)]
mod tests;

use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use filter::Own;
use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_MAX: u32 = 200;
pub const DEFAULT_MAX: u32 = 25;
/// nejvýš připnutých položek
pub const MAX_PINS: usize = 25;
/// strop paměti historie (součet textů, bajty UTF-8) — drží `ledge.exe` pod rozpočtem i při 200 × 64 KB
pub const MAX_TOTAL_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_EXCLUDE: usize = 50;
/// náhled v paletě (znaky po sloučení bílých znaků)
pub const PREVIEW_CHARS: usize = 160;
pub const PINS_FILE: &str = "clip-pins.json";

// ---------- config ----------

fn default_apps() -> Vec<String> {
    ["KeePass.exe", "KeePassXC.exe", "1Password.exe", "Bitwarden.exe"].map(String::from).to_vec()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClipboardConfig {
    /// Historie schránky v paletě (prefix `v:`). Výchozí false = nic se neposlouchá ani neukládá.
    pub enabled: bool,
    /// Kolik posledních (nepřipnutých) textů držet v paměti, 1–200. Výchozí 25.
    pub max: u32,
    /// Neukládat texty, které vypadají jako tokeny / klíče (`sk-…`, `ghp_…`, `AKIA…`, JWT, PEM). Výchozí true.
    pub skip_secrets: bool,
    /// Aplikace (název exe), ze kterých se nic neukládá — správci hesel. Výchozí KeePass, KeePassXC,
    /// 1Password, Bitwarden.
    pub exclude_apps: Vec<String>,
    /// Při zamčení počítače (Win+L) smazat nepřipnuté položky. Výchozí true.
    pub clear_on_lock: bool,
    /// Připnuté položky uložit na disk (`%LOCALAPPDATA%\Ledge\clip-pins.json`), ať přežijí restart.
    /// false = i připnuté jen v paměti (a soubor se smaže). Výchozí true.
    pub save_pins: bool,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self { enabled: false, max: DEFAULT_MAX, skip_secrets: true, exclude_apps: default_apps(), clear_on_lock: true, save_pins: true }
    }
}

pub fn validate(c: &ClipboardConfig) -> Result<(), FieldError> {
    if !(1..=MAX_MAX).contains(&c.max) {
        return Err(fe("clipboard.max", trf!("clipboard.max: {} — povoleno 1–{MAX_MAX}.", "clipboard.max: {} — allowed 1–{MAX_MAX}.", c.max)));
    }
    if c.exclude_apps.len() > MAX_EXCLUDE {
        return Err(fe("clipboard.excludeApps", trf!("clipboard.excludeApps: nejvýš {MAX_EXCLUDE} aplikací.", "clipboard.excludeApps: at most {MAX_EXCLUDE} apps.")));
    }
    Ok(())
}

// ---------- historie (čistá logika) ----------

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: u64,
    pub text: String,
    pub at_ms: i64,
    pub pinned: bool,
}

/// Připnutá položka na disku.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    pub text: String,
    #[serde(default)]
    pub at_ms: i64,
}

#[derive(Serialize, Deserialize, Default)]
struct PinsFile {
    #[serde(default)]
    pins: Vec<Pin>,
}

/// Řádek pro paletu: jen náhled, ne celý text (ten zůstává v jádře, `paste` ho vezme podle id).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryView {
    pub id: u64,
    pub preview: String,
    pub chars: usize,
    pub lines: usize,
    pub at_ms: i64,
    pub pinned: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PinErr {
    Missing,
    TooMany,
}

/// Historie: nejnovější první. Strop `max` platí pro nepřipnuté, připnutých je nejvýš `MAX_PINS`,
/// součet textů nejvýš `MAX_TOTAL_BYTES` (vyhazují se nejstarší nepřipnuté, nejnovější zůstává vždy).
#[derive(Debug, Default)]
pub struct History {
    items: Vec<Entry>,
    next_id: u64,
}

impl History {
    pub fn from_pins(pins: Vec<Pin>) -> Self {
        let mut h = History::default();
        for p in pins.into_iter().filter(|p| filter::text_verdict(&p.text, false).is_none()).take(MAX_PINS) {
            if h.items.iter().any(|e| e.text == p.text) {
                continue;
            }
            h.next_id += 1;
            h.items.push(Entry { id: h.next_id, text: p.text, at_ms: p.at_ms, pinned: true });
        }
        h
    }

    /// Nový text ze schránky. Stejný text = posun nahoru s novým časem (id i připnutí zůstanou).
    pub fn push(&mut self, text: &str, now: i64, max: usize) {
        if let Some(i) = self.items.iter().position(|e| e.text == text) {
            let mut e = self.items.remove(i);
            e.at_ms = now;
            self.items.insert(0, e);
        } else {
            self.next_id += 1;
            self.items.insert(0, Entry { id: self.next_id, text: text.to_string(), at_ms: now, pinned: false });
        }
        self.trim(max);
    }

    /// Vyhodí nejstarší nepřipnuté nad stropem počtu / paměti; první (nejnovější) položka zůstane.
    pub fn trim(&mut self, max: usize) {
        let max = max.max(1);
        let mut unpinned = self.items.iter().filter(|e| !e.pinned).count();
        let mut bytes: usize = self.items.iter().map(|e| e.text.len()).sum();
        let mut i = self.items.len();
        while i > 1 && (unpinned > max || bytes > MAX_TOTAL_BYTES) {
            i -= 1;
            if !self.items[i].pinned {
                let e = self.items.remove(i);
                unpinned -= 1;
                bytes -= e.text.len();
            }
        }
    }

    pub fn pin(&mut self, id: u64, on: bool) -> Result<bool, PinErr> {
        let pins = self.items.iter().filter(|e| e.pinned).count();
        let e = self.items.iter_mut().find(|e| e.id == id).ok_or(PinErr::Missing)?;
        if e.pinned == on {
            return Ok(false);
        }
        if on && pins >= MAX_PINS {
            return Err(PinErr::TooMany);
        }
        e.pinned = on;
        Ok(true)
    }

    /// Smaže položku (i připnutou). Vrací, jestli byla připnutá (`None` = nebyla vůbec).
    pub fn remove(&mut self, id: u64) -> Option<bool> {
        let i = self.items.iter().position(|e| e.id == id)?;
        Some(self.items.remove(i).pinned)
    }

    /// „Vymazat historii“ / zamčení: pryč všechno nepřipnuté. Vrací, jestli se něco změnilo.
    pub fn clear_unpinned(&mut self) -> bool {
        let n = self.items.len();
        self.items.retain(|e| e.pinned);
        n != self.items.len()
    }

    pub fn text(&self, id: u64) -> Option<&str> {
        self.items.iter().find(|e| e.id == id).map(|e| e.text.as_str())
    }

    pub fn pins(&self) -> Vec<Pin> {
        self.items.iter().filter(|e| e.pinned).map(|e| Pin { text: e.text.clone(), at_ms: e.at_ms }).collect()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Pro paletu: připnuté nahoře, jinak od nejnovějšího.
    pub fn view(&self) -> Vec<EntryView> {
        let mut out: Vec<EntryView> = self.items.iter().map(entry_view).collect();
        out.sort_by_key(|v| !v.pinned); // stabilní: uvnitř skupin zůstane pořadí podle času
        out
    }
}

fn entry_view(e: &Entry) -> EntryView {
    EntryView { id: e.id, preview: preview(&e.text), chars: e.text.chars().count(), lines: e.text.lines().count().max(1), at_ms: e.at_ms, pinned: e.pinned }
}

/// Jednořádkový náhled: bílé znaky (i konce řádků) sloučené do mezery, nejvýš `PREVIEW_CHARS` znaků + „…“.
pub fn preview(text: &str) -> String {
    let mut out = String::new();
    // slova spojená jednou mezerou (první mezeru `skip` zahodí)
    for (n, ch) in text.split_whitespace().flat_map(|w| std::iter::once(' ').chain(w.chars())).skip(1).enumerate() {
        if n == PREVIEW_CHARS {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

pub fn parse_pins(s: &str) -> Vec<Pin> {
    serde_json::from_str::<PinsFile>(s).map(|f| f.pins).unwrap_or_default()
}

pub fn serialize_pins(pins: &[Pin]) -> String {
    serde_json::to_string_pretty(&serde_json::json!({ "pins": pins })).unwrap_or_default()
}

pub fn load_pins(file: &Path) -> Vec<Pin> {
    std::fs::read_to_string(file).map(|s| parse_pins(&s)).unwrap_or_default()
}

/// Uloží připnuté (`.tmp` + rename). Žádné připnuté = soubor pryč (na disku nic zbytečně neleží).
pub fn save_pins_to(file: &Path, pins: &[Pin]) -> Result<(), String> {
    if pins.is_empty() {
        return match std::fs::remove_file(file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serialize_pins(pins)).and_then(|_| std::fs::rename(&tmp, file)).map_err(|e| crate::errors::io(tr("Připnuté ze schránky nejde uložit", "Can't save the pinned clipboard items"), &e))
}

// ---------- stav za běhu ----------

struct Live {
    hist: History,
    own: Option<Own>,
    cfg: ClipboardConfig,
    pins_file: PathBuf,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn pins_file() -> PathBuf {
    crate::actionrun::local_dir().join(PINS_FILE)
}

fn notify() {
    use tauri::Emitter;
    let _ = crate::state::core().app.emit_to(crate::drawer::LABEL, "cliphist", ());
}

/// Start a každá změna `clipboard` v configu: zapnuto = listener + historie (připnuté z disku),
/// vypnuto = listener pryč a historie z paměti smazaná. `savePins: false` smaže soubor připnutých.
pub fn sync(c: &ClipboardConfig) {
    let file = pins_file();
    if c.enabled {
        {
            let mut g = LIVE.lock();
            let fresh = g.is_none();
            let live = g.get_or_insert_with(|| Live {
                hist: if c.save_pins { History::from_pins(load_pins(&file)) } else { History::default() },
                own: None,
                cfg: c.clone(),
                pins_file: file.clone(),
            });
            live.cfg = c.clone();
            live.hist.trim(c.max as usize);
            if c.save_pins && !fresh {
                save_logged(&live.pins_file, &live.hist.pins());
            }
        }
        win::start();
    } else {
        win::stop();
        if LIVE.lock().take().is_some() {
            notify();
        }
    }
    if !c.save_pins {
        save_logged(&file, &[]);
    }
}

fn save_logged(file: &Path, pins: &[Pin]) {
    if let Err(e) = save_pins_to(file, pins) {
        crate::state::log(&format!("cliphist: {e}"));
    }
}

/// Config pro vlákno listeneru (`None` = funkce vypnutá).
fn active() -> Option<ClipboardConfig> {
    LIVE.lock().as_ref().map(|l| l.cfg.clone())
}

/// Listener: text prošel `pre_verdict` (formáty, aplikace) — zbývá vlastní zápis a `text_verdict`.
fn on_text(seq: u32, text: String) {
    let changed = {
        let mut g = LIVE.lock();
        let Some(live) = g.as_mut() else { return };
        if filter::is_own(live.own.as_ref(), seq, &text) {
            live.own = None;
            return;
        }
        if filter::text_verdict(&text, live.cfg.skip_secrets).is_some() {
            return;
        }
        live.hist.push(&text, crate::state::now_ms(), live.cfg.max as usize);
        true
    };
    if changed {
        notify();
    }
}

/// Zamčení stanice (`WTS_SESSION_LOCK`).
fn on_lock() {
    let changed = {
        let mut g = LIVE.lock();
        match g.as_mut() {
            Some(l) if l.cfg.clear_on_lock => l.hist.clear_unpinned(),
            _ => false,
        }
    };
    if changed {
        notify();
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipList {
    pub enabled: bool,
    pub items: Vec<EntryView>,
}

pub fn list() -> ClipList {
    match LIVE.lock().as_ref() {
        Some(l) => ClipList { enabled: true, items: l.hist.view() },
        None => ClipList { enabled: false, items: vec![] },
    }
}

fn off() -> String {
    tr("Historie schránky je vypnutá.", "Clipboard history is off.").into()
}
fn gone() -> String {
    tr("Položka už v historii není.", "That item is no longer in the history.").into()
}

/// Vloží položku zpět do schránky. Zápis se označí jako vlastní → nový záznam nevznikne a položka
/// zůstane, kde je.
pub fn paste(id: u64) -> Result<(), String> {
    let text = {
        let mut g = LIVE.lock();
        let live = g.as_mut().ok_or_else(off)?;
        let text = live.hist.text(id).ok_or_else(gone)?.to_string();
        live.own = Some(Own { text: text.clone(), seq: None });
        text
    };
    let res = crate::shelf::clip::set_text(&text);
    let seq = win::seq();
    if let Some(live) = LIVE.lock().as_mut()
        && let Some(own) = live.own.as_mut()
    {
        if res.is_ok() {
            own.seq = Some(seq);
        } else {
            live.own = None;
        }
    }
    res
}

pub fn pin(id: u64, on: bool) -> Result<(), String> {
    {
        let mut g = LIVE.lock();
        let live = g.as_mut().ok_or_else(off)?;
        match live.hist.pin(id, on) {
            Ok(false) => return Ok(()),
            Ok(true) => {}
            Err(PinErr::Missing) => return Err(gone()),
            Err(PinErr::TooMany) => return Err(trf!("Připnout jde nejvýš {MAX_PINS} položek.", "At most {MAX_PINS} items can be pinned.")),
        }
        if live.cfg.save_pins {
            save_pins_to(&live.pins_file, &live.hist.pins())?;
        }
    }
    notify();
    Ok(())
}

pub fn remove(id: u64) -> Result<(), String> {
    {
        let mut g = LIVE.lock();
        let live = g.as_mut().ok_or_else(off)?;
        let Some(was_pinned) = live.hist.remove(id) else { return Ok(()) };
        if was_pinned && live.cfg.save_pins {
            save_pins_to(&live.pins_file, &live.hist.pins())?;
        }
    }
    notify();
    Ok(())
}

/// „Vymazat historii“ — nepřipnuté pryč (připnuté se mažou jednotlivě).
pub fn clear() {
    let changed = LIVE.lock().as_mut().is_some_and(|l| l.hist.clear_unpinned());
    if changed {
        notify();
    }
}
