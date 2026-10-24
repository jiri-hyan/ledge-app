//! Ukládání widgetů na ploše (`desk.json`) a přichycení po přesunu — vyčleněno z `win.rs` (kolo 21, bez změny
//! chování). Zápis se zpožděním ve vlákně `desk-save`, které spí na kanálu: série pohybů a změn velikosti = jeden
//! zápis; po tahu okna se přichytí k okrajům (`snap`), až když je pustěné tlačítko myši. `flush` zapíše hned
//! (konec Ledge, nelze-li spustit vlákno).

use super::win::{STATE, app, monitors, store_path, windows, with};
use super::*;
use crate::state::log;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{Manager, PhysicalPosition};

static SAVER: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
/// Widgety posunuté od posledního přichycení.
static SNAP_DUE: Mutex<Option<HashSet<String>>> = Mutex::new(None);
const SAVE_DELAY: Duration = Duration::from_millis(400);

/// Widget se posunul → po pustění myši ho přichytit (`snap_moved`) a zapsat.
pub(super) fn mark_moved(id: &str) {
    SNAP_DUE.lock().get_or_insert_with(HashSet::new).insert(id.to_string());
    persist();
}

/// Zapsat se zpožděním (série pohybů = jeden zápis); po přesunu přichytit, až je myš puštěná.
pub(super) fn persist() {
    let mut g = SAVER.lock();
    if g.as_ref().is_some_and(|tx| tx.send(()).is_ok()) {
        return;
    }
    let (tx, rx) = mpsc::channel::<()>();
    let _ = tx.send(());
    let spawned = std::thread::Builder::new().name("desk-save".into()).spawn(move || {
        while rx.recv().is_ok() {
            loop {
                while rx.recv_timeout(SAVE_DELAY).is_ok() {}
                // tah okna ještě běží (tlačítko držené) → počkat; jinak by se přichycení přetahovalo s myší
                if !mouse_down() {
                    break;
                }
            }
            snap_moved();
            flush();
        }
    });
    match spawned {
        Ok(_) => *g = Some(tx),
        Err(_) => flush(),
    }
}

fn mouse_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    // SAFETY: jen čtení stavu klávesy
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

/// Zapsat hned (konec Ledge, vlákno `desk-save`).
pub(crate) fn flush() {
    let Some(snap) = STATE.lock().clone() else { return };
    if let Err(e) = save(&store_path(), &snap) {
        log(&format!("widgety na ploše: stav se neuložil ({e})"));
    }
}

/// Po puštění: posunuté widgety přichytit k okraji plochy / k sousedům (config `desk.snap`).
fn snap_moved() {
    let due: Vec<String> = SNAP_DUE.lock().take().map(|s| s.into_iter().collect()).unwrap_or_default();
    if due.is_empty() || !crate::state::CORE.get().is_some_and(|c| c.cfg.read().desk.snap) {
        return;
    }
    let rects: Vec<(String, Area, f64)> = windows()
        .into_iter()
        .filter_map(|(l, w)| {
            let p = w.outer_position().ok()?;
            let s = w.outer_size().ok()?;
            let scale = w.scale_factor().unwrap_or(1.0);
            Some((l, Area { l: p.x, t: p.y, r: p.x + s.width as i32, b: p.y + s.height as i32 }, scale))
        })
        .collect();
    let mons = monitors();
    for id in due {
        let label = label_for(&id);
        let Some((_, r, scale)) = rects.iter().find(|(l, _, _)| *l == label).cloned() else { continue };
        let Some(win) = app().get_webview_window(&label) else { continue };
        let Some(m) = pins::monitor_of(r.l, r.t, r.r - r.l, (BAR_H * scale) as i32, &mons).map(|i| mons[i]) else { continue };
        let others: Vec<Area> = rects.iter().filter(|(l, _, _)| *l != label).map(|(_, a, _)| *a).collect();
        let (x, y) = snap((r.l, r.t, r.r - r.l, r.b - r.t), m.work, &others, (SNAP_PX * scale).round() as i32);
        if (x, y) != (r.l, r.t) {
            let _ = win.set_position(PhysicalPosition::new(x, y));
            with(|s| {
                if let Some(w) = s.by_id_mut(&id) {
                    (w.x, w.y) = (Some(x), Some(y));
                }
            });
        }
    }
}
