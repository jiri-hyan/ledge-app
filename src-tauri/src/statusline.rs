//! `ledge.exe --statusline` — průchozí wrapper stavového řádku Claude Code. Běží jako krátký proces
//! (spouští ho Claude Code po každé změně, debounce 300 ms), ZPRACUJE SE V `main` PŘED Tauri:
//! žádné okno, žádný single-instance zámek, žádné vlákno Ledge.
//!
//! 1. přečte stdin (JSON od Claude Code), z `rate_limits` vezme vzorek a atomicky ho zapíše do
//!    `%LOCALAPPDATA%\Ledge\ratelimits.json` (`limits::to_file`; stejné hodnoty nejvýš 1×/min),
//! 2. spustí PŮVODNÍ příkaz uživatele (`statuslinesetup::original_command`) tak jako Claude Code —
//!    přes Git Bash (`bash -c`), když je, jinak přes PowerShell (`-NoProfile -Command`) — se stejným
//!    stdin a jeho stdout předá byte po bytu; exit kód taky. Bez původního příkazu nevypíše nic.
//!
//! Nikdy nespadne (chyby tiše), původní příkaz má termín `ORIGINAL_TIMEOUT` a běží v Job Objectu
//! (KILL_ON_JOB_CLOSE): když Claude Code wrapper zruší (nová změna), umře i celý strom původního
//! příkazu. Spawn s CREATE_NO_WINDOW. GUI subsystém (release) stdout do roury předá — Claude Code
//! spouští s rourami (STARTF_USESTDHANDLES), `GetStdHandle` je vrátí i bez konzole.

use crate::limits;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Termín původního příkazu (Claude Code ho stejně zruší, když přijde další změna).
pub const ORIGINAL_TIMEOUT: Duration = Duration::from_secs(5);
/// Strop čtení stdin (vstup Claude Code má jednotky kB).
const MAX_INPUT: u64 = 4 * 1024 * 1024;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Soubor se vzorkem limitů, který wrapper píše a Ledge čte (`limitslive`).
pub fn sample_path() -> PathBuf {
    crate::actionrun::local_dir().join("ratelimits.json")
}

/// Vstup wrapperu → zapsat vzorek? Zapíše jen s `rate_limits` a jen když se liší / je starý.
pub fn record(path: &Path, input: &[u8], now: i64) -> bool {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(input) else { return false };
    let Some(s) = limits::from_status_input(&v, now) else { return false };
    let old = std::fs::read_to_string(path).ok().and_then(|t| limits::from_file(&t, limits::Source::StatusLine));
    if !limits::should_write(old.as_ref(), &s) {
        return false;
    }
    crate::statuslinesetup::write_atomic(path, limits::to_file(&s).as_bytes()).is_ok()
}

// ---------- shell jako Claude Code ----------

/// Kandidáti na Git Bash v pořadí přednosti (čistá funkce, existenci ověřuje volající):
/// `CLAUDE_CODE_GIT_BASH_PATH`, pak `bin\bash.exe` u `git.exe` z PATH (`<git>\cmd\git.exe`,
/// `<git>\bin\git.exe`, `<git>\mingw64\bin\git.exe`), pak běžné instalace. Nikdy WSL `bash.exe`
/// ze System32.
pub fn bash_candidates(env_override: Option<&str>, git_exes: &[PathBuf], program_files: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(p) = env_override.map(str::trim).filter(|p| !p.is_empty()) {
        out.push(PathBuf::from(p.trim_matches('"')));
    }
    for g in git_exes {
        let Some(dir) = g.parent() else { continue };
        let name = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let root = match name.as_str() {
            "cmd" => dir.parent(),
            "bin" => match dir.parent() {
                Some(p) if p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("mingw64")) => p.parent(),
                p => p,
            },
            _ => None,
        };
        if let Some(r) = root {
            out.push(r.join("bin").join("bash.exe"));
        }
    }
    for pf in program_files {
        out.push(pf.join("Git").join("bin").join("bash.exe"));
    }
    let mut seen = Vec::new();
    out.retain(|p| {
        let k = p.to_string_lossy().to_lowercase();
        let wsl = k.contains("\\windows\\system32\\") || k.contains("/windows/system32/");
        !wsl && !seen.contains(&k) && {
            seen.push(k);
            true
        }
    });
    out
}

/// Git Bash na tomhle PC (stejné pravidlo jako Claude Code: když je, statusLine jede přes něj).
pub fn git_bash() -> Option<PathBuf> {
    let env = std::env::var("CLAUDE_CODE_GIT_BASH_PATH").ok();
    let git_exes: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).map(|d| d.join("git.exe")).filter(|g| g.is_file()).collect())
        .unwrap_or_default();
    let pf: Vec<PathBuf> = ["ProgramFiles", "ProgramW6432", "LOCALAPPDATA"]
        .iter()
        .filter_map(|k| std::env::var_os(k).map(PathBuf::from))
        .map(|p| if p.ends_with("AppData\\Local") || p.ends_with("AppData/Local") { p.join("Programs") } else { p })
        .collect();
    bash_candidates(env.as_deref(), &git_exes, &pf).into_iter().find(|p| p.is_file())
}

/// Program a argumenty pro původní příkaz: Git Bash `-c`, jinak PowerShell `-Command`.
pub fn shell_command(cmd: &str, bash: Option<&Path>) -> (PathBuf, Vec<String>) {
    match bash {
        Some(b) => (b.to_path_buf(), vec!["-c".into(), cmd.into()]),
        None => {
            let root = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
            let ps = root.join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
            (ps, vec!["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), cmd.into()])
        }
    }
}

/// Spustí původní příkaz se vstupem `input`; vrátí (stdout beze změny, exit kód). Po termínu
/// zabije celý strom a vrátí `None`.
pub fn run_original(program: &Path, args: &[String], input: Vec<u8>, deadline: Instant) -> Option<(Vec<u8>, i32)> {
    let mut cmd = Command::new(program);
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
    let job = crate::procman::Job::new().ok();
    let mut child = match &job {
        Some(j) => j.spawn(&mut cmd, CREATE_NO_WINDOW).ok()?,
        None => {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW).spawn().ok()?
        }
    };
    let kill = |child: &mut std::process::Child| {
        if let Some(j) = &job {
            j.kill();
        }
        let _ = child.kill();
        let _ = child.wait();
    };
    if let Some(mut sin) = child.stdin.take() {
        // zápis ve vlákně: velký vstup by jinak uvázl proti plnému stdout
        std::thread::spawn(move || {
            let _ = sin.write_all(&input);
        });
    }
    let Some(mut out) = child.stdout.take() else {
        kill(&mut child);
        return None;
    };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let left = || deadline.saturating_duration_since(Instant::now());
    let Ok(buf) = rx.recv_timeout(left()) else {
        kill(&mut child);
        return None;
    };
    let code = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.code().unwrap_or(1),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(2)),
            _ => {
                kill(&mut child);
                return None;
            }
        }
    };
    Some((buf, code))
}

/// Vstupní bod `ledge.exe --statusline` (volá `main.rs`). Vrací exit kód procesu.
pub fn main() -> i32 {
    let mut input = Vec::new();
    let _ = std::io::stdin().take(MAX_INPUT).read_to_end(&mut input);
    let now = crate::state::now_ms();
    record(&sample_path(), &input, now);
    let original = crate::statuslinesetup::load_stored(&crate::statuslinesetup::store_path())
        .ok()
        .flatten()
        .as_ref()
        .and_then(crate::statuslinesetup::original_command);
    let Some(cmd) = original else { return 0 };
    let (program, args) = shell_command(&cmd, git_bash().as_deref());
    match run_original(&program, &args, input, Instant::now() + ORIGINAL_TIMEOUT) {
        Some((out, code)) => {
            let mut so = std::io::stdout().lock();
            let _ = so.write_all(&out);
            let _ = so.flush();
            code
        }
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_writes_only_with_limits_and_changes() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("Ledge").join("ratelimits.json");
        let input = br#"{"session_id":"x","rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600}}}"#;
        let t0 = 1_738_420_000_000;
        assert!(!record(&p, b"nejson", t0));
        assert!(!record(&p, br#"{"model":{}}"#, t0), "bez rate_limits nic");
        assert!(!p.exists());
        assert!(record(&p, input, t0));
        let s = limits::from_file(&std::fs::read_to_string(&p).unwrap(), limits::Source::StatusLine).unwrap();
        assert_eq!((s.at, s.five_hour.unwrap().pct, s.five_hour.unwrap().resets_at), (t0, 23.5, Some(1_738_425_600_000)));
        assert!(!record(&p, input, t0 + 1000), "stejné hodnoty hned znovu nepíše");
        assert!(record(&p, input, t0 + limits::REWRITE_SAME_MS));
        let changed = br#"{"rate_limits":{"five_hour":{"used_percentage":24,"resets_at":1738425600}}}"#;
        assert!(record(&p, changed, t0 + limits::REWRITE_SAME_MS + 1));
        // rozbitý soubor se přepíše
        std::fs::write(&p, "{rozbité").unwrap();
        assert!(record(&p, changed, t0 + limits::REWRITE_SAME_MS + 2));
    }

    #[test]
    fn bash_candidates_table() {
        let pb = PathBuf::from;
        let got = bash_candidates(
            Some(" \"D:\\Git\\bin\\bash.exe\" "),
            &[pb(r"C:\Program Files\Git\cmd\git.exe"), pb(r"E:\PortableGit\mingw64\bin\git.exe"), pb(r"F:\G\bin\git.exe"), pb(r"C:\weird\git.exe")],
            &[pb(r"C:\Program Files"), pb(r"C:\Windows\System32")],
        );
        assert_eq!(
            got,
            vec![pb(r"D:\Git\bin\bash.exe"), pb(r"C:\Program Files\Git\bin\bash.exe"), pb(r"E:\PortableGit\bin\bash.exe"), pb(r"F:\G\bin\bash.exe")],
            "duplicita a WSL ze System32 pryč"
        );
        assert!(bash_candidates(Some("  "), &[], &[]).is_empty());
    }

    #[test]
    fn shell_command_table() {
        let (p, a) = shell_command("echo hi", Some(Path::new(r"C:\Git\bin\bash.exe")));
        assert_eq!((p, a), (PathBuf::from(r"C:\Git\bin\bash.exe"), vec!["-c".to_string(), "echo hi".into()]));
        let (p, a) = shell_command("Write-Output hi", None);
        assert!(p.to_string_lossy().to_lowercase().ends_with(r"windowspowershell\v1.0\powershell.exe"));
        assert_eq!(a.last().map(String::as_str), Some("Write-Output hi"));
        assert!(a.contains(&"-NoProfile".to_string()));
    }

    /// Průchod: stdin dojde původnímu příkazu a jeho stdout se vrátí byte po bytu (bez okna).
    #[test]
    fn run_original_passes_bytes_through() {
        let Some(bash) = git_bash() else { return };
        let script = "cat; printf '\\xc4\\x8d\\r\\n'; exit 3".to_string();
        let (out, code) = run_original(&bash, &["-c".into(), script], b"{\"a\":1}".to_vec(), Instant::now() + Duration::from_secs(20)).unwrap();
        assert_eq!(out, b"{\"a\":1}\xc4\x8d\r\n");
        assert_eq!(code, 3);
        // termín: zabije a vrátí None
        let t = Instant::now();
        assert_eq!(run_original(&bash, &["-c".into(), "sleep 30".into()], vec![], Instant::now() + Duration::from_millis(500)), None);
        assert!(t.elapsed() < Duration::from_secs(10));
    }
}
