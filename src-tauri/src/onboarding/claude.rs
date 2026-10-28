//! Průvodce, krok Claude Code: je nainstalovaný (`claude` na PATH nebo v `~\.local\bin`), existuje
//! `~\.claude`, jaká verze (`claude --version` bez okna, termín 8 s — jen v appce, nikdy v testech).
//! Hooky řeší existující `hooksetup.rs` / příkaz `hooks_install`.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeInfo {
    /// cesta ke spustitelnému `claude` (`None` = nenalezen)
    pub exe: Option<String>,
    /// složka `~\.claude` existuje (Claude Code tu už běžel)
    pub dir: bool,
    /// verze z `claude --version`; `None` = nevím
    pub version: Option<String>,
    /// je co zapojit: spustitelný `claude` nebo složka `~\.claude`
    pub installed: bool,
}

/// `claude.exe` / `.cmd` (npm) v PATH, záložně nativní instalace `%USERPROFILE%\.local\bin`.
/// Jen absolutní složky (nikdy cwd). Čistá nad `exists`.
pub fn find_exe(path_var: Option<&str>, home: Option<&str>, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = path_var.map(|p| std::env::split_paths(p).filter(|d| d.is_absolute()).collect()).unwrap_or_default();
    if let Some(h) = home.filter(|h| !h.trim().is_empty()) {
        dirs.push(Path::new(h).join(".local").join("bin"));
    }
    dirs.iter().flat_map(|d| ["claude.exe", "claude.cmd"].map(|n| d.join(n))).find(|p| exists(p))
}

/// První `x.y.z` ve výstupu `claude --version` („2.1.3 (Claude Code)“).
pub fn parse_version(out: &str) -> Option<String> {
    out.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == 'v')
        .find(|w| {
            let parts: Vec<&str> = w.split('.').collect();
            parts.len() >= 3 && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(str::to_string)
}

fn version_of(exe: &Path) -> Option<String> {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--version").current_dir(std::env::temp_dir());
    let out = crate::gitinfo::capture(&mut cmd, None, false, Instant::now() + Duration::from_secs(8)).ok()?;
    out.success.then(|| parse_version(&out.stdout)).flatten()
}

/// Detekce na skutečném stroji (spouští `claude --version`).
pub fn detect() -> ClaudeInfo {
    let home = crate::dirs::home().map(|h| h.to_string_lossy().into_owned());
    let exe = find_exe(std::env::var("PATH").ok().as_deref(), home.as_deref(), |p| p.is_file());
    let dir = crate::sessionlive::claude_dir().is_some_and(|d| d.is_dir());
    let version = exe.as_deref().and_then(version_of);
    ClaudeInfo { installed: exe.is_some() || dir, exe: exe.map(|p| p.to_string_lossy().into_owned()), dir, version }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_exe_table() {
        let has = |set: &'static [&'static str]| move |p: &Path| set.iter().any(|s| Path::new(s) == p);
        let path = r"C:\nic;relativni;C:\npm";
        assert_eq!(find_exe(Some(path), None, has(&[r"C:\npm\claude.cmd"])), Some(PathBuf::from(r"C:\npm\claude.cmd")));
        assert_eq!(find_exe(Some(path), Some(r"C:\Users\ada"), has(&[r"C:\Users\ada\.local\bin\claude.exe"])), Some(PathBuf::from(r"C:\Users\ada\.local\bin\claude.exe")));
        assert_eq!(find_exe(Some(path), None, has(&[r"relativni\claude.exe"])), None, "relativní složka z PATH se ignoruje");
        assert_eq!(find_exe(None, None, has(&[])), None);
    }

    #[test]
    fn version_table() {
        for (o, v) in [("2.1.3 (Claude Code)\n", Some("2.1.3")), ("claude v1.0.44", Some("1.0.44")), ("nevim", None), ("1.2", None)] {
            assert_eq!(parse_version(o).as_deref(), v, "{o}");
        }
    }
}
