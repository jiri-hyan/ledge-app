//! Okna lístků `note-<klíč>` (vzor tvorby dalšího okna: `webpanel.rs`).
//!
//! - Vznik bez fokusu (`focused(false)` + rovnou viditelné → Windows `SW_SHOWNOACTIVATE`); fokus jen
//!   „Nový lístek“ (výslovný klik, hned se píše). Bez rámu, průhledné, mimo hlavní panel; změnu
//!   velikosti tahem za okraj obstará Tauri (okno bez rámu s `resizable`).
//! - Příznaky z `PinPrefs` → okno v `apply`: navrchu (ne při hře přes celou obrazovku / herním režimu — `sync_dead`; prohlížeč ve F11 lístky nechá),
//!   zámek a sbalení vypnou změnu velikosti, průchozí = `set_ignore_cursor_events` + bez fokusu.
//! - Geometrie z událostí `Moved` / `Resized` do paměti; zápis `note-pins.json` se zpožděním ve vlákně
//!   `pins-save`, které jinak spí na kanálu (žádný časovač, žádný polling). `flush` při ukončení.
//! - Obnova po startu (`restore_all`), zprávy oknům: `pin-prefs` (volby změnil někdo jiný — zkratka,
//!   paleta) a `pin-note` (otisk souboru po přenačtení poznámek; UI si podle něj přečte znovu).

use super::*;
use crate::i18n::tr;
use crate::state::{core, log, mark_dirty};
use crate::threads::notes_watch::notes_root;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Stav lístků v paměti (načte se při první potřebě).
static STATE: Mutex<Option<PinsFile>> = Mutex::new(None);
/// Kanál vlákna `pins-save` (vznikne s prvním zápisem).
static SAVER: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
/// Poslední známé „lístky ustupují“ (`pins_yield`: herní režim / hra, ne prohlížeč ve F11) — lístky
/// přestanou být navrchu.
static DEAD: AtomicBool = AtomicBool::new(false);
/// Poslední známé okno přes celý monitor (zapisuje vlákno `hover`, `set_fs_kind`).
static FS_KIND: AtomicU8 = AtomicU8::new(0);
/// Obnova po startu čeká na konec fullscreenu / herního režimu.
static RESTORE_PENDING: AtomicBool = AtomicBool::new(false);
/// Dvojklik na „Připnout“ = dvě volání naráz → druhé počká a okno už najde.
static OPENING: Mutex<()> = Mutex::new(());
/// Lístky právě vytvořené „Nový lístek“ — UI hned otevře „+ bod“ (vyzvedne `take_fresh`).
static FRESH: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Zpoždění zápisu po posledním pohybu / změně.
const SAVE_DELAY: Duration = Duration::from_millis(400);

fn store_path() -> PathBuf {
    crate::actionrun::local_dir().join(FILE)
}

fn with<R>(f: impl FnOnce(&mut PinsFile) -> R) -> R {
    let mut g = STATE.lock();
    f(g.get_or_insert_with(|| load(&store_path())))
}

/// Zapsat se zpožděním (série pohybů = jeden zápis).
fn persist() {
    let mut g = SAVER.lock();
    if g.as_ref().is_some_and(|tx| tx.send(()).is_ok()) {
        return;
    }
    let (tx, rx) = mpsc::channel::<()>();
    let _ = tx.send(());
    let spawned = std::thread::Builder::new().name("pins-save".into()).spawn(move || {
        while rx.recv().is_ok() {
            while rx.recv_timeout(SAVE_DELAY).is_ok() {}
            flush();
        }
    });
    match spawned {
        Ok(_) => *g = Some(tx),
        Err(_) => flush(),
    }
}

/// Zapsat hned (konec Ledge, vlákno `pins-save`).
pub(crate) fn flush() {
    let Some(snap) = STATE.lock().clone() else { return };
    if let Err(e) = save(&store_path(), &snap) {
        log(&format!("lístky: stav se neuložil ({e})"));
    }
}

fn app() -> &'static AppHandle {
    &core().app
}

/// Otevřená okna lístků (label, okno).
fn windows() -> Vec<(String, WebviewWindow)> {
    app().webview_windows().into_iter().filter(|(l, _)| l.starts_with(LABEL_PREFIX)).collect()
}

pub(crate) fn any_open() -> bool {
    crate::state::CORE.get().is_some_and(|c| c.app.webview_windows().keys().any(|l| l.starts_with(LABEL_PREFIX)))
}

/// Cesta poznámky otevřeného lístku.
pub(crate) fn path_of(label: &str) -> Option<String> {
    STATE.lock().as_ref()?.by_label(label).map(|p| p.path.clone())
}

pub(crate) fn prefs_of(label: &str) -> Option<PinPrefs> {
    STATE.lock().as_ref()?.by_label(label).map(|p| p.prefs)
}

/// Je lístek čerstvě vytvořený („Nový lístek“)? Jednou — podruhé už ne.
pub(crate) fn take_fresh(label: &str) -> bool {
    let mut g = FRESH.lock();
    let had = g.iter().any(|l| l == label);
    g.retain(|l| l != label);
    had
}

pub(crate) fn hint_seen() -> bool {
    with(|s| s.hint_seen)
}

pub(crate) fn set_hint_seen() {
    with(|s| s.hint_seen = true);
    persist();
}

/// Lístky v snapshotu šuplíku (jen paměť): které poznámky jsou připnuté a jestli průchozí.
#[derive(serde::Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PinsView {
    pub open: Vec<PinRef>,
    /// zkratka průchozího režimu (`hotkeys.notePassthrough`), prázdná = žádná
    pub key: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PinRef {
    pub path: String,
    pub through: bool,
}

pub(crate) fn view(key: &str) -> PinsView {
    let labels: Vec<String> = match crate::state::CORE.get() {
        Some(c) => c.app.webview_windows().into_keys().filter(|l| l.starts_with(LABEL_PREFIX)).collect(),
        None => Vec::new(),
    };
    let g = STATE.lock();
    let open = g
        .as_ref()
        .map(|s| s.pins.iter().filter(|p| labels.contains(&label_for(&p.path))).map(|p| PinRef { path: p.path.clone(), through: p.prefs.through }).collect())
        .unwrap_or_default();
    PinsView { open, key: key.to_string() }
}

/// Po otevření prvního / zavření posledního lístku: zkratka průchozího režimu se (od)registruje,
/// šuplík dostane nový stav. Mimo vlákno událostí okna (registrace zkratek tam nepatří).
fn changed_set() {
    std::thread::spawn(|| {
        crate::hotkeys::sync_hotkeys();
        mark_dirty();
    });
}

/// Monitor proužku jako `Mon` + mezera pod proužkem (fyzicky) — sem jdou nové lístky.
fn fallback() -> (Mon, i32) {
    let c = core();
    let mon = c.monitor();
    let scale = mon.dpi as f64 / 96.0;
    let r = Area::from(mon.rect);
    let work = Area::from(mon.work);
    let strip_h = c.strip_logical(&mon).1;
    let gap = ((r.t + ((strip_h + 12.0) * scale).round() as i32) - work.t).max((12.0 * scale).round() as i32);
    (Mon { work, scale }, gap)
}

fn monitors() -> Vec<Mon> {
    crate::screen::monitors().iter().map(|m| Mon { work: Area::from(m.work), scale: m.dpi as f64 / 96.0 }).collect()
}

fn title_of(path: &str) -> String {
    let t = core().notes.lock().all.iter().find(|n| same_note(&n.path, path)).map(|n| n.title.clone()).unwrap_or_default();
    let t = if t.trim().is_empty() { Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default() } else { t };
    trf!("Lístek: {t}", "Sticky: {t}")
}

/// Připne poznámku na plochu (nebo najde už otevřený lístek). `focus` = výslovná tvorba nového
/// lístku (hned se do něj píše); připnutí existující poznámky fokus prohlížeči nebere.
pub(crate) fn open(path: &str, focus: bool) -> Result<(), String> {
    let c = core();
    if yielding(c) {
        return Err(tr("Hra / herní režim — lístek se teď neotevře.", "Game / game mode — the sticky won't open now.").into());
    }
    let root = notes_root().ok_or_else(|| tr("Poznámky jsou vypnuté v Nastavení.", "Notes are off in Settings."))?;
    crate::notes::check_note_path(&root, Path::new(path))?;
    let _one = OPENING.lock();
    let label = label_for(path);
    if let Some(w) = app().get_webview_window(&label) {
        if focus {
            let _ = w.set_focus();
        }
        return Ok(());
    }
    let pin = with(|s| s.ensure(path).clone());
    let index = windows().len();
    let (fb, gap) = fallback();
    let place = restore(&pin, &monitors(), &fb, gap, index);
    // Výchozí logická pozice pro tvorbu (tao ji přepočte podle monitoru); přesně ji hned nastaví
    // `set_position` ve fyzických px. Okno je průhledné a stránka se ještě nenačetla → nic neblikne.
    let w = WebviewWindowBuilder::new(app(), &label, WebviewUrl::App("pin.html".into()))
        .title(title_of(path))
        .visible(true)
        .focused(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(pin.prefs.on_top && !yielding(c))
        .resizable(resizable(&pin.prefs))
        .min_inner_size(MIN_W, if pin.prefs.collapsed { BAR_H } else { MIN_H })
        .inner_size(place.w as f64 / fb.scale, place.h as f64 / fb.scale)
        .position(place.x as f64 / fb.scale, place.y as f64 / fb.scale)
        // stejné argumenty + výchozí data dir jako šuplík → jeden proces prohlížeče
        .additional_browser_args(crate::drawer::BROWSER_ARGS)
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| trf!("Lístek se neotevřel: {e}", "The sticky didn't open: {e}"))?;
    let _ = w.set_position(PhysicalPosition::new(place.x, place.y));
    let _ = w.set_size(PhysicalSize::new(place.w, place.h));
    apply(&w, &pin.prefs);
    with(|s| {
        let p = s.ensure(path);
        (p.x, p.y) = (Some(place.x), Some(place.y));
    });
    watch(&w, &label);
    if focus {
        FRESH.lock().push(label.clone());
        let _ = w.set_focus();
    }
    persist();
    changed_set();
    Ok(())
}

fn resizable(p: &PinPrefs) -> bool {
    !p.locked && !p.through && !p.collapsed
}

/// Volby → okno (bez velikosti; sbalení řeší `set`).
fn apply(w: &WebviewWindow, p: &PinPrefs) {
    let _ = w.set_always_on_top(p.on_top && !DEAD.load(Ordering::Relaxed));
    let _ = w.set_resizable(resizable(p));
    let _ = w.set_ignore_cursor_events(p.through);
    // průchozí lístek nesmí dostat fokus (Alt+Tab, klik do jeho místa jde na stránku pod ním)
    let _ = w.set_focusable(!p.through);
}

/// Pohyb a změna velikosti do paměti, zavření křížkem Windows (Alt+F4) = odepnout.
fn watch(w: &WebviewWindow, label: &str) {
    let (label, win) = (label.to_string(), w.clone());
    w.on_window_event(move |e| match e {
        tauri::WindowEvent::Moved(p) => {
            with(|s| {
                if let Some(pin) = s.by_label_mut(&label) {
                    (pin.x, pin.y) = (Some(p.x), Some(p.y));
                }
            });
            persist();
        }
        tauri::WindowEvent::Resized(sz) => {
            let scale = win.scale_factor().unwrap_or(1.0).max(0.1);
            if sz.width == 0 || sz.height == 0 || win.is_minimized().unwrap_or(false) {
                return;
            }
            with(|s| {
                if let Some(pin) = s.by_label_mut(&label) {
                    pin.w = (sz.width as f64 / scale).clamp(MIN_W, MAX_W);
                    if !pin.prefs.collapsed {
                        pin.h = (sz.height as f64 / scale).clamp(MIN_H, MAX_H);
                    }
                }
            });
            persist();
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            if let Some(path) = path_of(&label) {
                with(|s| s.remove(&path));
                persist();
            }
        }
        tauri::WindowEvent::Destroyed => changed_set(),
        _ => {}
    });
}

/// Odepne lístek (poznámka zůstane) a zavře jeho okno.
pub(crate) fn close(path: &str) {
    let label = label_for(path);
    with(|s| s.remove(path));
    persist();
    if let Some(w) = app().get_webview_window(&label) {
        let _ = w.destroy();
    }
    changed_set();
}

pub(crate) fn is_open(path: &str) -> bool {
    app().get_webview_window(&label_for(path)).is_some()
}

/// Změna voleb lístku `label` (UI lístku, šuplík, zkratka). Vrací nové volby; okno je dostane i
/// událostí `pin-prefs`, kdyby změna nepřišla od něj.
pub(crate) fn set(label: &str, patch: &PinPatch) -> Result<PinPrefs, String> {
    let w = app().get_webview_window(label).ok_or_else(|| tr("Lístek už není otevřený.", "The sticky is no longer open."))?;
    let (old, new, size) = with(|s| {
        let pin = s.by_label_mut(label)?;
        let old = pin.prefs;
        pin.prefs = patch.apply(old);
        Some((old, pin.prefs, (pin.w, pin.h)))
    })
    .ok_or_else(|| tr("Lístek už není připnutý.", "The sticky is no longer pinned."))?;
    if old.collapsed != new.collapsed {
        if new.collapsed {
            let _ = w.set_min_size(None::<LogicalSize<f64>>);
            let _ = w.set_size(LogicalSize::new(size.0, BAR_H));
        } else {
            let _ = w.set_size(LogicalSize::new(size.0, size.1));
            let _ = w.set_min_size(Some(LogicalSize::new(MIN_W, MIN_H)));
        }
    }
    apply(&w, &new);
    let _ = w.emit_to(label, "pin-prefs", new);
    persist();
    if old.through != new.through {
        mark_dirty();
    }
    Ok(new)
}

/// Zkratka / paleta: je-li některý lístek průchozí, vypne průchozí u všech; jinak ho zapne všem.
/// Vrací nový stav (`None` = žádný lístek není otevřený).
pub(crate) fn toggle_through_all() -> Option<bool> {
    let open: Vec<String> = windows().into_iter().map(|(l, _)| l).collect();
    if open.is_empty() {
        return None;
    }
    let any = with(|s| s.pins.iter().any(|p| p.prefs.through && open.contains(&label_for(&p.path))));
    let patch = PinPatch { through: Some(!any), ..Default::default() };
    for l in &open {
        if let Err(e) = set(l, &patch) {
            log(&format!("lístek {l}: {e}"));
        }
    }
    Some(!any)
}

const KINDS: [FsKind; 5] = [FsKind::None, FsKind::Exclusive, FsKind::CoverBrowser, FsKind::CoverOwn, FsKind::CoverOther];

/// Zapíše, co leží přes monitor; vrací, jestli se to změnilo (pak má volající zavolat `sync_dead`).
pub(crate) fn set_fs_kind(k: FsKind) -> bool {
    let v = KINDS.iter().position(|x| *x == k).unwrap_or(0) as u8;
    FS_KIND.swap(v, Ordering::Relaxed) != v
}

/// Mají teď lístky ustoupit (herní režim, hra / prezentace přes celou obrazovku)? Prohlížeč ve F11 ne.
fn yielding(c: &crate::state::Core) -> bool {
    let kind = KINDS.get(FS_KIND.load(Ordering::Relaxed) as usize).copied().unwrap_or(FsKind::None);
    // `fullscreen` je pravda o monitoru; druh doplňuje jen to, co leží přes něj
    let kind = if c.fullscreen.load(Ordering::Relaxed) { kind } else { FsKind::None };
    pins_yield(c.game_mode.load(Ordering::SeqCst), kind)
}

/// Fullscreen / herní režim / okno v popředí přes celý monitor se změnilo (vlákno `hover`,
/// `gamemode::set`): lístky „Vždy navrchu“ ustoupí hře a pak se vrátí (prohlížeč ve F11 je nechá);
/// obnova po startu, která čekala, doběhne.
pub(crate) fn sync_dead() {
    let Some(c) = crate::state::CORE.get() else { return };
    let dead = yielding(c);
    if DEAD.swap(dead, Ordering::Relaxed) == dead {
        return;
    }
    for (l, w) in windows() {
        if let Some(p) = prefs_of(&l) {
            let _ = w.set_always_on_top(p.on_top && !dead);
        }
    }
    if !dead && RESTORE_PENDING.swap(false, Ordering::Relaxed) {
        std::thread::spawn(restore_all);
    }
}

/// Po startu Ledge: připnuté lístky zpátky na místa (mimo obrazovku → na monitor proužku).
pub(crate) fn restore_all() {
    let Some(c) = crate::state::CORE.get() else { return };
    if !c.cfg.read().notes.enabled {
        return;
    }
    if yielding(c) {
        RESTORE_PENDING.store(true, Ordering::Relaxed);
        return;
    }
    let paths: Vec<String> = with(|s| s.pins.iter().map(|p| p.path.clone()).collect());
    for p in paths {
        if let Err(e) = open(&p, false) {
            log(&format!("lístek {p}: {e}"));
        }
    }
}

/// Po přenačtení poznámek (`reload_notes`): každému lístku otisk jeho souboru (`""` = soubor zmizel).
/// UI ho porovná se svým a jen při rozdílu si poznámku přečte znovu.
pub(crate) fn on_notes_changed() {
    if !any_open() {
        return;
    }
    for (l, w) in windows() {
        let Some(path) = path_of(&l) else { continue };
        let hash = std::fs::read(&path).map(|b| crate::notes::edit::hash(&b)).unwrap_or_default();
        let _ = w.emit_to(l.as_str(), "pin-note", hash);
    }
}
