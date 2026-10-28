//! I/O pro Agent View (`agentview.rs` = čistá logika): spuštění `claude <příkaz>` bez okna
//! (`CREATE_NO_WINDOW`) s termínem a stropem výstupu, a `claude attach <id>` v novém okně terminálu.
//! Volá se JEN z výslovného kliku v kartě (`commands/agents.rs`) — nic periodického.
//!
//! Proč ne `gitinfo::capture`: ten běh zavírá v Job Objectu s KILL_ON_JOB_CLOSE, takže by po návratu
//! zabil i proces, který `claude --bg` / `respawn` nechává běžet na pozadí. Tady se při termínu zabije
//! jen přímý potomek a čtení výstupu nečeká na konec roury (drží-li ji zděděná session, vlákno čtení
//! skončí až s ní; nic nepočítá).

use crate::agentview::{self, Program};
use crate::i18n::tr;
use crate::sessions::Launch;
use parking_lot::Mutex;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

const NO_WINDOW: u32 = 0x0800_0000;
const DETACHED: u32 = 0x0000_0008;
/// Jak dlouho po skončení procesu ještě dočíst rouru.
const DRAIN: Duration = Duration::from_millis(500);

/// `claude` z PATH bez shellu.
pub fn program() -> Result<Program, String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let dirs: Vec<PathBuf> = std::env::split_paths(&path).collect();
    agentview::resolve_claude(&dirs, &|p| p.is_file()).ok_or_else(|| tr("Claude Code (claude) není na PATH.", "Claude Code (claude) isn't on PATH.").to_string())
}

pub struct Out {
    pub ok: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// začátek výstupu se nevešel do stropu
    pub cut: bool,
}

impl Out {
    /// První neprázdný řádek chybového (jinak běžného) výstupu — do hlášky.
    pub fn first_line(&self) -> String {
        [&self.stderr, &self.stdout]
            .iter()
            .map(|b| agentview::clean_output(b))
            .find_map(|t| t.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string))
            .unwrap_or_default()
    }
}

#[derive(Default)]
struct Tail {
    buf: Vec<u8>,
    cut: bool,
}

fn reader(mut r: impl Read + Send + 'static, cap: usize, tail: Arc<Mutex<Tail>>, done: mpsc::Sender<()>) {
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match r.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let mut t = tail.lock();
                    t.buf.extend_from_slice(&chunk[..n]);
                    if t.buf.len() > cap * 2 {
                        let drop = t.buf.len() - cap;
                        t.buf.drain(..drop);
                        t.cut = true;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = done.send(());
    });
}

fn take(t: &Mutex<Tail>, cap: usize) -> (Vec<u8>, bool) {
    let mut t = t.lock();
    let mut buf = std::mem::take(&mut t.buf);
    let mut cut = t.cut;
    if buf.len() > cap {
        buf.drain(..buf.len() - cap);
        cut = true;
    }
    (buf, cut)
}

/// `claude <args>` bez okna. Po termínu zabije přímého potomka a vrátí chybu.
pub fn run(args: &[String], cwd: Option<&Path>, wait: Duration, cap: usize) -> Result<Out, String> {
    let (exe, args) = match program()? {
        Program::Exe(p) => (p, args.to_vec()),
        Program::Batch(p) => (p, agentview::for_batch(args)),
    };
    let mut cmd = Command::new(&exe);
    cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).creation_flags(NO_WINDOW);
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let mut child = cmd.spawn().map_err(|e| crate::errors::spawn(tr("Claude Code nejde spustit", "Can't start Claude Code"), &e))?;
    let (out, err) = (Arc::new(Mutex::new(Tail::default())), Arc::new(Mutex::new(Tail::default())));
    let (tx, rx) = mpsc::channel();
    if let Some(o) = child.stdout.take() {
        reader(o, cap, out.clone(), tx.clone());
    }
    if let Some(e) = child.stderr.take() {
        reader(e, cap, err.clone(), tx);
    }
    let deadline = Instant::now() + wait;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let s = wait.as_secs();
                return Err(trf!("Claude Code neodpověděl do {s} s.", "Claude Code didn't answer within {s} s."));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(crate::errors::io(tr("Claude Code nejde dočkat", "Can't wait for Claude Code"), &e)),
        }
    };
    let end = Instant::now() + DRAIN;
    for _ in 0..2 {
        if rx.recv_timeout(end.saturating_duration_since(Instant::now())).is_err() {
            break;
        }
    }
    let (stdout, cut) = take(&out, cap);
    let (stderr, _) = take(&err, cap);
    Ok(Out { ok: status.success(), stdout, stderr, cut })
}

/// `claude attach <id>` v novém okně terminálu (výslovný klik). Neexistující složka session
/// (smazaný worktree) → domovská složka; `attach` podle id funguje odkudkoli.
pub fn attach(cwd: &str, id: &str) -> Result<(), String> {
    let dir = if Path::new(cwd).is_dir() { cwd.to_string() } else { crate::dirs::home().map(|h| h.to_string_lossy().into_owned()).unwrap_or_default() };
    let bad = || tr("Session nejde bezpečně připojit (neobvyklé id nebo složka).", "The session can't be attached safely (unusual id or folder).").to_string();
    if let Some(Launch::Wt(args)) = agentview::attach_command(true, &dir, id)
        && Command::new("wt").args(&args).creation_flags(DETACHED).spawn().is_ok()
    {
        return Ok(());
    }
    match agentview::attach_command(false, &dir, id).ok_or_else(bad)? {
        Launch::Cmd(raw) => Command::new("cmd").raw_arg(raw).creation_flags(NO_WINDOW).spawn().map(|_| ()).map_err(|e| crate::errors::spawn(tr("Terminál nejde otevřít", "Can't open a terminal"), &e)),
        Launch::Wt(_) => Err(bad()),
    }
}
