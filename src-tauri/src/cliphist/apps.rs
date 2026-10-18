//! Výběr aplikací pro `clipboard.excludeApps` v Nastavení: „Přidat běžící program“. Na vyžádání
//! (klik), nikdy na pozadí: procesy s viditelným oknem (EnumWindows) + známí správci hesel, kteří
//! běží jen v oznamovací oblasti (Toolhelp). Čisté: `known_label`, `app_list` (tabulkový test).

use super::filter::exe_key;
use serde::Serialize;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GW_OWNER, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible};
use windows::core::BOOL;

/// Známé aplikace s hesly / tajemstvími: exe → lidský název.
pub const KNOWN: &[(&str, &str)] = &[
    ("keepass", "KeePass"),
    ("keepassxc", "KeePassXC"),
    ("1password", "1Password"),
    ("bitwarden", "Bitwarden"),
    ("enpass", "Enpass"),
    ("dashlane", "Dashlane"),
    ("nordpass", "NordPass"),
    ("roboform", "RoboForm"),
    ("lastpass", "LastPass"),
    ("proton pass", "Proton Pass"),
    ("keeper", "Keeper"),
];

/// Lidský název známé aplikace (`KeePassXC.exe` → „KeePassXC“), jinak `None`.
pub fn known_label(exe: &str) -> Option<&'static str> {
    let k = exe_key(exe);
    KNOWN.iter().find(|(e, _)| *e == k).map(|(_, l)| *l)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppChoice {
    /// název exe, jak se uloží do configu (`KeePassXC.exe`)
    pub exe: String,
    /// lidský název (známé aplikace), jinak exe bez přípony
    pub label: String,
    /// správce hesel ze seznamu `KNOWN`
    pub known: bool,
}

/// Jedna aplikace pro UI: exe bez cesty + lidský název (známé aplikace), jinak exe bez přípony.
pub fn choice(x: &str) -> AppChoice {
    let exe = x.rsplit(['\\', '/']).next().unwrap_or(x).trim().to_string();
    let known = known_label(&exe);
    let label = known.map(str::to_string).unwrap_or_else(|| {
        let k = exe.len().saturating_sub(4);
        if exe.get(k..).is_some_and(|s| s.eq_ignore_ascii_case(".exe")) { exe[..k].to_string() } else { exe.clone() }
    });
    AppChoice { exe, label, known: known.is_some() }
}

/// Z názvů exe (s cestou i bez) seznam pro výběr: bez duplicit (bez ohledu na velikost), bez Ledge
/// a systémových procesů, známí správci hesel napřed, jinak podle abecedy.
pub fn app_list(exes: &[String]) -> Vec<AppChoice> {
    const SKIP: &[&str] = &["ledge", "explorer", "applicationframehost", "textinputhost", "shellexperiencehost", "searchhost", "startmenuexperiencehost", "systemsettings", "lockapp", "msedgewebview2"];
    let mut out: Vec<AppChoice> = Vec::new();
    for x in exes {
        let key = exe_key(x);
        if key.is_empty() || SKIP.contains(&key.as_str()) || out.iter().any(|a| exe_key(&a.exe) == key) {
            continue;
        }
        out.push(choice(x));
    }
    out.sort_by(|a, b| b.known.cmp(&a.known).then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase())));
    out
}

/// Běžící programy pro výběr (jen na klik v Nastavení).
pub fn running() -> Vec<AppChoice> {
    let mut exes: Vec<String> = window_pids().into_iter().filter_map(crate::ports::process_image).collect();
    // správci hesel často běží jen v oznamovací oblasti (bez viditelného okna)
    exes.extend(process_names().into_iter().filter(|n| known_label(n).is_some()));
    app_list(&exes)
}

fn window_pids() -> Vec<u32> {
    unsafe extern "system" fn cb(h: HWND, data: LPARAM) -> BOOL {
        unsafe {
            let out = &mut *(data.0 as *mut Vec<u32>);
            if IsWindowVisible(h).as_bool() && GetWindow(h, GW_OWNER).is_err() && GetWindowTextLengthW(h) > 0 {
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, Some(&mut pid));
                if pid != 0 && !out.contains(&pid) {
                    out.push(pid);
                }
            }
            BOOL(1)
        }
    }
    let mut out: Vec<u32> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM((&raw mut out) as isize));
    }
    out
}

fn process_names() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snap, &mut e).is_ok();
        while ok {
            let n = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
            out.push(String::from_utf16_lossy(&e.szExeFile[..n]));
            ok = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_labels() {
        for (inp, exe, label) in [(r"C:\x\KeePass.exe", "KeePass.exe", "KeePass"), ("Code.EXE", "Code.EXE", "Code"), ("tool", "tool", "tool")] {
            let c = choice(inp);
            assert_eq!((c.exe.as_str(), c.label.as_str()), (exe, label), "{inp}");
        }
    }

    #[test]
    fn known_labels() {
        for (exe, want) in [("KeePassXC.exe", Some("KeePassXC")), (r"C:\x\1Password.exe", Some("1Password")), ("bitwarden", Some("Bitwarden")), ("notepad.exe", None), ("", None)] {
            assert_eq!(known_label(exe), want, "{exe}");
        }
    }

    #[test]
    fn list_dedup_sort_skip() {
        let exes: Vec<String> = [r"C:\Windows\notepad.exe", r"C:\Program Files\Mozilla Firefox\firefox.exe", "KeePassXC.exe", r"C:\Apps\Ledge\ledge.exe", r"C:\Windows\explorer.exe", "NOTEPAD.EXE", "Bitwarden.exe", ""].map(String::from).to_vec();
        let got: Vec<(String, String, bool)> = app_list(&exes).into_iter().map(|a| (a.exe, a.label, a.known)).collect();
        let want: Vec<(String, String, bool)> = vec![
            ("Bitwarden.exe".into(), "Bitwarden".into(), true),
            ("KeePassXC.exe".into(), "KeePassXC".into(), true),
            ("firefox.exe".into(), "firefox".into(), false),
            ("notepad.exe".into(), "notepad".into(), false),
        ];
        assert_eq!(got, want);
    }
}
