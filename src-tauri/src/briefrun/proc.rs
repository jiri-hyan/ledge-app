//! I/O běhu zadání: `claude.exe` v Job Objectu s KILL_ON_JOB_CLOSE (`procman::Job`) bez okna, cwd =
//! projekt, klíč jen v env dítěte. Jedno čtecí vlákno na běh: pošle zprávu na stdin, čte stream-json,
//! po `result` zavře stdin a nechá proces doběhnout; pojistka zdroje klíče zabije strom hned. Žádné
//! časovače — vlákno spí v `read`. Konec běhu (i zastavení, pád) → `on_end(Ended)`; pak se zavře
//! job → případné zbylé vnuky (shelly session) umřou. Ukončení Ledge zavře všechny joby → žádní sirotci.

use super::stream::{self, Ended, Event};
use crate::procman::Job;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

const NO_WINDOW: u32 = 0x0800_0000;
/// Kolik konce stderr držet (do logu jde jen první řádek, začištěný).
const ERR_CAP: usize = 4096;

struct Run {
    job: Arc<Job>,
    stopped: Arc<AtomicBool>,
}

static RUNS: LazyLock<Mutex<HashMap<PathBuf, Run>>> = LazyLock::new(Default::default);

/// Spuštění (vše ověřené volajícím). `key` jen pro env a začištění výpisu chyb.
pub struct Start {
    /// kanonická cesta poznámky = klíč běhu
    pub real: PathBuf,
    pub exe: PathBuf,
    pub args: Vec<String>,
    pub env_set: Vec<(String, String)>,
    pub env_remove: Vec<String>,
    pub cwd: PathBuf,
    /// jeden řádek stream-json pro stdin
    pub stdin_line: String,
    pub key: String,
}

pub fn running(real: &Path) -> bool {
    RUNS.lock().contains_key(real)
}

/// Zabije strom běhu (výslovný klik „Zastavit“). Vrací, zda běh byl.
pub fn stop(real: &Path) -> bool {
    let runs = RUNS.lock();
    let Some(r) = runs.get(real) else { return false };
    r.stopped.store(true, Ordering::SeqCst);
    r.job.kill();
    true
}

/// Konec Ledge: zabít všechny běhy (zavření handle jobů by to udělalo taky).
pub fn stop_all() {
    for r in RUNS.lock().values() {
        r.stopped.store(true, Ordering::SeqCst);
        r.job.kill();
    }
}

/// Spustí běh; `on_end` zavolá čtecí vlákno po skončení procesu.
pub fn start(s: Start, on_end: impl FnOnce(Ended) + Send + 'static) -> Result<(), String> {
    let job = Arc::new(Job::new()?);
    let stopped = Arc::new(AtomicBool::new(false));
    {
        let mut runs = RUNS.lock();
        if runs.contains_key(&s.real) {
            return Err(crate::i18n::tr("Zadání už běží.", "The brief is already running.").into());
        }
        runs.insert(s.real.clone(), Run { job: job.clone(), stopped: stopped.clone() });
    }
    let mut cmd = Command::new(&s.exe);
    cmd.args(&s.args).current_dir(&s.cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in &s.env_set {
        cmd.env(k, v);
    }
    for k in &s.env_remove {
        cmd.env_remove(k);
    }
    let mut child = match job.spawn(&mut cmd, NO_WINDOW) {
        Ok(c) => c,
        Err(e) => {
            RUNS.lock().remove(&s.real);
            return Err(crate::errors::spawn_text(crate::i18n::tr("Claude Code nejde spustit", "Can't start Claude Code"), &e));
        }
    };
    let (stdin, stdout, stderr) = (child.stdin.take(), child.stdout.take(), child.stderr.take());
    let err_tail = Arc::new(Mutex::new(Vec::<u8>::new()));
    let err_thread = stderr.map(|mut e| {
        let tail = err_tail.clone();
        std::thread::Builder::new().name("brief-err".into()).spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(n) = e.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let mut t = tail.lock();
                if t.len() < ERR_CAP {
                    let room = ERR_CAP - t.len();
                    t.extend_from_slice(&chunk[..n.min(room)]);
                }
            }
        })
    });
    let spawned = std::thread::Builder::new().name("brief".into()).spawn(move || {
        let mut ended = Ended::default();
        let mut stdin = stdin;
        if let Some(w) = stdin.as_mut() {
            // proces mohl spadnout hned — chybu zápisu prozradí konec bez `result`
            let _ = w.write_all(s.stdin_line.as_bytes()).and_then(|()| w.flush());
        }
        if let Some(out) = stdout {
            read_events(BufReader::new(out), &mut ended, &mut stdin, &job);
        }
        drop(stdin);
        let _ = child.wait();
        if let Some(Ok(h)) = err_thread {
            let _ = h.join();
        }
        let err = String::from_utf8_lossy(&err_tail.lock()).into_owned();
        ended.stderr = err.lines().map(str::trim).find(|l| !l.is_empty()).map(|l| stream::redact(l, &s.key)).unwrap_or_default();
        ended.stopped = stopped.load(Ordering::SeqCst);
        // job se zavře až tady (drop posledních `Arc`) → zbylí potomci session umřou
        RUNS.lock().remove(&s.real);
        drop(job);
        on_end(ended);
    });
    spawned.map(|_| ()).map_err(|e| e.to_string())
}

/// Čte řádky stream-json až do konce výstupu. `result` → zavřít stdin; špatný zdroj klíče → zabít strom.
pub(crate) fn read_events(mut r: impl BufRead, ended: &mut Ended, stdin: &mut Option<impl Write>, job: &Job) {
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match r.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        match stream::parse_line(&String::from_utf8_lossy(&buf)) {
            Event::Init { api_key_source } if stream::wrong_source(api_key_source.as_deref()) => {
                ended.wrong_source = api_key_source;
                job.kill();
                *stdin = None;
            }
            Event::Result { cost, is_error, subtype } => {
                ended.result = Some((cost, is_error, subtype));
                // jeden tah na běh: stdin zavřít → CLI skončí, nic nečeká na další zprávu
                *stdin = None;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_close_stdin_and_guard_kills() {
        let job = Job::new().unwrap();
        let ok = "{\"type\":\"system\",\"subtype\":\"init\",\"apiKeySource\":\"ANTHROPIC_API_KEY\"}\n{\"type\":\"assistant\"}\n\
                  {\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.25}\n";
        let mut e = Ended::default();
        let mut stdin = Some(Vec::<u8>::new());
        read_events(ok.as_bytes(), &mut e, &mut stdin, &job);
        assert_eq!(e.result, Some((Some(0.25), false, "success".into())));
        assert!(stdin.is_none(), "po result je stdin zavřený");
        assert_eq!(e.wrong_source, None);

        let bad = "{\"type\":\"system\",\"subtype\":\"init\",\"apiKeySource\":\"none\"}\n";
        let mut e = Ended::default();
        let mut stdin = Some(Vec::<u8>::new());
        read_events(bad.as_bytes(), &mut e, &mut stdin, &job);
        assert_eq!(e.wrong_source.as_deref(), Some("none"));
        assert!(stdin.is_none());
        assert_eq!(stream::finish(&e, false).status, "draft");
    }

    #[test]
    fn real_process_without_window_reports_end() {
        // náhradní „claude“: cmd.exe vypíše připravený stream-json (žádný skutečný claude, žádné okno)
        let dir = std::env::temp_dir().join(format!("ledge-briefproc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("out.jsonl");
        std::fs::write(&out, "{\"type\":\"system\",\"subtype\":\"init\",\"apiKeySource\":\"ANTHROPIC_API_KEY\"}\r\n{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.5}\r\n").unwrap();
        let cmd = PathBuf::from(std::env::var("ComSpec").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into()));
        let real = dir.join("z.md");
        let (tx, rx) = std::sync::mpsc::channel();
        let s = Start {
            real: real.clone(),
            exe: cmd,
            args: vec!["/d".into(), "/c".into(), "type".into(), out.to_string_lossy().into_owned()],
            env_set: vec![("ANTHROPIC_API_KEY".into(), "sk-ant-test".into())],
            env_remove: vec!["ANTHROPIC_AUTH_TOKEN".into()],
            cwd: dir.clone(),
            stdin_line: "{}\n".into(),
            key: "sk-ant-test".into(),
        };
        start(s, move |e| tx.send(e).unwrap()).unwrap();
        let e = rx.recv_timeout(std::time::Duration::from_secs(20)).unwrap();
        assert_eq!(e.result, Some((Some(0.5), false, "success".into())));
        assert!(!e.stopped);
        assert!(!running(&real), "po konci běh není v registru");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
