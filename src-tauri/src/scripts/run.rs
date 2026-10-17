//! Jak skript spustit: interpret z PATH (`find_on_path` — aliasy Microsoft Storu z `WindowsApps`
//! se přeskočí, jinak by `python.exe` otevřel Store), program + argumenty (`plan`, bez shellu
//! s uživatelským vstupem: PowerShell `-Command "& 'cesta' 'arg'"`, `cmd /c ""cesta" arg"`,
//! Python/Node přímo), otisk pro souhlas (`fingerprint` — cesta, SHA-256 obsahu, režim, argumenty)
//! a text potvrzení. Čisté kromě `find_on_path` (stat) a `command`.

use super::meta::{Lang, Mode};
use crate::actions::{quote_cmd, quote_ps};
use crate::i18n::tr;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Čím se skript spustí.
#[derive(Debug, Clone, PartialEq)]
pub enum Interp {
    /// `powershell` (Windows PowerShell, je v systému vždy)
    Powershell,
    /// `cmd` (systém)
    Cmd,
    /// `.exe` interpretu z PATH (Python, Node)
    Exe(PathBuf),
}

/// První `name` v adresářích `path_var` (proměnná PATH), který je soubor. Složky `WindowsApps`
/// (aliasy Storu) se přeskočí.
pub fn find_on_path(names: &[&str], path_var: &OsStr) -> Option<PathBuf> {
    let dirs: Vec<PathBuf> = std::env::split_paths(path_var).filter(|d| !d.as_os_str().is_empty()).collect();
    for name in names {
        for d in &dirs {
            if d.to_string_lossy().to_ascii_lowercase().contains(r"\microsoft\windowsapps") {
                continue;
            }
            let p = d.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Interpret pro jazyk. `Err` = není na PATH (skript se ukáže s problémem).
pub fn interp(lang: Lang, path_var: &OsStr) -> Result<Interp, String> {
    match lang {
        Lang::Powershell => Ok(Interp::Powershell),
        Lang::Cmd => Ok(Interp::Cmd),
        // `py` (spouštěč Pythonu) respektuje shebang; pak klasický python.exe
        Lang::Python => find_on_path(&["py.exe", "python.exe", "python3.exe"], path_var).map(Interp::Exe).ok_or_else(|| tr("Python není na PATH", "Python isn't on PATH").into()),
        Lang::Node => find_on_path(&["node.exe"], path_var).map(Interp::Exe).ok_or_else(|| tr("Node.js není na PATH", "Node.js isn't on PATH").into()),
    }
}

/// Program a argumenty. `raw` = surový řetězec pro `cmd` (vlastní pravidla uvozovek).
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub program: PathBuf,
    pub raw: Option<String>,
    pub args: Vec<String>,
    pub env: Vec<(&'static str, &'static str)>,
}

impl Plan {
    pub fn command(&self, dir: &Path) -> Command {
        let mut cmd = Command::new(&self.program);
        if let Some(raw) = &self.raw {
            use std::os::windows::process::CommandExt;
            cmd.raw_arg(raw);
        }
        cmd.args(&self.args).current_dir(dir);
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd
    }
}

/// `%` (rozbalí se i v uvozovkách) a `"` nejdou do řádky `cmd` bezpečně.
fn cmd_safe(p: &Path) -> Result<String, String> {
    let s = p.to_string_lossy().into_owned();
    if s.contains(['%', '"', '\n', '\r']) {
        Err(trf!("cesta „{s}“ obsahuje % nebo uvozovky — přejmenuj soubor / složku", "the path “{s}” contains % or quotes — rename the file / folder"))
    } else {
        Ok(s)
    }
}

fn cmd_line(parts: &[String]) -> String {
    parts.iter().enumerate().map(|(i, p)| if i == 0 { format!("\"{p}\"") } else { quote_cmd(p) }).collect::<Vec<_>>().join(" ")
}

/// Jak spustit `script` s `args`. `terminal` = viditelné okno, které po doběhnutí zůstane.
pub fn plan(script: &Path, interp: &Interp, args: &[String], terminal: bool) -> Result<Plan, String> {
    if args.iter().any(|a| a.contains(['"', '\n', '\r', '\0'])) {
        return Err(tr("argumenty bez uvozovek a nových řádků", "arguments without quotes and new lines").into());
    }
    let ps_call = || {
        let mut line = format!("& {}", quote_ps(&script.to_string_lossy()));
        for a in args {
            line.push(' ');
            line.push_str(&quote_ps(a));
        }
        line
    };
    let cmd_wrap = |parts: Vec<String>, keep: bool| Plan {
        program: "cmd".into(),
        raw: Some(if keep { format!("/d /s /k \"{}\"", cmd_line(&parts)) } else { format!("/d /s /c \"chcp 65001>nul & {}\"", cmd_line(&parts)) }),
        args: vec![],
        env: vec![],
    };
    Ok(match (interp, terminal) {
        (Interp::Powershell, false) => Plan {
            program: "powershell".into(),
            raw: None,
            args: ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"].map(String::from).into_iter().chain([format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {}; exit $LASTEXITCODE", ps_call())]).collect(),
            env: vec![],
        },
        (Interp::Powershell, true) => Plan {
            program: "powershell".into(),
            raw: None,
            args: ["-NoProfile", "-NoExit", "-ExecutionPolicy", "Bypass", "-Command"].map(String::from).into_iter().chain([ps_call()]).collect(),
            env: vec![],
        },
        (Interp::Cmd, keep) => cmd_wrap(std::iter::once(cmd_safe(script)?).chain(args.iter().cloned()).collect(), keep),
        (Interp::Exe(exe), false) => Plan {
            program: exe.clone(),
            raw: None,
            args: std::iter::once(script.to_string_lossy().into_owned()).chain(args.iter().cloned()).collect(),
            env: vec![("PYTHONIOENCODING", "utf-8"), ("PYTHONUTF8", "1")],
        },
        // okno musí po doběhnutí zůstat → přes `cmd /k`
        (Interp::Exe(exe), true) => cmd_wrap([cmd_safe(exe)?, cmd_safe(script)?].into_iter().chain(args.iter().cloned()).collect(), true),
    })
}

/// SHA-256 (hex) bajtů.
pub fn sha256_hex(h: sha2::Sha256) -> String {
    use sha2::Digest;
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Otisk pro souhlas: cesta (malými), obsah, režim, argumenty. Jiný prefix než `actions::fingerprint`,
/// takže se s otiskem akce nikdy nepotká. Změna souboru = nový otisk = nové potvrzení.
pub fn fingerprint(path: &Path, content_sha: &str, mode: Mode, args: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for part in ["ledge-script", &path.to_string_lossy().to_lowercase(), content_sha, mode.key()] {
        h.update(part.as_bytes());
        h.update([0u8]);
    }
    for a in args {
        h.update(a.as_bytes());
        h.update([1u8]);
    }
    sha256_hex(h)
}

/// Co člověk uvidí v potvrzení (a co musí UI poslat zpátky): jazyk, cesta, argumenty, začátek
/// otisku obsahu. Jiný obsah = jiný text → staré potvrzení neplatí.
pub fn confirm_text(lang: Lang, path: &Path, args: &[String], content_sha: &str) -> String {
    let mut s = format!("{}: {}", lang.name(), path.display());
    for a in args {
        s.push(' ');
        s.push_str(&quote_cmd(a));
    }
    let short = content_sha.get(..12).unwrap_or(content_sha);
    s.push_str(&format!(" · SHA-256 {short}…"));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn plan_table() {
        let a = vec!["--x".to_string(), "a b".to_string()];
        let s = p(r"C:\Users\x\Ledge\scripts\čisti temp.ps1");
        let ps = plan(&s, &Interp::Powershell, &a, false).unwrap();
        assert_eq!(ps.program, p("powershell"));
        assert!(ps.args.contains(&"Bypass".to_string()) && ps.args.contains(&"-NonInteractive".to_string()));
        assert!(ps.args.last().unwrap().contains(r"& 'C:\Users\x\Ledge\scripts\čisti temp.ps1' --x 'a b'; exit $LASTEXITCODE"), "{:?}", ps.args);
        let pst = plan(&s, &Interp::Powershell, &a, true).unwrap();
        assert!(pst.args.contains(&"-NoExit".to_string()));

        let c = p(r"C:\s\zaloha.cmd");
        assert_eq!(plan(&c, &Interp::Cmd, &a, false).unwrap().raw.as_deref(), Some(r#"/d /s /c "chcp 65001>nul & "C:\s\zaloha.cmd" --x "a b"""#));
        assert_eq!(plan(&c, &Interp::Cmd, &[], true).unwrap().raw.as_deref(), Some(r#"/d /s /k ""C:\s\zaloha.cmd"""#));
        assert!(plan(&p(r"C:\s\100%.cmd"), &Interp::Cmd, &[], false).is_err());

        let py = Interp::Exe(p(r"C:\Python\py.exe"));
        let h = plan(&p(r"C:\s\a.py"), &py, &a, false).unwrap();
        assert_eq!((h.program, h.raw, h.args), (p(r"C:\Python\py.exe"), None, vec![r"C:\s\a.py".to_string(), "--x".into(), "a b".into()]));
        assert!(h.env.contains(&("PYTHONIOENCODING", "utf-8")));
        let t = plan(&p(r"C:\s\a.py"), &py, &[], true).unwrap();
        assert_eq!(t.raw.as_deref(), Some(r#"/d /s /k ""C:\Python\py.exe" C:\s\a.py""#));
        assert!(plan(&c, &Interp::Cmd, &["a\"b".into()], false).is_err());
    }

    #[test]
    fn path_lookup_skips_store_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("Microsoft").join("WindowsApps");
        let real = dir.path().join("Python312");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(store.join("python.exe"), b"").unwrap();
        let path = std::env::join_paths([store.clone(), real.clone()]).unwrap();
        assert_eq!(find_on_path(&["python.exe"], &path), None, "alias Storu se nepočítá");
        std::fs::write(real.join("python.exe"), b"").unwrap();
        assert_eq!(find_on_path(&["py.exe", "python.exe"], &path), Some(real.join("python.exe")));
        assert!(interp(Lang::Node, &path).unwrap_err().contains("Node"));
        assert_eq!(interp(Lang::Powershell, OsStr::new("")), Ok(Interp::Powershell));
    }

    #[test]
    fn fingerprint_changes_with_content_mode_args() {
        let s = p(r"C:\s\a.ps1");
        let base = fingerprint(&s, "abc", Mode::Output, &[]);
        assert_eq!(base, fingerprint(&p(r"C:\S\A.PS1"), "abc", Mode::Output, &[]), "cesta bez ohledu na velikost písmen");
        assert_ne!(base, fingerprint(&s, "abd", Mode::Output, &[]));
        assert_ne!(base, fingerprint(&s, "abc", Mode::Terminal, &[]));
        assert_ne!(base, fingerprint(&s, "abc", Mode::Output, &["x".into()]));
        let t = confirm_text(Lang::Powershell, &s, &["a b".into()], "0123456789abcdef");
        assert_eq!(t, r#"PowerShell: C:\s\a.ps1 "a b" · SHA-256 0123456789ab…"#);
    }
}
