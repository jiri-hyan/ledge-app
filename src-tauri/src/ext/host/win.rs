//! Win32 pomocníky hostitele rozšíření: třída okna s obslužnou funkcí v closure, smyčka zpráv s vlákovými
//! zprávami (probuzení z jiných vláken), krátké čekání se zpracováním zpráv (inicializace WebView2) a DPI.
//! Žádná logika rozšíření — jen to, co bude potřebovat každé okno (tapeta, později prohlížeč).

#![allow(unsafe_op_in_unsafe_fn)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

/// Obsluha zpráv okna. `Some(výsledek)` = zpráva vyřízena, `None` = výchozí zpracování.
pub type Handler = Box<dyn FnMut(HWND, u32, WPARAM, LPARAM) -> Option<LRESULT>>;

struct Slot(RefCell<Handler>);

/// Fyzické pixely a ne virtualizované souřadnice (Progman, monitory). Chyba (už nastaveno) se ignoruje.
pub fn init_dpi() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn class_name(h: HWND) -> String {
    let mut b = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut b) } as usize;
    String::from_utf16_lossy(&b[..n])
}

/// `(left, top, right, bottom)` okna v obrazovkových souřadnicích.
pub fn window_rect(h: HWND) -> (i32, i32, i32, i32) {
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(h, &mut r);
    }
    (r.left, r.top, r.right, r.bottom)
}

pub fn thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

/// Vlákno `tid` probudit zprávou (`WM_APP + n`); `false` = vlákno už neexistuje.
pub fn post_thread(tid: u32, msg: u32) -> bool {
    unsafe { PostThreadMessageW(tid, msg, WPARAM(0), LPARAM(0)).is_ok() }
}

unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if m == WM_NCCREATE {
        let cs = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(h, GWLP_USERDATA, cs.lpCreateParams as isize);
    }
    let p = GetWindowLongPtrW(h, GWLP_USERDATA) as *const Slot;
    let mut res = None;
    if !p.is_null() {
        // vnořená zpráva z uvnitř obsluhy (SendMessage ze sebe) → výchozí zpracování, ne panika
        if let Ok(mut f) = (*p).0.try_borrow_mut() {
            res = f(h, m, wp, lp);
        }
    }
    if m == WM_NCDESTROY && !p.is_null() {
        SetWindowLongPtrW(h, GWLP_USERDATA, 0);
        drop(Box::from_raw(p as *mut Slot));
    }
    match res {
        Some(r) => r,
        None => DefWindowProcW(h, m, wp, lp),
    }
}

fn register(class: &str) {
    unsafe {
        let hi = GetModuleHandleW(None).unwrap_or_default();
        let name = wide(class);
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hi.into(), lpszClassName: PCWSTR(name.as_ptr()), ..Default::default() };
        // opakovaná registrace téže třídy selže — nevadí
        RegisterClassW(&wc);
    }
}

/// Vytvoří okno třídy `class` (zaregistruje ji při prvním použití) s obsluhou `handler`. `rect` = x, y, šířka,
/// výška (u dítěte relativně k rodiči). Okno vzniká skryté (bez `WS_VISIBLE`).
pub fn create_window(class: &str, ex: WINDOW_EX_STYLE, style: WINDOW_STYLE, rect: (i32, i32, i32, i32), parent: Option<HWND>, handler: Handler) -> Option<HWND> {
    register(class);
    let slot = Box::into_raw(Box::new(Slot(RefCell::new(handler))));
    unsafe {
        let hi = GetModuleHandleW(None).unwrap_or_default();
        let name = wide(class);
        let r = CreateWindowExW(ex, PCWSTR(name.as_ptr()), PCWSTR(name.as_ptr()), style, rect.0, rect.1, rect.2, rect.3, parent, None, Some(hi.into()), Some(slot as *const c_void));
        match r {
            Ok(h) => Some(h),
            Err(_) => {
                // WM_NCDESTROY nepřijde, pokud vznik selhal před WM_NCCREATE
                drop(Box::from_raw(slot));
                None
            }
        }
    }
}

/// Smyčka zpráv vlákna. Zprávy bez okna (`PostThreadMessage`) jdou do `thread_msg`; `false` ukončí smyčku
/// (stejně jako `WM_QUIT`).
pub fn message_loop(mut thread_msg: impl FnMut(u32, WPARAM, LPARAM) -> bool) {
    let mut msg = MSG::default();
    loop {
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if r.0 <= 0 {
            break;
        }
        if msg.hwnd.0.is_null() {
            if !thread_msg(msg.message, msg.wParam, msg.lParam) {
                break;
            }
            continue;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Čeká na hodnotu z kanálu a mezitím obsluhuje zprávy (asynchronní inicializace WebView2 potřebuje běžící
/// smyčku). Vlákové zprávy v tu dobu zahazuje — kdo je posílá, musí mít i frontu, kterou po návratu vyprázdní.
pub fn pump_until<T>(rx: &mpsc::Receiver<T>, ms: u64) -> Option<T> {
    let t0 = Instant::now();
    let mut msg = MSG::default();
    while t0.elapsed() < Duration::from_millis(ms) {
        if let Ok(v) = rx.try_recv() {
            return Some(v);
        }
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}
