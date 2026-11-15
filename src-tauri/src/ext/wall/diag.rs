//! `ledge.exe --ext wall --diag`: 60 s záznam do souboru, co se děje s oknem tapety a stromem Progman/WorkerW
//! (viditelnost, rodič, pořadí, zakrytí). Jen události (`SetWinEventHook`, žádný poll) — pro šéfův test
//! Win+D / restart Průzkumníka, které Claude nedělá. Soubor: `%LOCALAPPDATA%\Ledge\ext\wall\diag.log`.

#![allow(unsafe_op_in_unsafe_fn)]

use crate::ext::host::win::class_name;
use parking_lot::Mutex;
use std::ffi::c_void;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::*;

struct State {
    file: Mutex<std::fs::File>,
    t0: Instant,
    ours: AtomicIsize,
    parent: AtomicIsize,
}

static STATE: OnceLock<State> = OnceLock::new();

fn hwnd(v: isize) -> HWND {
    HWND(v as *mut c_void)
}

/// Řádek logu s časem od startu.
pub fn note(msg: &str) {
    let Some(s) = STATE.get() else { return };
    let line = format!("+{:>6} ms  {msg}\n", s.t0.elapsed().as_millis());
    let _ = s.file.lock().write_all(line.as_bytes());
}

/// Pořadí potomků `p` shora dolů, např. `SHELLDLL_DefView > WorkerW`.
fn children(p: HWND) -> String {
    let mut v = vec![];
    unsafe {
        let mut c = GetWindow(p, GW_CHILD).unwrap_or_default();
        while !c.0.is_null() && v.len() < 12 {
            v.push(format!("{}{}", class_name(c), if IsWindowVisible(c).as_bool() { "" } else { "(skryté)" }));
            c = GetWindow(c, GW_HWNDNEXT).unwrap_or_default();
        }
    }
    v.join(" > ")
}

fn describe() -> String {
    let Some(s) = STATE.get() else { return String::new() };
    let (o, p) = (hwnd(s.ours.load(Ordering::Relaxed)), hwnd(s.parent.load(Ordering::Relaxed)));
    if o.0.is_null() {
        return "wall: okno nevzniklo".into();
    }
    unsafe {
        if !IsWindow(Some(o)).as_bool() {
            return "wall: okno zaniklo".into();
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(o, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut c_void, 4);
        let par = GetParent(o).unwrap_or_default();
        format!(
            "wall: viditelné={} cloaked={} rodič={}({:x}) [čekáno {:x}] | potomci rodiče: {}",
            IsWindowVisible(o).as_bool(),
            cloaked,
            class_name(par),
            par.0 as usize,
            p.0 as usize,
            if par.0.is_null() { String::new() } else { children(par) }
        )
    }
}

fn event_name(e: u32) -> &'static str {
    match e {
        EVENT_SYSTEM_FOREGROUND => "FOREGROUND",
        EVENT_OBJECT_CREATE => "CREATE",
        EVENT_OBJECT_DESTROY => "DESTROY",
        EVENT_OBJECT_SHOW => "SHOW",
        EVENT_OBJECT_HIDE => "HIDE",
        EVENT_OBJECT_REORDER => "REORDER",
        EVENT_OBJECT_LOCATIONCHANGE => "LOCATIONCHANGE",
        EVENT_OBJECT_CLOAKED => "CLOAKED",
        EVENT_OBJECT_UNCLOAKED => "UNCLOAKED",
        _ => "?",
    }
}

unsafe extern "system" fn on_event(_h: HWINEVENTHOOK, event: u32, h: HWND, id_object: i32, _child: i32, _thread: u32, _ms: u32) {
    let Some(s) = STATE.get() else { return };
    if event != EVENT_SYSTEM_FOREGROUND && id_object != OBJID_WINDOW.0 {
        return;
    }
    let cls = class_name(h);
    let ours = h.0 as isize == s.ours.load(Ordering::Relaxed);
    let parent = h.0 as isize == s.parent.load(Ordering::Relaxed);
    let shell = matches!(cls.as_str(), "Progman" | "WorkerW" | "SHELLDLL_DefView");
    if !(ours || parent || shell || event == EVENT_SYSTEM_FOREGROUND) {
        return;
    }
    let who = if ours { "wall".to_string() } else { cls };
    note(&format!("{:<14} {who} {:x} | {}", event_name(event), h.0 as usize, describe()));
}

/// Založí soubor a háčky (popředí, události objektů, cloak). Volat na vlákně se smyčkou zpráv.
pub fn start(path: &Path) -> bool {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let Ok(file) = std::fs::File::create(path) else { return false };
    let _ = STATE.set(State { file: Mutex::new(file), t0: Instant::now(), ours: AtomicIsize::new(0), parent: AtomicIsize::new(0) });
    unsafe {
        for (lo, hi) in [(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND), (EVENT_OBJECT_CREATE, EVENT_OBJECT_LOCATIONCHANGE), (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED)] {
            let _ = SetWinEventHook(lo, hi, None, Some(on_event), 0, 0, WINEVENT_OUTOFCONTEXT);
        }
    }
    note("diag start (Win+D, změna tapety, restart Průzkumníka — zkus a pak přečti tento soubor)");
    true
}

/// Okno tapety a jeho rodič se změnily (po připnutí).
pub fn set_target(ours: HWND, parent: HWND) {
    if let Some(s) = STATE.get() {
        s.ours.store(ours.0 as isize, Ordering::Relaxed);
        s.parent.store(parent.0 as isize, Ordering::Relaxed);
        note(&format!("připnuto | {}", describe()));
    }
}
