//! Větev, neuložené změny a ahead/behind — jen na vyžádání (otevřený šuplík), s cache.

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub branch: Option<String>,
    pub dirty: u32,
    pub ahead: u32,
    pub behind: u32,
}

pub fn parse_porcelain_v2(out: &str) -> GitInfo {
    let mut g = GitInfo::default();
    for line in out.lines() {
        if let Some(b) = line.strip_prefix("# branch.head ") {
            g.branch = if b == "(detached)" { None } else { Some(b.to_string()) };
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for part in ab.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    g.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    g.behind = n.parse().unwrap_or(0);
                }
            }
        } else if !line.starts_with('#') && !line.is_empty() {
            g.dirty += 1;
        }
    }
    g
}

pub fn status(dir: &Path) -> Option<GitInfo> {
    let out = Command::new("git")
        .args(["status", "--porcelain=v2", "--branch", "--untracked-files=normal"])
        .current_dir(dir)
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(parse_porcelain_v2(&String::from_utf8_lossy(&out.stdout)))
}

/// `git <args>` v `dir` s tvrdým termínem (hook Claude Code čeká max. 5 s). Po termínu zabije celý
/// strom gitu (Job Object) a vrátí `None`, stejně jako při chybě nebo nenulovém exit kódu.
/// Vždy s `core.fsmonitor=false` — cwd přichází z hooku a fsmonitor by mohl spustit cizí program.
pub fn run(dir: &Path, args: &[&str], deadline: Instant) -> Option<String> {
    run_with_input(dir, args, None, deadline)
}

/// Jako [`run`], `input` jde gitu na stdin (např. pathspecy pro `rev-list --stdin`).
pub fn run_with_input(dir: &Path, args: &[&str], input: Option<&str>, deadline: Instant) -> Option<String> {
    let out = capture(&mut git_cmd(dir, args), input, false, deadline).ok()?;
    out.success.then_some(out.stdout)
}

/// Jako [`run`], navíc proměnné prostředí `env` a vrací i stderr a neúspěšný exit kód
/// (synchronizace poznámek podle stderr rozlišuje offline / nepřihlášeno).
pub fn run_env(dir: &Path, args: &[&str], env: &[(&str, &str)], deadline: Instant) -> Result<Captured, RunError> {
    let mut cmd = git_cmd(dir, args);
    cmd.envs(env.iter().copied());
    capture(&mut cmd, None, true, deadline)
}

/// `git -c core.fsmonitor=false <args>` v `dir`.
fn git_cmd(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "core.fsmonitor=false"]).args(args).current_dir(dir);
    cmd
}

/// Výsledek [`capture`]: proces doběhl (s jakýmkoli exit kódem).
#[derive(Debug)]
pub struct Captured {
    pub success: bool,
    pub stdout: String,
    /// prázdné, když volající stderr nechtěl
    pub stderr: String,
}

/// Proč [`capture`] nevrátil výstup.
#[derive(Debug, PartialEq, Eq)]
pub enum RunError {
    /// termín prošel (před spuštěním nebo během běhu — strom procesu je zabitý)
    Timeout,
    /// proces nešel spustit
    Spawn(String),
}

/// Obecné spuštění programu bez okna (CREATE_NO_WINDOW) s tvrdým termínem: po termínu zabije celý
/// strom (Job Object) a vrátí [`RunError::Timeout`]. `cmd` připraví volající (program, argumenty,
/// cwd, env); stdin/stdout/stderr nastaví tahle funkce. `input` jde na stdin, `stderr` = číst i stderr.
/// Jediné místo, které proces opravdu spouští — `run`, `run_with_input`, `run_env` jsou obálky.
pub fn capture(cmd: &mut Command, input: Option<&str>, stderr: bool, deadline: Instant) -> Result<Captured, RunError> {
    capture_tail(cmd, input, stderr, deadline, None)
}

/// Přečte rouru do konce; s `cap` drží v paměti jen posledních `cap` bajtů (ukecaný příkaz rychlé
/// akce nesmí nafouknout Ledge). Vrací (data, bylo useknuto?).
pub fn read_tail(r: &mut impl std::io::Read, cap: Option<usize>) -> (Vec<u8>, bool) {
    let Some(cap) = cap else {
        let mut buf = Vec::new();
        let _ = r.read_to_end(&mut buf);
        return (buf, false);
    };
    let mut buf: Vec<u8> = Vec::with_capacity(cap.min(64 * 1024));
    let mut chunk = [0u8; 16 * 1024];
    let mut cut = false;
    loop {
        match r.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        if buf.len() > cap.saturating_mul(2).max(cap + chunk.len()) {
            buf.drain(..buf.len() - cap);
            cut = true;
        }
    }
    if buf.len() > cap {
        buf.drain(..buf.len() - cap);
        cut = true;
    }
    (buf, cut)
}

/// Jako [`capture`], ale ze stdout i stderr drží jen posledních `cap` bajtů (viz [`read_tail`]).
pub fn capture_tail(cmd: &mut Command, input: Option<&str>, stderr: bool, deadline: Instant, cap: Option<usize>) -> Result<Captured, RunError> {
    use std::io::Write;
    use std::process::Stdio;
    if Instant::now() >= deadline {
        return Err(RunError::Timeout);
    }
    cmd.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(if stderr { Stdio::piped() } else { Stdio::null() });
    // KILL_ON_JOB_CLOSE: po návratu (i při úspěchu) nezůstane po procesu nic běžet;
    // do jobu se dostane už při vzniku (pozastavený), takže mu neuteče žádný potomek
    let job = crate::procman::Job::new().ok();
    let mut child = match &job {
        Some(j) => j.spawn(cmd, 0x0800_0000).map_err(RunError::Spawn)?,
        None => cmd.creation_flags(0x0800_0000).spawn().map_err(|e| RunError::Spawn(e.to_string()))?,
    };
    let kill = |child: &mut std::process::Child| {
        if let Some(j) = &job {
            j.kill();
        }
        let _ = child.kill();
        let _ = child.wait();
    };
    if let (Some(text), Some(mut sin)) = (input, child.stdin.take()) {
        let text = text.to_string();
        // zápis ve vlákně: velký vstup by jinak mohl uváznout proti plnému stdout
        std::thread::spawn(move || {
            let _ = sin.write_all(text.as_bytes());
        });
    }
    let Some(mut out) = child.stdout.take() else {
        kill(&mut child);
        return Err(RunError::Spawn("bez stdout".into()));
    };
    // stderr ve vlastním vlákně: plná roura stderr by jinak zablokovala proces i čtení stdout
    let err_rx = child.stderr.take().map(|mut e| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(read_tail(&mut e, cap).0);
        });
        rx
    });
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read_tail(&mut out, cap).0);
    });
    // na rouru se nikdy nečeká déle než do termínu
    let left = || deadline.saturating_duration_since(Instant::now());
    let Ok(buf) = rx.recv_timeout(left()) else {
        kill(&mut child);
        return Err(RunError::Timeout);
    };
    let err_buf = match &err_rx {
        Some(r) => match r.recv_timeout(left()) {
            Ok(b) => b,
            Err(_) => {
                kill(&mut child);
                return Err(RunError::Timeout);
            }
        },
        None => Vec::new(),
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(2)),
            _ => {
                kill(&mut child);
                return Err(RunError::Timeout);
            }
        }
    };
    Ok(Captured {
        success: status.success(),
        stdout: String::from_utf8_lossy(&buf).into_owned(),
        stderr: String::from_utf8_lossy(&err_buf).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_respects_deadline_and_errors() {
        let tmp = std::env::temp_dir();
        // termín už prošel → git se ani nespustí
        assert_eq!(run(&tmp, &["--version"], Instant::now()), None);
        let v = run(&tmp, &["--version"], Instant::now() + Duration::from_secs(5)).unwrap();
        assert!(v.starts_with("git version"));
        // nenulový exit → None
        assert_eq!(run(&tmp, &["nejsem-prikaz"], Instant::now() + Duration::from_secs(5)), None);
        // stdin dojde gitu (hash-object --stdin vrátí sha vstupu)
        let sha = run_with_input(&tmp, &["hash-object", "--stdin"], Some("ahoj\n"), Instant::now() + Duration::from_secs(5)).unwrap();
        assert_eq!(sha.trim().len(), 40);
        // termín uprostřed běhu: alias spustí shell se `sleep`, který drží rouru → zabít strom, None
        let t0 = Instant::now();
        assert_eq!(run(&tmp, &["-c", "alias.zz=!sleep 5", "zz"], Instant::now() + Duration::from_millis(300)), None);
        assert!(t0.elapsed() < Duration::from_secs(2));
        // run_env: proměnná prostředí dojde gitu, chyba vrátí stderr i neúspěch
        let d = Instant::now() + Duration::from_secs(5);
        let o = run_env(&tmp, &["var", "GIT_AUTHOR_IDENT"], &[("GIT_AUTHOR_NAME", "Zkouska"), ("GIT_AUTHOR_EMAIL", "z@x")], d).unwrap();
        assert!(o.success && o.stdout.starts_with("Zkouska <z@x>"), "{o:?}");
        let o = run_env(&tmp, &["nejsem-prikaz"], &[], d).unwrap();
        assert!(!o.success && o.stderr.contains("nejsem-prikaz"), "{o:?}");
        assert_eq!(run_env(&tmp, &["--version"], &[], Instant::now()).unwrap_err(), RunError::Timeout);
    }

    #[test]
    fn capture_reports_stderr_exit_and_spawn_errors() {
        let tmp = std::env::temp_dir();
        // nenulový exit: výstup se vrátí i se stderr (podle něj se rozhoduje „proč“)
        let mut cmd = Command::new("git");
        cmd.args(["nejsem-prikaz"]).current_dir(&tmp);
        let out = capture(&mut cmd, None, true, Instant::now() + Duration::from_secs(5)).unwrap();
        assert!(!out.success);
        assert!(out.stderr.contains("nejsem-prikaz"), "{}", out.stderr);
        // neexistující program → Spawn, ne panika
        let mut cmd = Command::new("ledge-neexistujici-program-xyz");
        assert!(matches!(capture(&mut cmd, None, true, Instant::now() + Duration::from_secs(5)), Err(RunError::Spawn(_))));
        // prošlý termín → Timeout bez spuštění
        let mut cmd = Command::new("git");
        assert_eq!(capture(&mut cmd, None, true, Instant::now()).unwrap_err(), RunError::Timeout);
    }

    #[test]
    fn parses_branch_ab_and_dirty() {
        let g = parse_porcelain_v2(
            "# branch.oid abc\n# branch.head atlas-v2\n# branch.upstream origin/atlas-v2\n# branch.ab +2 -1\n1 .M N... 100644 100644 100644 a b src/x.ts\n? new.txt\n",
        );
        assert_eq!(g, GitInfo { branch: Some("atlas-v2".into()), dirty: 2, ahead: 2, behind: 1 });
    }
}
