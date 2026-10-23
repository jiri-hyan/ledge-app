//! Win32 pro klik na session v kartě Claude (`session_open` v `lib.rs`): strom procesů (ToolHelp),
//! viditelná top-level okna (EnumWindows), přepnutí do okna a `claude --resume` v novém terminálu.
//! Volá se JEN z výslovného kliku uživatele (Windows pak `SetForegroundWindow` dovolí). Rozhodnutí
//! „co udělat“ je čisté v `sessions::decide_open` / `sessions::resume_command`.

use crate::i18n::tr;
use crate::sessions::Launch;
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, STILL_ACTIVE};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId, GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GW_OWNER, GetForegroundWindow, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE, SetForegroundWindow,
    ShowWindow,
};
use windows::core::BOOL;

/// Procesy, u kterých hledání okna končí (shell / služby — jejich okno není „okno session“).
const STOP_AT: &[&str] = &["explorer.exe", "sihost.exe", "svchost.exe", "services.exe", "wininit.exe", "winlogon.exe", "userinit.exe"];
/// Hloubka stromu: claude → shell → OpenConsole/pty host → terminál/editor (+ rezerva).
const MAX_DEPTH: usize = 8;

/// pid → (rodič, jméno exe malými písmeny)
pub type ProcTable = HashMap<u32, (u32, String)>;

pub fn pid_alive(pid: u32) -> bool {
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code).is_ok() && code == STILL_ACTIVE.0 as u32;
        let _ = CloseHandle(h);
        ok
    }
}

fn process_table() -> ProcTable {
    let mut t = HashMap::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return t };
        let mut pe = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(snap, &mut pe).is_ok();
        while more {
            let len = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len());
            t.insert(pe.th32ProcessID, (pe.th32ParentProcessID, String::from_utf16_lossy(&pe.szExeFile[..len]).to_lowercase()));
            more = Process32NextW(snap, &mut pe).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    t
}

/// `pid` a jeho předci (nejbližší první) až po shell/službu, smyčku nebo `skip` (Ledge sám).
pub fn ancestors(table: &ProcTable, pid: u32, skip: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut cur = pid;
    while out.len() < MAX_DEPTH && cur != 0 && cur != skip && !out.contains(&cur) {
        let Some((parent, name)) = table.get(&cur) else { break };
        if STOP_AT.contains(&name.as_str()) {
            break;
        }
        out.push(cur);
        cur = *parent;
    }
    out
}

/// První proces v řetězci, který má okno → jeho první okno. `windows` = (hwnd, pid) v Z-pořadí.
pub fn pick_window(chain: &[u32], windows: &[(isize, u32)]) -> Option<isize> {
    chain.iter().find_map(|p| windows.iter().find(|(_, wp)| wp == p).map(|(h, _)| *h))
}

/// Viditelná, nevlastněná, neschovaná (cloaked) top-level okna s titulkem, v Z-pořadí.
/// Konzolová okna conhost hlásí PID klientského procesu → najdou se přes řetězec předků.
fn top_windows() -> Vec<(isize, u32)> {
    unsafe extern "system" fn cb(h: HWND, data: LPARAM) -> BOOL {
        unsafe {
            let out = &mut *(data.0 as *mut Vec<(isize, u32)>);
            if IsWindowVisible(h).as_bool() && GetWindow(h, GW_OWNER).is_err() && GetWindowTextLengthW(h) > 0 {
                let mut cloaked = 0u32;
                let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, (&raw mut cloaked).cast(), 4);
                if cloaked == 0 {
                    let mut pid = 0u32;
                    GetWindowThreadProcessId(h, Some(&mut pid));
                    out.push((h.0 as isize, pid));
                }
            }
            BOOL(1)
        }
    }
    let mut out: Vec<(isize, u32)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM((&raw mut out) as isize));
    }
    out
}

fn focus(hwnd: isize) -> Result<(), String> {
    let h = HWND(hwnd as *mut _);
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        if SetForegroundWindow(h).as_bool() {
            return Ok(());
        }
        // klik přijal proces WebView2, ne ledge.exe → foreground lock; připojit se ke vstupu okna
        // v popředí (jen tady, na výslovný klik) a zkusit znovu
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let _ = BringWindowToTop(h);
        let ok = SetForegroundWindow(h).as_bool();
        if attached {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
        if ok { Ok(()) } else { Err(tr("Windows nedovolil přepnout do okna — klikni znovu.", "Windows didn't allow switching to the window — click again.").into()) }
    }
}

/// Je to desktopová appka Claude (MSIX `WindowsApps\Claude_…` nebo instalace `AnthropicClaude`)?
pub fn is_claude_desktop_image(path: &str) -> bool {
    let p = path.to_lowercase().replace('/', "\\");
    p.ends_with("\\claude.exe") && (p.contains("\\windowsapps\\claude_") || p.contains("\\anthropicclaude\\"))
}

/// Přepne do okna, ve kterém běží session `pid`. `desktop` = session z desktopové appky Claude:
/// když strom předků okno nemá, zkusí okno kterékoli běžící instance appky.
pub fn switch_to(pid: u32, desktop: bool) -> Result<(), String> {
    if !pid_alive(pid) {
        return Err(tr("Proces session už neběží — obnov seznam.", "The session's process isn't running any more — refresh the list.").into());
    }
    let table = process_table();
    let wins = top_windows();
    let chain = ancestors(&table, pid, std::process::id());
    if let Some(h) = pick_window(&chain, &wins) {
        return focus(h);
    }
    if desktop {
        let apps: Vec<u32> = table
            .iter()
            .filter(|(_, (_, name))| name == "claude.exe")
            .filter(|(p, _)| crate::ports::process_image(**p).is_some_and(|i| is_claude_desktop_image(&i)))
            .map(|(p, _)| *p)
            .collect();
        if let Some(h) = pick_window(&apps, &wins) {
            return focus(h);
        }
        return Err(tr("Okno Claude není otevřené (appka je asi jen v oznamovací oblasti).", "The Claude window isn't open (the app is probably only in the notification area).").into());
    }
    Err(tr("Nenašel jsem okno terminálu, ve kterém session běží.", "I didn't find the terminal window the session runs in.").into())
}

/// Spustí `claude --resume` v novém okně: Windows Terminal, bez něj `cmd /c start`.
pub fn launch(cwd: &str, id: &str) -> Result<(), String> {
    const DETACHED: u32 = 0x0000_0008;
    const NO_WINDOW: u32 = 0x0800_0000;
    if !std::path::Path::new(cwd).is_dir() {
        return Err(trf!("Složka session neexistuje: {cwd}", "The session folder doesn't exist: {cwd}"));
    }
    let bad = || tr("Session nejde bezpečně obnovit (neobvyklé id nebo složka).", "The session can't be resumed safely (unusual id or folder).").to_string();
    if let Some(Launch::Wt(args)) = crate::sessions::resume_command(true, cwd, id)
        && std::process::Command::new("wt").args(&args).creation_flags(DETACHED).spawn().is_ok()
    {
        return Ok(());
    }
    match crate::sessions::resume_command(false, cwd, id).ok_or_else(bad)? {
        Launch::Cmd(raw) => std::process::Command::new("cmd").raw_arg(raw).creation_flags(NO_WINDOW).spawn().map(|_| ()).map_err(|e| crate::errors::spawn(crate::i18n::tr("Terminál nejde otevřít", "Can't open a terminal"), &e)),
        Launch::Wt(_) => Err(bad()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[(u32, u32, &str)]) -> ProcTable {
        rows.iter().map(|(p, pp, n)| (*p, (*pp, n.to_string()))).collect()
    }

    #[test]
    fn ancestors_table() {
        // desktop: claude.exe (CLI) → Claude.exe (appka) → sihost
        let t = table(&[(12148, 13180, "claude.exe"), (13180, 6716, "claude.exe"), (6716, 2332, "sihost.exe"), (2332, 1480, "svchost.exe")]);
        assert_eq!(ancestors(&t, 12148, 999), [12148, 13180]);
        // terminál: claude → pwsh → WindowsTerminal → explorer
        let t = table(&[(5, 4, "claude.exe"), (4, 3, "pwsh.exe"), (3, 2, "windowsterminal.exe"), (2, 1, "explorer.exe")]);
        assert_eq!(ancestors(&t, 5, 999), [5, 4, 3]);
        // Ledge sám (resume spuštěný z Ledge) se nepočítá
        assert_eq!(ancestors(&t, 5, 3), [5, 4]);
        // smyčka (recyklované PID) a neznámý pid
        let t = table(&[(1, 2, "a.exe"), (2, 1, "b.exe")]);
        assert_eq!(ancestors(&t, 1, 0), [1, 2]);
        assert!(ancestors(&t, 77, 0).is_empty());
        // hloubka
        let deep: Vec<(u32, u32, &str)> = (1..30).map(|i| (i, i + 1, "x.exe")).collect();
        assert_eq!(ancestors(&table(&deep), 1, 0).len(), MAX_DEPTH);
    }

    #[test]
    fn pick_window_prefers_nearest() {
        let wins = [(100, 3), (200, 4), (300, 9)];
        assert_eq!(pick_window(&[5, 4, 3], &wins), Some(200));
        assert_eq!(pick_window(&[5], &wins), None);
        assert_eq!(pick_window(&[], &wins), None);
    }

    #[test]
    fn desktop_image() {
        assert!(is_claude_desktop_image(r"C:\Program Files\WindowsApps\Claude_2.42304.0.0_x64__pzs8sxrjxfjjc\app\claude.exe"));
        assert!(is_claude_desktop_image(r"C:\Users\x\AppData\Local\AnthropicClaude\app-1.0\claude.exe"));
        // CLI ve složce appky (2.1.299) není okno appky
        assert!(!is_claude_desktop_image(r"C:\Users\x\AppData\Roaming\Claude\claude-code\2.1.299\f0\claude.exe"));
        assert!(!is_claude_desktop_image(r"C:\Program Files\WindowsApps\Claude_2\app\other.exe"));
    }

    #[test]
    fn own_process_is_alive() {
        assert!(pid_alive(std::process::id()));
        assert!(!pid_alive(0xFFFF_FFF0));
    }
}
