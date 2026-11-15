//! Stop aplikace z karty (L238): aplikace NEJDE do Job Objectu (má přežít restart Ledge), proto ji nejde zabít
//! jobem. Zavře se jako člověk: `WM_CLOSE` oknům procesu, po lhůtě `TerminateProcess`. Nikdy naslepo podle PID:
//! při spuštění se zapíše otisk (PID + čas startu + cesta exe) a před zásahem se proti otevřenému procesu ověří
//! — PID se po skončení procesu recykluje, cizí proces se nesmí zavřít. Zabíjí se držený ověřený handle.

use std::os::windows::io::AsRawHandle;
use std::process::Child;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, FILETIME, GetLastError, HANDLE, HWND, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE};
use windows::core::{BOOL, PWSTR};

/// Lhůta na slušné zavření, než se proces ukončí natvrdo (aplikace může ukázat „Uložit?“).
pub(crate) const CLOSE_GRACE: Duration = Duration::from_secs(3);

/// Otisk spuštěného procesu: PID + čas startu (FILETIME) + celá cesta exe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppIdent {
    pub pid: u32,
    pub started: u64,
    pub image: String,
}

/// Je otevřený proces (čas startu, cesta) tím, který jsme spustili? Bez času startu nebo cesty se neověří = ne.
pub(crate) fn same_process(rec: &AppIdent, started: u64, image: &str) -> bool {
    rec.started != 0 && !rec.image.is_empty() && rec.started == started && rec.image.eq_ignore_ascii_case(image)
}

/// Čím se Stop provede.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StopKind {
    /// strom v Job Objectu (dev server) — `Job::kill`
    Job,
    /// aplikace bez jobu s ověřitelným otiskem — `WM_CLOSE` → kill
    App,
    /// není co zastavit (záznam bez procesu: příprava, otisk nejde zjistit) — jen zmizí z přehledu
    Entry,
}

pub(crate) fn stop_kind(has_job: bool, app: Option<&AppIdent>) -> StopKind {
    match (has_job, app) {
        (true, _) => StopKind::Job,
        (false, Some(_)) => StopKind::App,
        (false, None) => StopKind::Entry,
    }
}

/// Jak dopadlo zavření.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Closed {
    /// odešel sám po `WM_CLOSE`
    Graceful,
    /// po lhůtě ukončen
    Killed,
    /// už neběžel
    Gone,
    /// PID drží jiný proces (recyklace) nebo identita nejde ověřit — nic se nestalo
    Mismatch,
    /// proces nejde otevřít / ukončit (např. běží zvýšeně)
    Denied,
}

fn ft(f: FILETIME) -> u64 {
    (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)
}

/// (čas startu, cesta exe) procesu z handlu s právem `PROCESS_QUERY_LIMITED_INFORMATION`.
fn identify(h: HANDLE) -> Option<(u64, String)> {
    unsafe {
        let (mut c, mut e, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).ok()?;
        Some((ft(c), String::from_utf16_lossy(&buf[..len as usize])))
    }
}

/// Otisk právě spuštěného dítěte (z jeho vlastního handlu — PID tedy nejde zaměnit).
pub(crate) fn identify_child(child: &Child) -> Option<AppIdent> {
    let (started, image) = identify(HANDLE(child.as_raw_handle()))?;
    Some(AppIdent { pid: child.id(), started, image })
}

/// Pošle `WM_CLOSE` viditelným oknům nejvyšší úrovně procesu `pid`.
fn post_close(pid: u32) {
    unsafe extern "system" fn cb(h: HWND, lp: LPARAM) -> BOOL {
        unsafe {
            let mut p = 0u32;
            GetWindowThreadProcessId(h, Some(&mut p));
            if p == lp.0 as u32 && IsWindowVisible(h).as_bool() {
                let _ = PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        BOOL(1)
    }
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(pid as isize));
    }
}

/// Zavře aplikaci: ověří otisk proti otevřenému procesu, `WM_CLOSE`, po `grace` `TerminateProcess` na tentýž
/// ověřený handle. Blokuje do `grace` — volej z vlákna, ne z UI.
pub(crate) fn close_then_kill(rec: &AppIdent, grace: Duration) -> Closed {
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, rec.pid) else {
            // PID už neexistuje (proces skončil), jinak k němu nemáme právo
            return if GetLastError() == ERROR_INVALID_PARAMETER { Closed::Gone } else { Closed::Denied };
        };
        let res = match identify(h) {
            Some((started, image)) if same_process(rec, started, &image) => {
                if WaitForSingleObject(h, 0) == WAIT_OBJECT_0 {
                    Closed::Gone
                } else {
                    post_close(rec.pid);
                    if WaitForSingleObject(h, grace.as_millis() as u32) == WAIT_OBJECT_0 {
                        Closed::Graceful
                    } else if TerminateProcess(h, 1).is_ok() {
                        Closed::Killed
                    } else {
                        Closed::Denied
                    }
                }
            }
            _ => Closed::Mismatch,
        };
        let _ = CloseHandle(h);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec() -> AppIdent {
        AppIdent { pid: 42, started: 1000, image: r"C:\Apps\App.exe".into() }
    }

    #[test]
    fn same_process_table() {
        let r = rec();
        let cases = [
            (1000, r"C:\Apps\App.exe", true),
            (1000, r"c:\apps\app.EXE", true),    // Windows cesty nerozlišují velikost
            (1001, r"C:\Apps\App.exe", false),   // PID recyklovaný: jiný čas startu
            (1000, r"C:\Apps\Other.exe", false), // stejný čas, jiný obraz
            (0, r"C:\Apps\App.exe", false),
        ];
        for (started, image, want) in cases {
            assert_eq!(same_process(&r, started, image), want, "{started} {image}");
        }
        // bez otisku (čas 0 / prázdná cesta) se nikdy neověří
        let blank = AppIdent { started: 0, ..rec() };
        assert!(!same_process(&blank, 0, r"C:\Apps\App.exe"));
        let noimg = AppIdent { image: String::new(), ..rec() };
        assert!(!same_process(&noimg, 1000, ""));
    }

    #[test]
    fn stop_kind_table() {
        let r = rec();
        assert_eq!(stop_kind(true, None), StopKind::Job);
        assert_eq!(stop_kind(true, Some(&r)), StopKind::Job);
        assert_eq!(stop_kind(false, Some(&r)), StopKind::App);
        assert_eq!(stop_kind(false, None), StopKind::Entry);
    }

    /// Cizí PID (tenhle testovací proces s vymyšleným časem startu) se nezavře: otisk nesedí.
    /// Otisk z handlu dítěte má skutečný čas startu a cestu.
    #[test]
    fn foreign_pid_is_never_touched() {
        let me = AppIdent { pid: std::process::id(), started: 1, image: std::env::current_exe().unwrap().to_string_lossy().into_owned() };
        assert_eq!(close_then_kill(&me, Duration::from_millis(50)), Closed::Mismatch);
        let mut child = std::process::Command::new("cmd").args(["/C", "exit", "0"]).spawn().unwrap();
        let id = identify_child(&child).expect("otisk dítěte");
        let _ = child.wait();
        assert_eq!(id.pid, child.id());
        assert!(id.started != 0 && id.image.to_lowercase().ends_with("cmd.exe"), "{id:?}");
    }
}
