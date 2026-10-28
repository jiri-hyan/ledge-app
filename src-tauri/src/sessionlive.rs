//! Živý seznam Claude session (I/O k `sessions.rs`): vlákno `sessions` spouští
//! `claude agents --json --all` (bez okna, termín 10 s, nejvýš 1× za 5 s) JEN při otevřeném šuplíku,
//! a čte registr `~/.claude/sessions/<pid>.json` (watcher `notify`, debounce 2 s, cache podle mtime).
//! Spouště: otevření šuplíku (`request(Open)`), změna registru, hook neznámé session. Žádné polling.

use crate::sessions::{self, Agent, Reg};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime};

const AGENTS_TIMEOUT: Duration = Duration::from_secs(10);
const DEBOUNCE: Duration = Duration::from_secs(2);
const MAX_DEBOUNCE: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    /// šuplík se otevřel → čerstvý seznam
    Open,
    /// změna ve složce registru (po debounce)
    Registry,
    /// hook od session, kterou seznam nezná
    Hook,
}

#[derive(Default)]
pub struct Cache {
    /// `None` = seznam zatím nemáme (CLI chybí / selhalo) — UI pak ukáže jen hook session
    pub agents: Option<Vec<Agent>>,
    pub fetched_at: u64,
    pub registry: Vec<Reg>,
    /// soubor registru → (mtime, délka, obsah) — čte se jen změněný
    files: HashMap<PathBuf, (SystemTime, u64, Option<Reg>)>,
}

struct Live {
    cache: Mutex<Cache>,
    tx: Mutex<mpsc::Sender<Wake>>,
}

static LIVE: OnceLock<Live> = OnceLock::new();

pub type Hook = Arc<dyn Fn() + Send + Sync>;
pub type IsOpen = Arc<dyn Fn() -> bool + Send + Sync>;

use crate::util::now_ms;

pub use crate::dirs::claude_dir;

/// Spustí vlákno a watcher registru. `on_change` = překreslit (mark_dirty), `is_open` = šuplík otevřený.
pub fn start(on_change: Hook, is_open: IsOpen) {
    let (tx, rx) = mpsc::channel();
    if LIVE.set(Live { cache: Mutex::new(Cache::default()), tx: Mutex::new(tx.clone()) }).is_err() {
        return;
    }
    let dir = claude_dir().map(|d| d.join("sessions"));
    std::thread::Builder::new()
        .name("sessions".into())
        .spawn(move || worker(rx, dir, on_change, is_open))
        .ok();
    if let Some(d) = claude_dir().map(|d| d.join("sessions")) {
        watch(d, tx);
    }
}

pub fn request(w: Wake) {
    if let Some(l) = LIVE.get() {
        let _ = l.tx.lock().send(w);
    }
}

/// Čtení cache (snapshot). Před `start` prázdná.
pub fn with_cache<R>(f: impl FnOnce(&Cache) -> R) -> R {
    match LIVE.get() {
        Some(l) => f(&l.cache.lock()),
        None => f(&Cache::default()),
    }
}

/// Hook session, kterou poslední seznam procesů nezná a ozvala se až po něm → stojí za přenačtení.
/// (Starší neznámé — mrtvé procesy — spawn neopakují.)
pub fn unseen(session_id: &str, updated_at: u64) -> bool {
    with_cache(|c| updated_at > c.fetched_at && c.agents.as_ref().is_some_and(|a| !a.iter().any(|x| x.session_id == session_id)))
}

fn worker(rx: mpsc::Receiver<Wake>, dir: Option<PathBuf>, on_change: Hook, is_open: IsOpen) {
    let mut last_spawn: Option<u64> = None;
    while let Ok(first) = rx.recv() {
        let mut wakes = vec![first];
        wakes.extend(rx.try_iter());
        let mut changed = dir.as_deref().is_some_and(reload_registry);
        // Open přichází dřív, než šuplík přepne stav na Full → platí bez `is_open`
        let want = wakes.contains(&Wake::Open) || (is_open() && (wakes.contains(&Wake::Hook) || membership_changed()));
        if want {
            let wait = sessions::spawn_wait(last_spawn, now_ms());
            if wait > 0 {
                std::thread::sleep(Duration::from_millis(wait));
                rx.try_iter().for_each(drop); // co přišlo mezitím, pokryje tenhle běh
            }
            last_spawn = Some(now_ms());
            let agents = fetch_agents().or_else(registry_fallback);
            if let Some(l) = LIVE.get() {
                let mut c = l.cache.lock();
                c.agents = agents;
                c.fetched_at = now_ms();
            }
            changed = true;
        }
        if changed {
            on_change();
        }
    }
}

/// Liší se množina session v registru od posledního seznamu procesů?
fn membership_changed() -> bool {
    with_cache(|c| {
        let Some(a) = &c.agents else { return true };
        let mut reg: Vec<&str> = c.registry.iter().map(|r| r.session_id.as_str()).collect();
        let mut ag: Vec<&str> = a
            .iter()
            .filter(|x| x.kind.as_deref() != Some("background"))
            .filter(|x| sessions::live_status(x.status.as_deref(), x.state.as_deref(), x.waiting_for.as_deref()) != sessions::Status::Ended)
            .map(|x| x.session_id.as_str())
            .collect();
        reg.sort_unstable();
        ag.sort_unstable();
        reg != ag
    })
}

/// `claude agents --json --all` přes `cmd` (na PATH je `claude.cmd`/`.ps1`), bez okna, s termínem.
fn fetch_agents() -> Option<Vec<Agent>> {
    let mut cmd = std::process::Command::new("cmd");
    cmd.args(["/d", "/c", "claude", "agents", "--json", "--all"]);
    match crate::gitinfo::capture(&mut cmd, None, true, Instant::now() + AGENTS_TIMEOUT) {
        Ok(out) if out.success => sessions::parse_agents(&out.stdout),
        Ok(out) => {
            crate::log(&format!("claude agents: {}", out.stderr.lines().next().unwrap_or("selhalo")));
            None
        }
        Err(e) => {
            crate::log(&format!("claude agents: {e:?}"));
            None
        }
    }
}

/// Bez CLI: běžící procesy z registru (formát nedokumentovaný, proto jen záloha).
fn registry_fallback() -> Option<Vec<Agent>> {
    let reg = with_cache(|c| c.registry.clone());
    if reg.is_empty() {
        return None;
    }
    Some(
        reg.into_iter()
            .filter(|r| crate::winfocus::pid_alive(r.pid))
            .map(|r| Agent { pid: r.pid, session_id: r.session_id, cwd: r.cwd, kind: r.kind, started_at: r.started_at, name: r.name, status: r.status, ..Default::default() })
            .collect(),
    )
}

/// Přečte změněné soubory registru. Vrací `true`, když se obsah změnil.
fn reload_registry(dir: &Path) -> bool {
    let Some(l) = LIVE.get() else { return false };
    let Ok(rd) = std::fs::read_dir(dir) else {
        let mut c = l.cache.lock();
        let had = !c.files.is_empty();
        c.files.clear();
        c.registry.clear();
        return had;
    };
    let mut seen: Vec<(PathBuf, SystemTime, u64)> = Vec::new();
    for e in rd.flatten() {
        if !sessions::is_registry_file(&e.file_name().to_string_lossy()) {
            continue;
        }
        if let Ok(m) = e.metadata() {
            seen.push((e.path(), m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len()));
        }
    }
    let mut c = l.cache.lock();
    let before = c.files.len();
    c.files.retain(|p, _| seen.iter().any(|(s, _, _)| s == p));
    let mut changed = c.files.len() != before;
    for (p, mtime, len) in seen {
        if c.files.get(&p).is_some_and(|(m, l, _)| *m == mtime && *l == len) {
            continue;
        }
        let reg = std::fs::read_to_string(&p).ok().and_then(|t| sessions::parse_registry(&t));
        c.files.insert(p, (mtime, len, reg));
        changed = true;
    }
    if changed {
        c.registry = c.files.values().filter_map(|(_, _, r)| r.clone()).collect();
    }
    changed
}

/// Watcher složky registru (nerekurzivní) → po 2 s klidu `Wake::Registry`.
fn watch(dir: PathBuf, tx: mpsc::Sender<Wake>) {
    use notify::{RecursiveMode, Watcher};
    std::thread::Builder::new()
        .name("sessions-watch".into())
        .spawn(move || {
            let (ftx, frx) = mpsc::channel::<()>();
            let w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let relevant = res.is_ok_and(|e| e.paths.iter().any(|p| p.file_name().is_some_and(|n| sessions::is_registry_file(&n.to_string_lossy()))));
                if relevant {
                    let _ = ftx.send(());
                }
            });
            let Ok(mut w) = w else { return };
            if let Err(e) = w.watch(&dir, RecursiveMode::NonRecursive) {
                crate::log(&format!("sessions: registr {} nejde sledovat: {e}", dir.display()));
                return;
            }
            let _ = tx.send(Wake::Registry); // první načtení
            while frx.recv().is_ok() {
                // klid 2 s, ale při trvalém ruchu (session pracují) nejpozději po 10 s
                let t0 = Instant::now();
                while t0.elapsed() < MAX_DEBOUNCE && frx.recv_timeout(DEBOUNCE).is_ok() {}
                if tx.send(Wake::Registry).is_err() {
                    break;
                }
            }
            drop(w);
        })
        .ok();
}
