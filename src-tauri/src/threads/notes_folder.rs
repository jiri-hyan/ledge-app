//! Vlákno `notes-folder`: propojení poznámek se složkou (`notes::folder_sync`, config `notes.folderLink`).
//! Žádný polling: vlákno spí na kanálu; budí ho (1) změna poznámek v Ledge (`reload_notes`), (2) událost
//! systému souborů ve vybrané složce (`notify` = ReadDirectoryChangesW, jen s `.md` mimo skryté části),
//! (3) otevření šuplíku (nejvýš 1× za minutu), (4) změna configu, (5) „Synchronizovat teď“. Události se
//! slévají (debounce 3 s, nejdéle 30 s), jeden běh nezměněného vaultu jen projde `stat` souborů.
//! Vypnuté propojení = vlákno blokuje bez termínu, nic nesleduje. Stav pro UI je `Snapshot.notesFolder`.

use super::notes_watch::notes_root;
use crate::notes::{self, folder_sync};
use crate::notes::folder_sync_apply::ConflictInfo;
use crate::state::{core, log, mark_dirty, now_ms};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, mpsc};
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_secs(3);
const MAX_WAIT: Duration = Duration::from_secs(30);
const START_DELAY: Duration = Duration::from_secs(5);
const OPEN_GAP: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum State {
    /// propojení vypnuté
    #[default]
    Off,
    Idle,
    Syncing,
    Error,
}

/// `Snapshot.notesFolder`.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub state: State,
    /// poslední úspěšný běh (ms) — jen z tohoto běhu Ledge
    pub last_ms: Option<i64>,
    /// věta o posledním běhu, nebo chyba
    pub message: Option<String>,
    /// konflikty k zobrazení (nejnovější první), mažou se „Rozumím“
    pub conflicts: Vec<ConflictInfo>,
}

static STATUS: Mutex<Status> = Mutex::new(Status { state: State::Off, last_ms: None, message: None, conflicts: Vec::new() });
static ACTIVE: AtomicBool = AtomicBool::new(false);
static TX: OnceLock<Mutex<mpsc::Sender<Msg>>> = OnceLock::new();

pub(crate) fn status() -> Status {
    STATUS.lock().clone()
}

pub(crate) enum Msg {
    /// změna poznámek v Ledge nebo ve složce
    Poke,
    Opened,
    Config,
    /// „Synchronizovat teď“ — odpověď jde zpět kanálem
    Now(mpsc::Sender<Result<String, String>>),
}

pub(crate) fn send(m: Msg) {
    if let Some(tx) = TX.get() {
        let _ = tx.lock().send(m);
    }
}

/// Změnily se poznámky (volá `reload_notes`) — jen když propojení běží.
pub(crate) fn poke() {
    if ACTIVE.load(Ordering::Relaxed) {
        send(Msg::Poke);
    }
}

pub(crate) fn opened() {
    if ACTIVE.load(Ordering::Relaxed) {
        send(Msg::Opened);
    }
}

pub(crate) fn config_changed() {
    send(Msg::Config);
}

fn off_text() -> String {
    crate::i18n::tr("Propojení se složkou je vypnuté v Nastavení.", "Linking with a folder is off in Settings.").into()
}

/// Nastavení, když má propojení běžet (poznámky zapnuté, propojení zapnuté, složka vybraná).
fn link() -> Option<folder_sync::LinkRun> {
    let (on, path, dir, deletes) = {
        let cfg = core().cfg.read();
        let l = &cfg.notes.folder_link;
        (cfg.notes.enabled && l.enabled, l.path.trim().to_string(), l.direction, l.propagate_deletes)
    };
    if !on || path.is_empty() {
        return None;
    }
    Some(folder_sync::LinkRun { root: notes_root()?, folder: path.into(), dir, deletes })
}

fn set_status(f: impl FnOnce(&mut Status)) {
    let before = STATUS.lock().clone();
    let after = {
        let mut s = STATUS.lock();
        f(&mut s);
        s.clone()
    };
    if before != after {
        mark_dirty();
    }
}

/// Jeden běh. `Err` = nic se nesmazalo ani nepřepsalo (třeba odpojený disk).
pub(crate) fn run_once(link: &folder_sync::LinkRun) -> Result<String, String> {
    set_status(|s| s.state = State::Syncing);
    match folder_sync::run(link, &folder_sync::state_path(), notes::now_local()) {
        Ok(o) => {
            let changed = o.report != Default::default();
            if changed {
                log(&format!("propojení se složkou: {}", o.message));
            }
            set_status(|s| (s.state, s.last_ms, s.message, s.conflicts) = (State::Idle, Some(now_ms()), Some(o.message.clone()), o.conflicts.clone()));
            Ok(o.message)
        }
        Err(e) => {
            log(&format!("propojení se složkou: {e}"));
            set_status(|s| (s.state, s.message) = (State::Error, Some(e.clone())));
            Err(e)
        }
    }
}

/// „Rozumím“ — smaže zapamatované konflikty (soubory se nemění).
pub(crate) fn clear_conflicts() -> Result<(), String> {
    folder_sync::clear_conflicts(&folder_sync::state_path())?;
    set_status(|s| s.conflicts.clear());
    Ok(())
}

fn watch(link: &folder_sync::LinkRun) -> Option<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher};
    if !link.folder.is_dir() {
        return None;
    }
    let base = link.folder.clone();
    let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
        Ok(ev) if folder_sync::event_relevant(&base, &ev.paths) => poke(),
        Ok(_) => {}
        Err(_) => poke(),
    })
    .ok()?;
    w.watch(&link.folder, RecursiveMode::Recursive).ok()?;
    Some(w)
}

pub(crate) fn spawn_notes_folder() {
    let (tx, rx) = mpsc::channel();
    let _ = TX.set(Mutex::new(tx));
    std::thread::Builder::new().name("notes-folder".into()).spawn(move || run_loop(rx)).ok();
}

fn run_loop(rx: mpsc::Receiver<Msg>) {
    let mut watcher: Option<(std::path::PathBuf, notify::RecommendedWatcher)> = None;
    let mut last_run: Option<Instant> = None;
    // termín příštího běhu + kdy se čeká nejdéle (debounce s horním mezníkem)
    let mut due: Option<Instant> = Some(Instant::now() + START_DELAY);
    let mut first: Option<Instant> = None;
    loop {
        let Some(l) = link() else {
            watcher = None;
            ACTIVE.store(false, Ordering::Relaxed);
            set_status(|s| *s = Status::default());
            (due, first) = (None, None);
            match rx.recv() {
                Ok(Msg::Now(r)) => {
                    let _ = r.send(Err(off_text()));
                }
                Ok(Msg::Config) => due = Some(Instant::now() + Duration::from_secs(1)),
                Ok(_) => {}
                Err(_) => break,
            }
            continue;
        };
        ACTIVE.store(true, Ordering::Relaxed);
        if watcher.as_ref().map(|(p, _)| p) != Some(&l.folder) {
            watcher = watch(&l).map(|w| (l.folder.clone(), w));
        }
        let mut reply = None;
        let got = match due {
            Some(t) => rx.recv_timeout(t.saturating_duration_since(Instant::now())),
            None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        let now = Instant::now();
        match got {
            Ok(Msg::Now(r)) => {
                reply = Some(r);
                due = Some(now);
            }
            Ok(Msg::Poke) => {
                let f = *first.get_or_insert(now);
                due = Some((now + DEBOUNCE).min(f + MAX_WAIT));
            }
            Ok(Msg::Config) => due = Some(now + Duration::from_secs(1)),
            Ok(Msg::Opened) => {
                if last_run.is_none_or(|t| now.duration_since(t) >= OPEN_GAP) {
                    due.get_or_insert(now + Duration::from_secs(1));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if due.is_some_and(|t| t <= Instant::now()) {
            (due, first) = (None, None);
            let res = run_once(&l);
            last_run = Some(Instant::now());
            if res.is_err() {
                // odpojený disk / smazaná složka: sledování vznikne znovu při dalším běhu
                watcher = None;
            }
            if let Some(r) = reply {
                let _ = r.send(res);
            }
        } else if let Some(r) = reply {
            let _ = r.send(Err(off_text()));
        }
    }
}
