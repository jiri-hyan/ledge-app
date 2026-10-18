//! Monitor, DPI a detekce „něco běží přes celou obrazovku“ (hra, video, prezentace).

use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromWindow,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Shell::{
    QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetClassNameW, GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsWindowVisible, IsZoomed, MONITORINFOF_PRIMARY, SM_SWAPBUTTON, WS_CAPTION,
};
use windows::core::BOOL;

#[derive(Debug, Clone, Copy)]
pub struct Monitor {
    pub handle: isize,
    pub rect: RECT,
    /// pracovní plocha (bez hlavního panelu) — strop výšky šuplíku (`drawerfit.rs`)
    pub work: RECT,
    pub primary: bool,
    pub dpi: u32,
}

pub fn monitors() -> Vec<Monitor> {
    unsafe extern "system" fn cb(h: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        unsafe {
            let out = &mut *(data.0 as *mut Vec<Monitor>);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if GetMonitorInfoW(h, &mut mi).as_bool() {
                let (mut dx, mut dy) = (96u32, 96u32);
                let _ = GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
                out.push(Monitor {
                    handle: h.0 as isize,
                    rect: mi.rcMonitor,
                    work: mi.rcWork,
                    primary: mi.dwFlags & MONITORINFOF_PRIMARY != 0,
                    dpi: dx,
                });
            }
            BOOL(1)
        }
    }
    let mut v: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut v as *mut _ as isize));
    }
    v
}

/// "primary" nebo index.
pub fn pick(spec: &str) -> Option<Monitor> {
    let all = monitors();
    if let Ok(i) = spec.parse::<usize>()
        && let Some(m) = all.get(i)
    {
        return Some(*m);
    }
    all.iter().find(|m| m.primary).copied().or_else(|| all.first().copied())
}

pub fn cursor() -> Option<POINT> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok().map(|_| p) }
}

/// Co je přes celou obrazovku: exkluzivní režim (D3D, prezentace), okno popředí pokrývající monitor, nic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsState {
    None,
    Exclusive,
    Covering,
}

/// Běží na daném monitoru něco přes celou obrazovku?
/// Kombinace: stav notifikací Windows (D3D exkluzivní fullscreen, prezentace)
/// + popředí okno pokrývá celý monitor (borderless fullscreen hry, video, prohlížeč po F11).
pub fn fullscreen_state(mon: &Monitor) -> FsState {
    unsafe {
        if let Ok(state) = SHQueryUserNotificationState()
            && (state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE)
        {
            return FsState::Exclusive;
        }
    }
    if foreground_covers(mon) { FsState::Covering } else { FsState::None }
}

fn foreground_covers(mon: &Monitor) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || !IsWindowVisible(fg).as_bool() {
            return false;
        }
        if is_shell(fg) {
            return false;
        }
        if MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST).0 as isize != mon.handle {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let covers = r.left <= mon.rect.left && r.top <= mon.rect.top && r.right >= mon.rect.right && r.bottom >= mon.rect.bottom;
        if !covers {
            return false;
        }
        // maximalizované okno s titulkem (prohlížeč, IDE) při automaticky skrytém hlavním panelu
        // pokrývá celý monitor taky — to NENÍ hra ani video, Ledge má zůstat
        let style = GetWindowLongPtrW(fg, GWL_STYLE) as u32;
        let captioned = style & WS_CAPTION.0 == WS_CAPTION.0;
        !(IsZoomed(fg).as_bool() || captioned)
    }
}

/// Je právě stisknuté primární tlačítko myši (s ohledem na prohozená tlačítka)?
pub fn primary_button_down() -> bool {
    unsafe {
        let vk = if GetSystemMetrics(SM_SWAPBUTTON) != 0 { VK_RBUTTON } else { VK_LBUTTON };
        (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0
    }
}

/// Okno v popředí pro kontextový trigger (prohlížeč s taby nahoře).
#[derive(Debug, Clone, Copy)]
pub struct ForegroundCtx<'a> {
    /// název exe procesu (např. "opera.exe"); prázdný, když ho nejde zjistit
    pub exe: &'a str,
    pub maximized: bool,
    /// HMONITOR, na kterém okno leží (porovnat s `Monitor::handle`)
    pub monitor: isize,
    pub pid: u32,
}

/// Čte okno v popředí. Název exe se cachuje podle (HWND, PID) — pro stejné okno se nemění
/// a `OpenProcess` není zadarmo; jinak jen pár levných volání bez alokace. Neúspěšné zjištění
/// (prázdný název) se zkouší znovu po 2 s.
#[derive(Debug)]
pub struct ForegroundProbe {
    hwnd: isize,
    pid: u32,
    exe: String,
    exe_at: std::time::Instant,
}

impl Default for ForegroundProbe {
    fn default() -> Self {
        Self { hwnd: 0, pid: 0, exe: String::new(), exe_at: std::time::Instant::now() }
    }
}

impl ForegroundProbe {
    pub fn read(&mut self) -> Option<ForegroundCtx<'_>> {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            if pid == 0 {
                return None;
            }
            let h = fg.0 as isize;
            let retry = self.exe.is_empty() && self.exe_at.elapsed() > std::time::Duration::from_secs(2);
            if h != self.hwnd || pid != self.pid || retry {
                self.hwnd = h;
                self.pid = pid;
                self.exe_at = std::time::Instant::now();
                self.exe = crate::ports::process_image(pid)
                    .and_then(|p| p.rsplit(['\\', '/']).next().map(str::to_string))
                    .unwrap_or_default();
            }
            Some(ForegroundCtx {
                exe: &self.exe,
                maximized: IsZoomed(fg).as_bool(),
                monitor: MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST).0 as isize,
                pid,
            })
        }
    }
}

fn is_shell(h: HWND) -> bool {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(h, &mut buf) } as usize;
    let class = String::from_utf16_lossy(&buf[..n]);
    matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_a_primary_monitor_with_sane_dpi() {
        let all = monitors();
        if all.is_empty() {
            return; // headless CI
        }
        let p = pick("primary").unwrap();
        assert!(p.dpi >= 96);
        assert!(p.rect.right > p.rect.left);
    }
}
