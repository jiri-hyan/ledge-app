//! Skutečný tlak myši na horní hranu přes raw input.
//!
//! Kurzor je na horním řádku clampnutý, ale `WM_INPUT` dál nese relativní pohyb (`lLastY < 0`) —
//! z toho se pozná, že myš opravdu tlačí nahoru, a ne jen stojí na tabu. Vlastní vlákno
//! s message-only oknem jen sčítá; rozhoduje `edge::PushTracker` v hover vlákně.
//!
//! Výkon: myš se registruje (`RIDEV_INPUTSINK`) JEN na povel `arm(true)` — hover vlákno ho dá, když
//! je v popředí lištové okno a kurzor je pár px u horní hrany v zóně proužku. Jinak `arm(false)`
//! a v klidu žádné `WM_INPUT` nechodí. Registrace raw inputu je pro (usage page, usage) per-proces
//! a Tauri (tao) si myš registruje pro device eventy — při odzbrojení se proto obnoví předchozí
//! registrace místo `RIDEV_REMOVE`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, GetRegisteredRawInputDevices, HRAWINPUT, MOUSE_MOVE_ABSOLUTE, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEMOUSE, RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG, PostMessageW,
    RI_MOUSE_LEFT_BUTTON_DOWN, RI_MOUSE_MIDDLE_BUTTON_DOWN, RI_MOUSE_RIGHT_BUTTON_DOWN, RegisterClassW, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_INPUT, WNDCLASSW,
};
use windows::core::w;

const WM_ARM: u32 = WM_APP + 1;
/// HID usage page / usage: generic desktop / mouse
const MOUSE: (u16, u16) = (1, 2);

/// Co se nasbíralo od posledního `take`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RawTake {
    /// součet pohybu nahoru (relativní counts, jen záporné `lLastY`)
    pub up: u32,
    /// mezitím stisk tlačítka (klik na tab mezi dvěma tiky)
    pub clicked: bool,
    /// od zapnutí přišel absolutní pohyb (tablet, vzdálená plocha) → tlak měřit nejde
    pub absolute: bool,
    /// myš je zaregistrovaná a události chodí
    pub live: bool,
}

#[derive(Default)]
pub struct RawPush {
    hwnd: AtomicIsize,
    up: AtomicU32,
    clicks: AtomicU32,
    absolute: AtomicBool,
    live: AtomicBool,
}

impl RawPush {
    pub fn start() -> Arc<RawPush> {
        let raw = Arc::new(RawPush::default());
        let r = raw.clone();
        std::thread::Builder::new()
            .name("rawpush".into())
            .spawn(move || unsafe { run(r) })
            .ok();
        raw
    }

    /// Zapnout/vypnout odběr raw myši. Volat jen při změně (posílá zprávu vláknu).
    pub fn arm(&self, on: bool) {
        let h = self.hwnd.load(Ordering::Acquire);
        if h != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(h as *mut _)), WM_ARM, WPARAM(on as usize), LPARAM(0));
            }
        }
    }

    /// Vybere nasčítané hodnoty (a vynuluje je).
    pub fn take(&self) -> RawTake {
        RawTake {
            up: self.up.swap(0, Ordering::Relaxed),
            clicked: self.clicks.swap(0, Ordering::Relaxed) > 0,
            absolute: self.absolute.load(Ordering::Relaxed),
            live: self.live.load(Ordering::Relaxed),
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

/// Aktuální registrace myši v procesu (typicky tao) — aby šla po odzbrojení vrátit.
unsafe fn registered_mouse() -> Option<RAWINPUTDEVICE> {
    unsafe {
        let size = std::mem::size_of::<RAWINPUTDEVICE>() as u32;
        let mut n = 0u32;
        GetRegisteredRawInputDevices(None, &mut n, size);
        if n == 0 {
            return None;
        }
        let mut buf = vec![RAWINPUTDEVICE::default(); n as usize];
        let got = GetRegisteredRawInputDevices(Some(buf.as_mut_ptr()), &mut n, size);
        if got == u32::MAX {
            return None;
        }
        buf.into_iter().take(got as usize).find(|d| (d.usUsagePage, d.usUsage) == MOUSE)
    }
}

unsafe fn run(raw: Arc<RawPush>) {
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("LedgeRawPush");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc);
        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinst.into()),
            None,
        ) else {
            crate::log("rawpush: message-only okno nevzniklo — tlak se měří jen časem");
            return;
        };
        raw.hwnd.store(hwnd.0 as isize, Ordering::Release);

        let size = std::mem::size_of::<RAWINPUTDEVICE>() as u32;
        let mut armed = false;
        let mut prev: Option<RAWINPUTDEVICE> = None;
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            match msg.message {
                WM_ARM => {
                    let on = msg.wParam.0 != 0;
                    if on && !armed {
                        prev = registered_mouse().filter(|p| p.hwndTarget != hwnd);
                        raw.absolute.store(false, Ordering::Relaxed);
                        raw.up.store(0, Ordering::Relaxed);
                        let dev = RAWINPUTDEVICE { usUsagePage: MOUSE.0, usUsage: MOUSE.1, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd };
                        armed = RegisterRawInputDevices(&[dev], size).is_ok();
                        raw.live.store(armed, Ordering::Relaxed);
                    } else if !on && armed {
                        armed = false;
                        raw.live.store(false, Ordering::Relaxed);
                        let restored = prev.take().is_some_and(|p| RegisterRawInputDevices(&[p], size).is_ok());
                        if !restored {
                            let off = RAWINPUTDEVICE { usUsagePage: MOUSE.0, usUsage: MOUSE.1, dwFlags: RIDEV_REMOVE, hwndTarget: HWND::default() };
                            let _ = RegisterRawInputDevices(&[off], size);
                        }
                    }
                    continue;
                }
                WM_INPUT if armed => {
                    let mut data = RAWINPUT::default();
                    let mut len = std::mem::size_of::<RAWINPUT>() as u32;
                    let n = GetRawInputData(
                        HRAWINPUT(msg.lParam.0 as *mut _),
                        RID_INPUT,
                        Some(&mut data as *mut _ as *mut core::ffi::c_void),
                        &mut len,
                        std::mem::size_of::<RAWINPUTHEADER>() as u32,
                    );
                    if n != u32::MAX && n > 0 && data.header.dwType == RIM_TYPEMOUSE.0 {
                        let m = data.data.mouse;
                        if m.usFlags.0 & MOUSE_MOVE_ABSOLUTE.0 != 0 {
                            raw.absolute.store(true, Ordering::Relaxed);
                        } else if m.lLastY < 0 {
                            raw.up.fetch_add(m.lLastY.unsigned_abs(), Ordering::Relaxed);
                        }
                        let buttons = m.Anonymous.Anonymous.usButtonFlags as u32;
                        if buttons & (RI_MOUSE_LEFT_BUTTON_DOWN | RI_MOUSE_RIGHT_BUTTON_DOWN | RI_MOUSE_MIDDLE_BUTTON_DOWN) != 0 {
                            raw.clicks.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                _ => {}
            }
            // WM_INPUT musí projít DefWindowProc (úklid systému)
            DispatchMessageW(&msg);
        }
    }
}
