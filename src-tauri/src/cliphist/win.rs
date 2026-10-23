//! Tenká vrstva WinAPI historie schránky: vlákno `cliphist` s oknem jen pro zprávy (`HWND_MESSAGE`),
//! `AddClipboardFormatListener` → `WM_CLIPBOARDUPDATE` a `WTSRegisterSessionNotification` →
//! `WM_WTSSESSION_CHANGE` (zamčení). Žádný časovač: vlákno spí v `GetMessageW`, dokud se schránka
//! nezmění. Vzniká v `start` (funkce zapnutá), `stop` okno zničí (listener i odběr zamčení se
//! odregistrují ve `WM_DESTROY`) a vlákno skončí.
//!
//! Na změnu: otevřít schránku → vyjmenovat formáty (+ jména registrovaných, DWORD u
//! `filter::DWORD_FORMATS`) → vlastník → `filter::pre_verdict`; text (`CF_UNICODETEXT`) se čte JEN když
//! tohle projde. Rozhodnutí je v `filter.rs`, tady jen čtení.

use super::filter::{self, DWORD_FORMATS, Fmt};
use crate::state::log;
use parking_lot::Mutex;
use std::sync::mpsc;
use std::time::Duration;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW, GetClipboardOwner,
    GetClipboardSequenceNumber, OpenClipboard, RemoveClipboardFormatListener,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::RemoteDesktop::{NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

/// hwnd okna listeneru (`None` = neběží)
static WIN: Mutex<Option<isize>> = Mutex::new(None);
/// nejvýš formátů na jedné schránce (pojistka proti nekonečnému výčtu)
const MAX_FORMATS: usize = 256;

pub fn seq() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

/// Spustí listener, pokud neběží. Počká (nejvýš 2 s), než okno vznikne — `stop` hned potom ho najde.
pub fn start() {
    let mut g = WIN.lock();
    if g.is_some() {
        return;
    }
    let (tx, rx) = mpsc::sync_channel::<isize>(1);
    if std::thread::Builder::new().name("cliphist".into()).spawn(move || unsafe { run(tx) }).is_err() {
        return;
    }
    if let Ok(h) = rx.recv_timeout(Duration::from_secs(2))
        && h != 0
    {
        *g = Some(h);
    }
}

/// Zastaví listener: okno se zničí, vlákno skončí.
pub fn stop() {
    if let Some(h) = WIN.lock().take() {
        unsafe {
            let _ = PostMessageW(Some(HWND(h as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}

unsafe fn run(tx: mpsc::SyncSender<isize>) {
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("LedgeClipHist");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc);
        let Ok(hwnd) = CreateWindowExW(WINDOW_EX_STYLE(0), class, w!("Ledge clipboard"), WINDOW_STYLE(0), 0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(hinst.into()), None) else {
            let _ = tx.send(0);
            return;
        };
        if let Err(e) = AddClipboardFormatListener(hwnd) {
            log(&format!("cliphist: AddClipboardFormatListener {e}"));
            let _ = DestroyWindow(hwnd);
            let _ = tx.send(0);
            return;
        }
        // zamčení stanice (`clearOnLock`); když se odběr nepovede, historie jen nebude mazaná při zamčení
        if let Err(e) = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) {
            log(&format!("cliphist: WTSRegisterSessionNotification {e}"));
        }
        let _ = tx.send(hwnd.0 as isize);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CLIPBOARDUPDATE => {
                on_update(hwnd);
                LRESULT(0)
            }
            WM_WTSSESSION_CHANGE => {
                if wp.0 as u32 == WTS_SESSION_LOCK {
                    super::on_lock();
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = RemoveClipboardFormatListener(hwnd);
                let _ = WTSUnRegisterSessionNotification(hwnd);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn on_update(hwnd: HWND) {
    unsafe {
        let Some(cfg) = super::active() else { return };
        let seq = GetClipboardSequenceNumber();
        // schránku může mít chvíli otevřenou jiná aplikace (právě do ní zapisuje)
        let mut open = false;
        for _ in 0..8 {
            if OpenClipboard(Some(hwnd)).is_ok() {
                open = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        if !open {
            return;
        }
        let fmts = formats();
        let owner = owner_exe();
        let text = match filter::pre_verdict(&fmts, owner.as_deref(), &cfg.exclude_apps) {
            None => read_text(),
            Some(_) => None,
        };
        let _ = CloseClipboard();
        if let Some(t) = text {
            super::on_text(seq, t);
        }
    }
}

/// Formáty na schránce (schránka otevřená): id, jméno registrovaných, DWORD u `DWORD_FORMATS`.
pub(crate) unsafe fn formats() -> Vec<Fmt> {
    unsafe {
        let mut out = Vec::new();
        let mut f = 0;
        while out.len() < MAX_FORMATS {
            f = EnumClipboardFormats(f);
            if f == 0 {
                break;
            }
            let name = (f >= 0xC000).then(|| format_name(f)).flatten();
            let dword = match &name {
                Some(n) if DWORD_FORMATS.iter().any(|d| d.eq_ignore_ascii_case(n)) => read_dword(f),
                _ => None,
            };
            out.push(Fmt { id: f, name, dword });
        }
        out
    }
}

unsafe fn format_name(f: u32) -> Option<String> {
    unsafe {
        let mut buf = [0u16; 128];
        let n = GetClipboardFormatNameW(f, &mut buf);
        (n > 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }
}

unsafe fn global(f: u32) -> Option<HGLOBAL> {
    unsafe {
        let h: HANDLE = GetClipboardData(f).ok()?;
        (!h.0.is_null()).then_some(HGLOBAL(h.0))
    }
}

unsafe fn read_dword(f: u32) -> Option<u32> {
    unsafe {
        let h = global(f)?;
        if GlobalSize(h) < 4 {
            return None;
        }
        let p = GlobalLock(h) as *const u32;
        if p.is_null() {
            return None;
        }
        let v = p.read_unaligned();
        let _ = GlobalUnlock(h);
        Some(v)
    }
}

/// `CF_UNICODETEXT` nejvýš do limitu (`filter::utf16_text`); delší = `None` (neukládá se).
unsafe fn read_text() -> Option<String> {
    unsafe {
        let h = global(filter::CF_UNICODETEXT)?;
        let units = (GlobalSize(h) / 2).min(filter::MAX_TEXT_BYTES + 1);
        let p = GlobalLock(h) as *const u16;
        if p.is_null() {
            return None;
        }
        let r = filter::utf16_text(std::slice::from_raw_parts(p, units));
        let _ = GlobalUnlock(h);
        r.ok()
    }
}

/// Exe procesu, kterému patří okno vlastníka schránky (`None` = bez vlastníka / nejde zjistit).
pub(crate) unsafe fn owner_exe() -> Option<String> {
    unsafe {
        let owner = GetClipboardOwner().ok().filter(|h| !h.0.is_null())?;
        let mut pid = 0u32;
        GetWindowThreadProcessId(owner, Some(&mut pid));
        (pid != 0).then(|| crate::ports::process_image(pid)).flatten()
    }
}
