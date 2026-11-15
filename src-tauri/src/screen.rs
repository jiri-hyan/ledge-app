//! Monitor, DPI a detekce „něco běží přes celou obrazovku“ (hra, video, prezentace).

use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFOEXW,
    MonitorFromWindow,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Shell::SHQueryUserNotificationState;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetClassNameW, GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed, MONITORINFOF_PRIMARY, SM_SWAPBUTTON, WS_CAPTION,
};
use std::sync::Mutex;
use windows::core::BOOL;

#[derive(Debug, Clone, Copy)]
pub struct Monitor {
    pub handle: isize,
    pub rect: RECT,
    /// pracovní plocha (bez hlavního panelu) — strop výšky šuplíku (`drawerfit.rs`)
    pub work: RECT,
    pub primary: bool,
    pub dpi: u32,
    /// název zařízení (`\\.\DISPLAY2`, `MONITORINFOEXW.szDevice`), UTF-16 zakončené nulou — stabilní identita
    /// monitoru (index v `EnumDisplayMonitors` se po dokování / odpojení přeskládá); čte `device_name()`
    pub device: [u16; 32],
}

impl Monitor {
    pub fn device_name(&self) -> String {
        let n = self.device.iter().position(|&c| c == 0).unwrap_or(self.device.len());
        String::from_utf16_lossy(&self.device[..n])
    }
}

pub fn monitors() -> Vec<Monitor> {
    unsafe extern "system" fn cb(h: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        unsafe {
            let out = &mut *(data.0 as *mut Vec<Monitor>);
            let mut ex = MONITORINFOEXW::default();
            // `cbSize` rozhoduje, že se vyplní i `szDevice` (`MONITORINFOEXW` začíná `MONITORINFO`)
            ex.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if GetMonitorInfoW(h, &mut ex.monitorInfo).as_bool() {
                let mi = ex.monitorInfo;
                let (mut dx, mut dy) = (96u32, 96u32);
                let _ = GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
                out.push(Monitor {
                    handle: h.0 as isize,
                    rect: mi.rcMonitor,
                    work: mi.rcWork,
                    primary: mi.dwFlags & MONITORINFOF_PRIMARY != 0,
                    dpi: dx,
                    device: ex.szDevice,
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

/// Identita monitoru pro čistý výběr (`resolve`): název zařízení, poloha + velikost, hlavní.
pub struct Ident<'a> {
    pub device: &'a str,
    pub rect: RECT,
    pub primary: bool,
}

/// Výsledek `resolve`: index do seznamu, případně proč to není to, co spec chtěl (`why`, jde do logu),
/// a stabilní spec, který si má volající zapamatovat (`learn`, jen u číselného specu).
#[derive(Debug, PartialEq, Eq)]
pub struct Resolved {
    pub idx: usize,
    pub why: Option<String>,
    pub learn: Option<String>,
}

/// Stabilní spec monitoru: `dev:<zařízení>;<š>x<v>@<x>,<y>` (název zařízení, rozlišení a poloha jako záloha).
pub fn stable_spec(device: &str, r: &RECT) -> String {
    format!("dev:{device};{}", geometry(r))
}

fn geometry(r: &RECT) -> String {
    format!("{}x{}@{},{}", r.right - r.left, r.bottom - r.top, r.left, r.top)
}

fn find_stable(spec: &str, all: &[Ident]) -> Option<usize> {
    let body = spec.strip_prefix("dev:")?;
    let (dev, geo) = body.split_once(';').unwrap_or((body, ""));
    if !dev.is_empty()
        && let Some(i) = all.iter().position(|m| m.device.eq_ignore_ascii_case(dev))
    {
        return Some(i);
    }
    // záloha: stejná poloha a velikost (ovladač přečíslování `\.\DISPLAYn`)
    if geo.is_empty() { None } else { all.iter().position(|m| geometry(&m.rect) == geo) }
}

/// `strip.monitor = "all"`: notch na každém monitoru (plný proužek na hlavním, tečka na ostatních, `stripset.rs`).
pub const SPEC_ALL: &str = "all";

/// Spec → monitor. `spec`: `primary` | `all` (hlavní monitor) | číselný index (starý config; při prvním platném výběru se přeloží na
/// stabilní spec, který volající drží v `remembered`) | `dev:...` (`stable_spec`). Neplatné / nepřipojené =
/// hlavní monitor + `why` (nikdy tiše). `all` nesmí být prázdné.
pub fn resolve(spec: &str, all: &[Ident], remembered: Option<&str>) -> Resolved {
    let primary = all.iter().position(|m| m.primary).unwrap_or(0);
    let fall = |why: String| Resolved { idx: primary, why: Some(why), learn: None };
    if spec == "primary" || spec == SPEC_ALL {
        return Resolved { idx: primary, why: None, learn: None };
    }
    if spec.starts_with("dev:") {
        return match find_stable(spec, all) {
            Some(i) => Resolved { idx: i, why: None, learn: None },
            None => fall(format!("monitor „{spec}“ není připojen — hlavní monitor")),
        };
    }
    match spec.parse::<usize>() {
        Ok(i) => {
            if let Some(r) = remembered {
                return match find_stable(r, all) {
                    Some(j) => Resolved { idx: j, why: None, learn: None },
                    None => fall(format!("monitor {i} ({r}) není připojen — hlavní monitor")),
                };
            }
            match all.get(i) {
                Some(m) => Resolved { idx: i, why: None, learn: Some(stable_spec(m.device, &m.rect)) },
                None => fall(format!("monitor {i} neexistuje (připojeno {}) — hlavní monitor", all.len())),
            }
        }
        Err(_) => fall(format!("neplatné určení monitoru „{spec}“ — hlavní monitor")),
    }
}

/// Číselný spec → stabilní spec zapamatovaný za běhu (jen záloha: config se při načtení přepíše na stabilní
/// spec — `stripset::migrate_config`; sem se číslo dostane jen z ručně upraveného / importovaného configu).
static REMEMBERED: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
/// Naposledy zalogovaný důvod záložního monitoru (log jen při změně, ne při každém čtení).
static LAST_WHY: Mutex<String> = Mutex::new(String::new());

/// "primary", "all", index nebo stabilní spec (`resolve`). Neplatný / nepřipojený = hlavní monitor se záznamem do logu.
pub fn pick(spec: &str) -> Option<Monitor> {
    let all = monitors();
    if all.is_empty() {
        return None;
    }
    with_idents(&all, |ids| pick_in(spec, &all, ids))
}

/// Identity monitorů pro čisté funkce (`resolve`, `stripset`): názvy zařízení žijí jen po dobu volání `f`.
pub fn with_idents<R>(all: &[Monitor], f: impl FnOnce(&[Ident]) -> R) -> R {
    let names: Vec<String> = all.iter().map(|m| m.device_name()).collect();
    let ids: Vec<Ident> = all.iter().zip(&names).map(|(m, d)| Ident { device: d, rect: m.rect, primary: m.primary }).collect();
    f(&ids)
}

fn pick_in(spec: &str, all: &[Monitor], ids: &[Ident]) -> Option<Monitor> {
    let remembered = REMEMBERED.lock().ok().and_then(|r| r.iter().find(|(s, _)| s == spec).map(|(_, v)| v.clone()));
    let r = resolve(spec, ids, remembered.as_deref());
    if let Some(l) = r.learn
        && let Ok(mut m) = REMEMBERED.lock()
    {
        m.push((spec.to_string(), l));
    }
    let why = r.why.unwrap_or_default();
    if let Ok(mut last) = LAST_WHY.lock()
        && *last != why
    {
        if !why.is_empty() {
            crate::log(&why);
        }
        *last = why;
    }
    all.get(r.idx).copied()
}

/// Je to zpráva okna, která mění zobrazení (monitor / rozlišení: WM_DISPLAYCHANGE, DPI: WM_DPICHANGED,
/// pracovní plocha = hlavní panel přesunut / skryt: WM_SETTINGCHANGE s SPI_SETWORKAREA)? Dostávají ji jen
/// top-level okna (`HWND_MESSAGE` broadcasty nedostane) — Ledge ji čte v oknu proužku (`strip.rs wndproc`).
pub fn is_display_msg(msg: u32, wparam: usize) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{SPI_SETWORKAREA, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_SETTINGCHANGE};
    msg == WM_DISPLAYCHANGE || msg == WM_DPICHANGED || (msg == WM_SETTINGCHANGE && wparam == SPI_SETWORKAREA.0 as usize)
}

/// Mezera (fyzické px) od horní hrany `work` ke spodku proužku + `margin` (logické px) — sem jdou nová okna
/// (lístky, widgety, webové panely). Proužek je u horní hrany monitoru (`rect_top`); pod ním (a pod hlavním
/// panelem nahoře, který posouvá `work_top`) zůstane aspoň `margin`. Jediné místo výpočtu (dřív 3 kopie).
pub fn below_strip(rect_top: i32, work_top: i32, strip_h: f64, margin: f64, scale: f64) -> i32 {
    ((rect_top + ((strip_h + margin) * scale).round() as i32) - work_top).max((margin * scale).round() as i32)
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

/// Okno v popředí pro určení fullscreenu (`classify`): na kterém monitoru leží, kam sahá a jestli se smí
/// brát za „přes celou obrazovku“ (`eligible`: ne maximalizované okno ani okno s titulkem — prohlížeč, IDE
/// při skrytém hlavním panelu). Shell (plocha, hlavní panel), neviditelné a vlastní okno jsou `None`
/// (vlastní okno — Studio roztažené přes monitor — není hra, Ledge se kvůli sobě neschová).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FgWin {
    /// HWND okna (lepkavé pokrytí: `fswatch::next_fs_map` ho drží a znovu ověřuje přes `window_state`)
    pub hwnd: isize,
    pub monitor: isize,
    pub rect: RECT,
    pub eligible: bool,
}

/// Čistě: co leží přes monitor `mon` (handle + obdélník)? `exclusive` = systémový stav notifikací
/// (D3D exkluzivní fullscreen, prezentace) — ten monitor nezná, proto se přiřadí monitoru okna v popředí;
/// bez takového okna platí pro všechny monitory (konzervativně, jako dřív). Jinak `Covering`, když okno
/// v popředí smí být fullscreen a pokrývá `mon` celý (okno přes dva monitory tedy pokryje oba).
pub fn classify(exclusive: bool, fg: Option<FgWin>, mon: isize, mon_rect: &RECT) -> FsState {
    match (exclusive, fg) {
        (true, None) => FsState::Exclusive,
        (true, Some(f)) => if f.monitor == mon { FsState::Exclusive } else { FsState::None },
        (false, Some(f)) if f.eligible && covers(&f.rect, mon_rect) => FsState::Covering,
        _ => FsState::None,
    }
}

pub fn covers(w: &RECT, m: &RECT) -> bool {
    w.left <= m.left && w.top <= m.top && w.right >= m.right && w.bottom >= m.bottom
}

/// Syrová hodnota `SHQueryUserNotificationState` (1 nepřítomen, 2 zaneprázdněn, 3 D3D fullscreen, 4 prezentace,
/// 5 přijímá oznámení, 6+ klid / appka). JEDINÉ místo čtení QUNS (screen, toast, wall).
pub fn notification_state() -> Option<i32> {
    unsafe { SHQueryUserNotificationState() }.ok().map(|s| s.0)
}

/// Čistě: D3D exkluzivní fullscreen (3) nebo prezentace (4).
pub fn qun_exclusive(s: i32) -> bool {
    matches!(s, 3 | 4)
}

/// Čistě: Windows přijímá oznámení (5).
pub fn qun_accepts(s: i32) -> bool {
    s == 5
}

/// Čistě: uživatel je zaneprázdněn / hra / prezentace (2–4) — guard živé zkoušky tapety (`ext/wall/live.rs`, test).
#[cfg(test)]
pub fn qun_busy(s: i32) -> bool {
    (2..=4).contains(&s)
}

/// Je systémový stav notifikací „něco exkluzivního přes celou obrazovku“ (D3D, prezentace)?
pub fn exclusive_now() -> bool {
    notification_state().is_some_and(qun_exclusive)
}

/// Přijímá Windows právě oznámení? (hra, prezentace, „nerušit“ i celoobrazovková appka = ne.)
pub fn accepts_notifications() -> bool {
    notification_state().is_some_and(qun_accepts)
}

/// HMONITOR, na kterém leží okno (`Monitor::handle`); `None`, když okno nemá HWND.
pub fn monitor_of_window(w: &tauri::WebviewWindow) -> Option<isize> {
    let h = w.hwnd().ok()?;
    Some(unsafe { MonitorFromWindow(HWND(h.0 as *mut _), MONITOR_DEFAULTTONEAREST) }.0 as isize)
}

/// Okno `hwnd` pro určení fullscreenu: existuje, je viditelné a ne minimalizované; `None` jinak (i pro shell).
/// Levné (pár volání bez alokace) — `fswatch` ho volá 1× za tik jen pro okno, které drží monitor „pokrytý“.
pub fn window_state(hwnd: isize) -> Option<FgWin> {
    unsafe {
        let fg = HWND(hwnd as *mut _);
        if fg.0.is_null() || !IsWindow(Some(fg)).as_bool() || !IsWindowVisible(fg).as_bool() || IsIconic(fg).as_bool() || is_shell(fg) {
            return None;
        }
        let mut rect = RECT::default();
        if GetWindowRect(fg, &mut rect).is_err() {
            return None;
        }
        let style = GetWindowLongPtrW(fg, GWL_STYLE) as u32;
        let captioned = style & WS_CAPTION.0 == WS_CAPTION.0;
        Some(FgWin { hwnd, monitor: MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST).0 as isize, rect, eligible: !(IsZoomed(fg).as_bool() || captioned) })
    }
}

/// Okno v popředí (`None` = shell, neviditelné, minimalizované, vlastní okno Ledge — Studio roztažené přes
/// monitor není hra a fullscreen hry kvůli kliknutí do Ledge nemá skončit, viz `fswatch::next_fs_map`).
pub fn foreground_window() -> Option<FgWin> {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == std::process::id() {
            return None;
        }
        window_state(fg.0 as isize)
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

    fn r(l: i32, t: i32, w: i32, h: i32) -> RECT {
        RECT { left: l, top: t, right: l + w, bottom: t + h }
    }

    #[test]
    fn resolve_table() {
        let all = [
            Ident { device: r"\.\DISPLAY1", rect: r(0, 0, 1920, 1080), primary: true },
            Ident { device: r"\.\DISPLAY2", rect: r(1920, 0, 2560, 1440), primary: false },
        ];
        let st2 = stable_spec(r"\.\DISPLAY2", &r(1920, 0, 2560, 1440));
        assert_eq!(st2, r"dev:\.\DISPLAY2;2560x1440@1920,0");
        let ok = |idx| Resolved { idx, why: None, learn: None };
        // spec, remembered, očekávání
        assert_eq!(resolve("primary", &all, None), ok(0));
        assert_eq!(resolve(SPEC_ALL, &all, None), ok(0), "všechny = hlavní proužek na hlavním monitoru");
        assert_eq!(resolve(&st2, &all, None), ok(1), "stabilní spec");
        assert_eq!(resolve(r"dev:\.\display2", &all, None), ok(1), "název bez ohledu na velikost písmen");
        // ovladač přečísloval zařízení → záloha podle rozlišení a polohy
        assert_eq!(resolve(r"dev:\.\DISPLAY9;2560x1440@1920,0", &all, None), ok(1), "záloha geometrie");
        // starý číselný spec: platný index se přeloží na stabilní spec
        assert_eq!(resolve("1", &all, None), Resolved { idx: 1, why: None, learn: Some(st2.clone()) });
        // po přeskládání enumerace drží zapamatovaný monitor, ne index
        let swapped = [all[1].device, all[0].device];
        let sw = [
            Ident { device: swapped[0], rect: all[1].rect, primary: false },
            Ident { device: swapped[1], rect: all[0].rect, primary: true },
        ];
        assert_eq!(resolve("1", &sw, Some(&st2)), ok(0), "zapamatovaný DISPLAY2 je teď index 0");
        // zapamatovaný monitor odpojen → hlavní + důvod (ne tichý index)
        let only = [Ident { device: r"\.\DISPLAY1", rect: r(0, 0, 1920, 1080), primary: true }];
        let g = resolve("1", &only, Some(&st2));
        assert_eq!((g.idx, g.why.is_some(), g.learn), (0, true, None));
        // neplatný index, nepřipojený stabilní spec, nesmysl → hlavní + důvod
        for spec in ["7", r"dev:\.\DISPLAY5;800x600@0,0", "xyz", ""] {
            let g = resolve(spec, &all, None);
            assert!(g.idx == 0 && g.why.is_some(), "{spec:?}");
        }
        // hlavní monitor není první v seznamu
        let p2 = [
            Ident { device: "A", rect: r(-1920, 0, 1920, 1080), primary: false },
            Ident { device: "B", rect: r(0, 0, 1920, 1080), primary: true },
        ];
        assert_eq!(resolve("primary", &p2, None).idx, 1);
        assert_eq!(resolve(SPEC_ALL, &p2, None).idx, 1);
        assert_eq!(resolve("9", &p2, None).idx, 1);
    }

    #[test]
    fn classify_table() {
        let a = r(0, 0, 1920, 1080);
        let b = r(1920, 0, 2560, 1440);
        let c = r(0, -1080, 1920, 1080); // monitor nad primárním (záporné y)
        let fg = |monitor, rect, eligible| Some(FgWin { hwnd: 1, monitor, rect, eligible });
        // (exkluzivní QUNS, popředí, monitor, očekávání)
        let cases = [
            (true, None, (1, a), FsState::Exclusive),
            (true, None, (2, b), FsState::Exclusive),
            (true, fg(1, a, true), (1, a), FsState::Exclusive),
            (true, fg(1, a, true), (2, b), FsState::None),
            (true, fg(2, b, false), (2, b), FsState::Exclusive),
            (true, fg(2, b, false), (1, a), FsState::None),
            (false, None, (1, a), FsState::None),
            (false, fg(2, b, true), (2, b), FsState::Covering),
            (false, fg(2, b, true), (1, a), FsState::None),
            // okno nepokrývá celý monitor / maximalizované s titulkem
            (false, fg(1, r(0, 0, 1920, 1040), true), (1, a), FsState::None),
            (false, fg(1, a, false), (1, a), FsState::None),
            // monitor vlevo / nad primárním
            (false, fg(3, c, true), (3, c), FsState::Covering),
            (false, fg(3, c, true), (1, a), FsState::None),
            // okno přesahuje monitor (borderless s přesahem) pořád pokrývá
            (false, fg(2, r(1910, -10, 2580, 1460), true), (2, b), FsState::Covering),
            // okno přes dva monitory pokryje oba (Surround), ale jen když pokrývá celý monitor
            (false, fg(1, r(0, 0, 4480, 1440), true), (1, a), FsState::Covering),
            (false, fg(1, r(0, 0, 4480, 1440), true), (2, b), FsState::Covering),
            (false, fg(1, r(0, 0, 3000, 1440), true), (2, b), FsState::None),
        ];
        for (i, (qun, f, (h, rect), want)) in cases.into_iter().enumerate() {
            assert_eq!(classify(qun, f, h, &rect), want, "případ {i}");
        }
    }

    #[test]
    fn qun_value_table() {
        // (hodnota QUNS, exkluzivní, přijímá, zaneprázdněn)
        let cases = [(1, false, false, false), (2, false, false, true), (3, true, false, true), (4, true, false, true), (5, false, true, false), (6, false, false, false), (7, false, false, false)];
        for (s, ex, acc, busy) in cases {
            assert_eq!((qun_exclusive(s), qun_accepts(s), qun_busy(s)), (ex, acc, busy), "QUNS {s}");
        }
    }

    #[test]
    fn below_strip_table() {
        // (rect_top, work_top, strip_h, margin, scale) → mezera
        let cases = [
            ((0, 0, 24.0, 12.0, 1.0), 36),
            ((0, 0, 24.0, 12.0, 1.5), 54),
            // hlavní panel nahoře (work začíná 40) a proužek: aspoň margin
            ((0, 40, 24.0, 12.0, 1.0), 12),
            ((0, 30, 24.0, 12.0, 1.0), 12),
            // monitor nad primárním (záporné rect.top)
            ((-1080, -1080, 24.0, 12.0, 1.0), 36),
        ];
        for ((rt, wt, h, m, s), want) in cases {
            assert_eq!(below_strip(rt, wt, h, m, s), want, "{rt} {wt} {h} {m} {s}");
        }
    }

    #[test]
    fn display_msg_table() {
        use windows::Win32::UI::WindowsAndMessaging::{SPI_SETDESKWALLPAPER, SPI_SETWORKAREA, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_PAINT, WM_SETTINGCHANGE};
        let work = SPI_SETWORKAREA.0 as usize;
        let cases = [
            (WM_DISPLAYCHANGE, 0, true),
            (WM_DPICHANGED, 0, true),
            (WM_SETTINGCHANGE, work, true),
            (WM_SETTINGCHANGE, SPI_SETDESKWALLPAPER.0 as usize, false),
            (WM_SETTINGCHANGE, 0, false),
            (WM_PAINT, work, false),
        ];
        for (msg, wp, want) in cases {
            assert_eq!(is_display_msg(msg, wp), want, "{msg:#x} {wp}");
        }
    }

    #[test]
    fn device_name_stops_at_nul() {
        let mut d = [0u16; 32];
        for (i, c) in r"\.\DISPLAY3".encode_utf16().enumerate() {
            d[i] = c;
        }
        let m = Monitor { handle: 1, rect: r(0, 0, 1, 1), work: r(0, 0, 1, 1), primary: true, dpi: 96, device: d };
        assert_eq!(m.device_name(), r"\.\DISPLAY3");
    }

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
