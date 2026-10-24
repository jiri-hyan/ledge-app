//! Okna widgetů `desk-<id>` (vzor: `pins/win.rs` — lístky na plochu).
//!
//! - Vznik bez fokusu (`focused(false)` + rovnou viditelné → `SW_SHOWNOACTIVATE`), bez rámu, průhledné, mimo
//!   hlavní panel, stejné `BROWSER_ARGS` a data dir jako šuplík → stejný proces prohlížeče i renderer
//!   (`--renderer-process-limit=1`).
//! - Vrstva: Navrchu = `always_on_top` (ve hře ne), Normální, Jen na ploše = `always_on_bottom`. Zámek a
//!   průchozí vypnou posun i změnu velikosti; průchozí = `set_ignore_cursor_events` + bez fokusu.
//! - Hra přes celou obrazovku / herní režim (`sync_yield` ze stejných míst jako `pins::win::sync_dead`):
//!   widget se schová (`SW_HIDE`, WebView uspaný: nízký cíl paměti + `TrySuspend`) a pak se vrátí bez
//!   aktivace. Schovaný widget nedostává data (`live.rs`).
//! - Geometrie z událostí `Moved` / `Resized`; zápis `desk.json` se zpožděním ve vlákně `desk-save`, které
//!   spí na kanálu; po přesunu přichycení k okrajům (`snap`), až když je pustěné tlačítko myši.
//! - Odpojený monitor: okno stojí na monitoru proužku (`DISPLACED`), domov v souboru zůstane; po připojení
//!   (`on_monitors` z vlákna `hover`, které monitory stejně čte co 3 s) se vrátí domů.

use super::*;
use super::save;
use crate::i18n::tr;
use crate::state::{core, log, mark_dirty};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Stav widgetů v paměti (načte se při první potřebě).
pub(super) static STATE: Mutex<Option<DeskFile>> = Mutex::new(None);
/// Poslední známé „ustoupit hře“ (`pins::pins_yield`).
static YIELD: AtomicBool = AtomicBool::new(false);
/// Co leží přes monitor (zapisuje vlákno `hover`, stejná hodnota jako u lístků).
static FS_KIND: AtomicU8 = AtomicU8::new(0);
/// Všechny widgety schované (paleta / Nastavení) — jen do konce běhu.
static HIDDEN_ALL: AtomicBool = AtomicBool::new(false);
static RESTORE_PENDING: AtomicBool = AtomicBool::new(false);
static OPENING: Mutex<()> = Mutex::new(());
/// Widgety mimo domov (monitor chybí): id → kam jsme okno dali (fyzicky).
static DISPLACED: Mutex<Option<HashMap<String, (i32, i32)>>> = Mutex::new(None);
/// Okna, která jsou teď vidět (label) — `live.rs` posílá data jen jim.
static SHOWN: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Widgety dočasně zvětšené kvůli otevřeným volbám (⋯) — jejich změna velikosti se neukládá.
static GROWN: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Kolik místa potřebují volby widgetu (logické px).
const MENU_W: f64 = 280.0;
const MENU_H: f64 = 330.0;

pub(super) fn store_path() -> PathBuf {
    crate::actionrun::local_dir().join(FILE)
}

pub(super) fn with<R>(f: impl FnOnce(&mut DeskFile) -> R) -> R {
    let mut g = STATE.lock();
    f(g.get_or_insert_with(|| load(&store_path())))
}

fn displaced<R>(f: impl FnOnce(&mut HashMap<String, (i32, i32)>) -> R) -> R {
    f(DISPLACED.lock().get_or_insert_with(HashMap::new))
}

/// Kopie widgetu podle id.
pub(crate) fn widget(id: &str) -> Option<DeskWidget> {
    with(|s| s.by_id(id).cloned())
}

pub(super) fn app() -> &'static AppHandle {
    &core().app
}

pub(super) fn windows() -> Vec<(String, WebviewWindow)> {
    app().webview_windows().into_iter().filter(|(l, _)| l.starts_with(LABEL_PREFIX)).collect()
}

/// Monitor proužku + mezera pod proužkem (fyzicky) — sem jdou nové widgety (kopie `pins::win::fallback`).
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

pub(super) fn monitors() -> Vec<Mon> {
    crate::screen::monitors().iter().map(|m| Mon { work: Area::from(m.work), scale: m.dpi as f64 / 96.0 }).collect()
}

fn hwnd(w: &WebviewWindow) -> Option<windows::Win32::Foundation::HWND> {
    w.hwnd().ok().map(|h| windows::Win32::Foundation::HWND(h.0 as *mut _))
}

/// Ukázat bez aktivace / schovat. `show()` z Tauri by okno aktivovalo (vzalo fokus hře nebo editoru).
fn set_visible(w: &WebviewWindow, on: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow};
    if let Some(h) = hwnd(w) {
        // SAFETY: platné HWND našeho okna
        unsafe {
            let _ = ShowWindow(h, if on { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }
    dormant(w, !on);
}

/// Schovaný widget: WebView2 nízký cíl paměti + uspání (žádné časovače stránky); viditelný zpět
/// (vzor `Drawer::set_dormant`).
fn dormant(w: &WebviewWindow, sleep: bool) {
    let _ = w.with_webview(move |wv| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::*;
        use windows::core::Interface;
        let ctrl = wv.controller();
        let Ok(core) = ctrl.CoreWebView2() else { return };
        if let Ok(c19) = core.cast::<ICoreWebView2_19>() {
            let level = if sleep { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL };
            let _ = c19.SetMemoryUsageTargetLevel(level);
        }
        if let Ok(c3) = core.cast::<ICoreWebView2_3>() {
            if sleep {
                let _ = ctrl.SetIsVisible(false);
                let h = webview2_com::TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(())));
                let _ = c3.TrySuspend(&h);
            } else {
                let _ = c3.Resume();
                let _ = ctrl.SetIsVisible(true);
            }
        }
    });
}

const KINDS: [pins::FsKind; 5] = [pins::FsKind::None, pins::FsKind::Exclusive, pins::FsKind::CoverBrowser, pins::FsKind::CoverOwn, pins::FsKind::CoverOther];

/// Vlákno `hover`: co leží přes monitor (stejně jako `pins::win::set_fs_kind`).
pub(crate) fn set_fs_kind(k: pins::FsKind) {
    let v = KINDS.iter().position(|x| *x == k).unwrap_or(0) as u8;
    FS_KIND.store(v, Ordering::Relaxed);
}

/// Mají widgety ustoupit hře? Stejné pravidlo jako lístky (`pins::pins_yield`): herní režim, D3D
/// fullscreen, okno jiné aplikace přes celý monitor; prohlížeč ve F11 a vlastní okno ne.
fn yielding() -> bool {
    let Some(c) = crate::state::CORE.get() else { return false };
    let kind = KINDS.get(FS_KIND.load(Ordering::Relaxed) as usize).copied().unwrap_or(pins::FsKind::None);
    let kind = if c.fullscreen.load(Ordering::Relaxed) { kind } else { pins::FsKind::None };
    pins::pins_yield(c.game_mode.load(Ordering::SeqCst), kind)
}

/// Volby → okno (bez polohy).
fn apply(win: &WebviewWindow, w: &DeskWidget, y: bool) {
    let top = on_top(w, y);
    let bottom = w.layer == Layer::Desktop;
    // nejdřív vypnout, pak zapnout — obojí naráz Windows neumí
    if !top {
        let _ = win.set_always_on_top(false);
    }
    if !bottom {
        let _ = win.set_always_on_bottom(false);
    }
    if top {
        let _ = win.set_always_on_top(true);
    }
    if bottom {
        let _ = win.set_always_on_bottom(true);
    }
    let _ = win.set_resizable(movable(w));
    let _ = win.set_ignore_cursor_events(w.through);
    let _ = win.set_focusable(!w.through);
}

fn mark_shown(label: &str, on: bool) {
    let mut g = SHOWN.lock();
    g.retain(|l| l != label);
    if on {
        g.push(label.to_string());
    }
}

/// Labely oken, která jsou vidět (data jen jim).
pub(crate) fn shown_labels() -> Vec<String> {
    SHOWN.lock().clone()
}

/// Viditelná okna s druhem widgetu (label, druh): každé okno dostane jen data svého druhu (`live::tick`).
pub(crate) fn shown_windows() -> Vec<(String, Kind)> {
    let labels = shown_labels();
    if labels.is_empty() {
        return vec![];
    }
    with(|s| s.widgets.iter().filter(|w| labels.contains(&w.label())).filter_map(|w| w.kind().map(|k| (w.label(), k))).collect())
}

/// Widgety ze skriptů na ploše (pro `widgets::live::placed`: snapshot je ukáže, plánovač je spustí).
pub(crate) fn script_ids(only_shown: bool) -> Vec<String> {
    if STATE.lock().is_none() {
        return vec![];
    }
    let shown = shown_labels();
    with(|s| s.widgets.iter().filter(|w| !only_shown || shown.contains(&w.label())).filter_map(|w| match w.kind() {
        Some(Kind::Script(id)) => Some(id),
        _ => None,
    }).collect())
}

/// Připnout druh na plochu (⋯ karty, Ctrl+K, MCP). Vrací id widgetu.
pub(crate) fn pin(kind: &Kind) -> Result<String, String> {
    if yielding() {
        return Err(tr("Hra / herní režim — widget se teď neotevře.", "Game / game mode — the widget won't open now.").into());
    }
    let d = core().cfg.read().desk.clone();
    let (id, _) = with(|s| s.add(kind, &d)).map_err(|e| match e {
        AddErr::Full => trf!(
            "Na ploše může být nejvýš {} widgetů — každý je samostatné okno a bere paměť. Nějaký odepni (⋯ na widgetu → Odepnout) a zkus to znovu.",
            "At most {} widgets fit on the desktop — each one is its own window and uses memory. Unpin one (⋯ on the widget → Unpin) and try again.",
            MAX_WIDGETS
        ),
        AddErr::Unknown => tr("Tohle na plochu připnout nejde.", "This can't be pinned to the desktop.").into(),
    })?;
    HIDDEN_ALL.store(false, Ordering::Relaxed);
    open(&id)?;
    if matches!(kind, Kind::Script(_)) {
        crate::widgets::live::wake();
    }
    Ok(id)
}

/// Otevře okno widgetu `id` (nebo nechá už otevřené).
pub(crate) fn open(id: &str) -> Result<(), String> {
    let _one = OPENING.lock();
    let label = label_for(id);
    if app().get_webview_window(&label).is_some() {
        return Ok(());
    }
    let w = widget(id).ok_or_else(|| tr("Widget už není na ploše.", "The widget is no longer on the desktop."))?;
    let y = yielding();
    let visible = shown(&w, y, HIDDEN_ALL.load(Ordering::Relaxed));
    let index = windows().len();
    let (fb, gap) = fallback();
    let mons = monitors();
    let (place, away) = plan_place(&w, &mons, &fb, gap, index);
    let win = WebviewWindowBuilder::new(app(), &label, WebviewUrl::App("desk.html".into()))
        .title(title_of(&w))
        .visible(visible)
        .focused(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(on_top(&w, y))
        .always_on_bottom(w.layer == Layer::Desktop)
        .resizable(movable(&w))
        .min_inner_size(MIN_W, MIN_H)
        .inner_size(place.w as f64 / fb.scale, place.h as f64 / fb.scale)
        .position(place.x as f64 / fb.scale, place.y as f64 / fb.scale)
        // stejné argumenty + výchozí data dir jako šuplík → jeden proces prohlížeče i renderer
        .additional_browser_args(crate::drawer::BROWSER_ARGS)
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| crate::errors::any(tr("Widget se neotevřel", "The widget didn't open"), &e))?;
    let _ = win.set_position(PhysicalPosition::new(place.x, place.y));
    let _ = win.set_size(PhysicalSize::new(place.w, place.h));
    apply(&win, &w, y);
    if away {
        displaced(|d| d.insert(id.to_string(), (place.x, place.y)));
    } else {
        with(|s| {
            if let Some(x) = s.by_id_mut(id) {
                (x.x, x.y) = (Some(place.x), Some(place.y));
            }
        });
    }
    if !visible {
        dormant(&win, true);
    }
    mark_shown(&label, visible);
    watch(&win, id);
    super::live::invalidate();
    save::persist();
    mark_dirty();
    Ok(())
}

fn title_of(w: &DeskWidget) -> String {
    match w.kind() {
        Some(Kind::Limits) => tr("Ledge: limity Clauda", "Ledge: Claude limits").into(),
        Some(Kind::Waiting) => tr("Ledge: čeká na tebe", "Ledge: waiting for you").into(),
        Some(Kind::Dev) => tr("Ledge: dev servery", "Ledge: dev servers").into(),
        Some(Kind::Agenda) => tr("Ledge: agenda dne", "Ledge: today's agenda").into(),
        Some(Kind::Week) => tr("Ledge: týden", "Ledge: week").into(),
        Some(Kind::Clock) => tr("Ledge: hodiny", "Ledge: clock").into(),
        Some(Kind::Usage) => tr("Ledge: Claude dnes", "Ledge: Claude today").into(),
        Some(Kind::Script(id)) => format!("Ledge: {}", crate::widgets::plan::file_of(&id).unwrap_or("widget")),
        None => "Ledge".into(),
    }
}

/// Pohyb a změna velikosti do paměti, Alt+F4 = odepnout.
fn watch(win: &WebviewWindow, id: &str) {
    let (id, w2) = (id.to_string(), win.clone());
    win.on_window_event(move |e| match e {
        tauri::WindowEvent::Moved(p) => moved(&id, p.x, p.y),
        tauri::WindowEvent::Resized(sz) => {
            if sz.width == 0 || sz.height == 0 || w2.is_minimized().unwrap_or(false) || GROWN.lock().contains(&id) {
                return;
            }
            let scale = w2.scale_factor().unwrap_or(1.0).max(0.1);
            let (lw, lh) = (sz.width as f64 / scale, sz.height as f64 / scale);
            with(|s| {
                if let Some(x) = s.by_id_mut(&id) {
                    if x.compact {
                        (x.cw, x.ch) = (lw.clamp(MIN_W, MAX_W), lh.clamp(MIN_H, MAX_H));
                    } else {
                        (x.w, x.h) = (lw.clamp(MIN_W, MAX_W), lh.clamp(MIN_H, MAX_H));
                    }
                }
            });
            save::persist();
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            with(|s| s.remove(&id));
            save::persist();
        }
        tauri::WindowEvent::Destroyed => {
            mark_shown(&label_for(&id), false);
            mark_dirty();
        }
        _ => {}
    });
}

/// Okno se posunulo: uživatel (nový domov), my (umístění mimo domov — ignorovat), nebo Windows po odpojení
/// monitoru (domov už na žádném monitoru neleží → okno je mimo domov, domov zůstane).
fn moved(id: &str, x: i32, y: i32) {
    if displaced(|d| d.get(id) == Some(&(x, y))) {
        return;
    }
    let mons = monitors();
    // šířka lišty jen jako odhad (pozice je fyzická) — měřítko 1 stačí, monitor se tu nečte
    let home_ok = with(|s| s.by_id(id).is_none_or(|w| w.x.is_none() || home_on_screen(w, &mons, 1.0)));
    if !home_ok {
        displaced(|d| d.insert(id.to_string(), (x, y)));
        return;
    }
    displaced(|d| d.remove(id));
    with(|s| {
        if let Some(w) = s.by_id_mut(id) {
            (w.x, w.y) = (Some(x), Some(y));
        }
    });
    save::mark_moved(id);
}

/// Volby (⋯) se nevejdou do malého widgetu: okno se na dobu voleb zvětší (neuloží se) a pak vrátí.
pub(crate) fn menu(id: &str, open: bool) {
    let Some(w) = widget(id) else { return };
    let Some(win) = app().get_webview_window(&label_for(id)) else { return };
    let (lw, lh) = w.size_now();
    if open {
        let mut g = GROWN.lock();
        if !g.iter().any(|x| x == id) {
            g.push(id.to_string());
        }
        drop(g);
        let _ = win.set_size(tauri::LogicalSize::new(lw.max(MENU_W), lh.max(MENU_H)));
    } else {
        let _ = win.set_size(tauri::LogicalSize::new(lw, lh));
        // Resized z vlastního zmenšení dorazí asynchronně — přestat ignorovat až po něm
        // Jednorázové krátké vlákno záměrně zůstává: `tokio` není přímá závislost (`Cargo.toml`) a `tokio::time` by
        // znamenalo novou závislost / feature kvůli jednomu pokynu po kliknutí na zavření voleb (ne v klidu).
        let id = id.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            GROWN.lock().retain(|x| *x != id);
        });
    }
}

/// Odepnout widget (zavře okno, zmizí ze souboru).
pub(crate) fn close(id: &str) {
    with(|s| s.remove(id));
    displaced(|d| d.remove(id));
    save::persist();
    if let Some(w) = app().get_webview_window(&label_for(id)) {
        let _ = w.destroy();
    }
    mark_shown(&label_for(id), false);
    mark_dirty();
}

/// Změna voleb widgetu `id` (okno widgetu, Nastavení). Vrací nový stav; okno ho dostane i událostí
/// `desk-prefs`.
pub(crate) fn set(id: &str, patch: &Patch) -> Result<DeskWidget, String> {
    let (old, new) = with(|s| {
        let w = s.by_id_mut(id)?;
        let old = w.clone();
        *w = patch.apply(&old);
        Some((old, w.clone()))
    })
    .ok_or_else(|| tr("Widget už není na ploše.", "The widget is no longer on the desktop."))?;
    let label = label_for(id);
    if let Some(win) = app().get_webview_window(&label) {
        if old.compact != new.compact {
            let (w, h) = new.size_now();
            let _ = win.set_size(tauri::LogicalSize::new(w, h));
        }
        let y = yielding();
        apply(&win, &new, y);
        let vis = shown(&new, y, HIDDEN_ALL.load(Ordering::Relaxed));
        if vis != shown(&old, y, HIDDEN_ALL.load(Ordering::Relaxed)) {
            set_visible(&win, vis);
            mark_shown(&label, vis);
            super::live::invalidate();
        }
        let _ = win.emit_to(label.as_str(), "desk-prefs", &new);
    }
    save::persist();
    mark_dirty();
    Ok(new)
}

/// Schovat / ukázat všechny widgety (paleta, Nastavení). `None` = přepnout. Vrací nový stav „schované“.
pub(crate) fn hide_all(hide: Option<bool>) -> bool {
    let now = hide.unwrap_or(!HIDDEN_ALL.load(Ordering::Relaxed));
    HIDDEN_ALL.store(now, Ordering::Relaxed);
    refresh_visibility();
    mark_dirty();
    now
}

/// Projde okna a srovná viditelnost a vrstvu se stavem hry / „schovat vše“.
fn refresh_visibility() {
    let y = yielding();
    let all = HIDDEN_ALL.load(Ordering::Relaxed);
    for (l, win) in windows() {
        let Some(w) = id_of(&l).and_then(widget) else { continue };
        let vis = shown(&w, y, all);
        let was = SHOWN.lock().contains(&l);
        apply(&win, &w, y);
        if vis != was {
            set_visible(&win, vis);
            mark_shown(&l, vis);
        }
    }
    super::live::invalidate();
}

/// Fullscreen / herní režim se změnil (vlákno `hover`, `gamemode::set`): widgety se schovají / vrátí.
pub(crate) fn sync_yield() {
    if crate::state::CORE.get().is_none() {
        return;
    }
    let y = yielding();
    if YIELD.swap(y, Ordering::Relaxed) != y {
        refresh_visibility();
    }
    // i beze změny: Ledge mohl startovat v herním režimu (YIELD je zpočátku false) a obnova čeká
    if !y && RESTORE_PENDING.swap(false, Ordering::Relaxed) {
        std::thread::spawn(restore_all);
    }
}

/// Po startu Ledge: widgety zpátky na místa (domov mimo obrazovku → monitor proužku, počká na návrat).
pub(crate) fn restore_all() {
    if crate::state::CORE.get().is_none() {
        return;
    }
    if yielding() {
        RESTORE_PENDING.store(true, Ordering::Relaxed);
        return;
    }
    let ids: Vec<String> = with(|s| s.widgets.iter().map(|w| w.id.clone()).collect());
    for id in &ids {
        if let Err(e) = open(id) {
            log(&format!("widget {id}: {e}"));
        }
    }
    if ids.iter().any(|id| widget(id).is_some_and(|w| matches!(w.kind(), Some(Kind::Script(_))))) {
        crate::widgets::live::wake();
    }
}

/// Změnila se sestava monitorů (vlákno `hover`): widgety mimo domov, jejichž monitor je zpět, jdou domů.
pub(crate) fn on_monitors() {
    let back: Vec<String> = displaced(|d| d.keys().cloned().collect());
    if back.is_empty() {
        return;
    }
    let mons = monitors();
    let (fb, _) = fallback();
    for id in back {
        let Some(w) = widget(&id) else {
            displaced(|d| d.remove(&id));
            continue;
        };
        if !home_on_screen(&w, &mons, fb.scale) {
            continue;
        }
        let (place, _) = plan_place(&w, &mons, &fb, 0, 0);
        displaced(|d| d.remove(&id));
        if let Some(win) = app().get_webview_window(&label_for(&id)) {
            let _ = win.set_position(PhysicalPosition::new(place.x, place.y));
            let _ = win.set_size(PhysicalSize::new(place.w, place.h));
        }
    }
}

/// Uložit dnešní rozmístění jako profil.
pub(crate) fn profile_save(name: &str) -> Result<String, String> {
    let r = with(|s| s.save_profile(name)).map_err(|e| match e {
        AddErr::Full => trf!("Profilů může být nejvýš {} — nějaký smaž.", "At most {} profiles — delete one first.", MAX_PROFILES),
        AddErr::Unknown => tr("Profil potřebuje jméno.", "The profile needs a name.").into(),
    })?;
    save::persist();
    mark_dirty();
    Ok(r)
}

/// Obnovit profil jedním klikem: zavřít, co v něm není, otevřít a přesunout zbytek.
pub(crate) fn profile_apply(name: &str) -> Result<(), String> {
    if yielding() {
        return Err(tr("Hra / herní režim — profil se obnoví po hře.", "Game / game mode — the profile will apply after the game.").into());
    }
    let diff = with(|s| s.apply_profile(name)).ok_or_else(|| tr("Takový profil není.", "There's no such profile."))?;
    for id in &diff.close {
        displaced(|d| d.remove(id));
        if let Some(w) = app().get_webview_window(&label_for(id)) {
            let _ = w.destroy();
        }
        mark_shown(&label_for(id), false);
    }
    let mons = monitors();
    let (fb, gap) = fallback();
    let y = yielding();
    for (i, id) in diff.keep.iter().enumerate() {
        let (Some(w), Some(win)) = (widget(id), app().get_webview_window(&label_for(id))) else { continue };
        let (place, away) = plan_place(&w, &mons, &fb, gap, i);
        displaced(|d| if away { d.insert(id.clone(), (place.x, place.y)); } else { d.remove(id); });
        let _ = win.set_position(PhysicalPosition::new(place.x, place.y));
        let _ = win.set_size(PhysicalSize::new(place.w, place.h));
        apply(&win, &w, y);
        let _ = win.emit_to(label_for(id).as_str(), "desk-prefs", &w);
    }
    for id in &diff.open {
        if let Err(e) = open(id) {
            log(&format!("widget {id}: {e}"));
        }
    }
    crate::widgets::live::wake();
    super::live::invalidate();
    save::persist();
    mark_dirty();
    Ok(())
}

pub(crate) fn profile_delete(name: &str) -> bool {
    let ok = with(|s| s.delete_profile(name));
    save::persist();
    mark_dirty();
    ok
}

/// Widgety pro snapshot šuplíku (⋯ karty, Ctrl+K, Nastavení) — jen paměť.
#[derive(serde::Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeskView {
    pub widgets: Vec<DeskWidget>,
    pub profiles: Vec<String>,
    pub active: Option<String>,
    pub hidden_all: bool,
    pub max: usize,
}

pub(crate) fn view() -> DeskView {
    // soubor se načte až s prvním widgetem / Nastavením — snapshot kvůli němu na disk nesahá
    let g = STATE.lock();
    let Some(s) = g.as_ref() else { return DeskView { max: MAX_WIDGETS, ..Default::default() } };
    DeskView {
        widgets: s.widgets.clone(),
        profiles: s.profiles.iter().map(|p| p.name.clone()).collect(),
        active: s.active.clone(),
        hidden_all: HIDDEN_ALL.load(Ordering::Relaxed),
        max: MAX_WIDGETS,
    }
}

/// Config se změnil: vzhled do oken widgetů (motiv bez restartu).
pub(crate) fn on_config(old: &crate::config::Config, new: &crate::config::Config) {
    if old.appearance != new.appearance {
        for (l, w) in windows() {
            let _ = w.emit_to(l.as_str(), "appearance", &new.appearance);
        }
    }
    if old.ui.lang != new.ui.lang {
        for (l, w) in windows() {
            let _ = w.emit_to(l.as_str(), "desk-lang", new.ui.lang);
        }
    }
}
