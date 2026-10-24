//! Vlákno `notes-sync`: git synchronizace poznámek (`notesync.rs`) — blokuje na kanálu `SyncMsg`
//! do dalšího termínu, žádné dotazování. Stav pro UI v `Core.notes_sync`.

use super::notes_watch::{NotesMsg, notes_root, request_notes_reload};
use crate::state::{core, log, mark_dirty, now_ms};
use crate::{notes, notesync};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ---------- synchronizace poznámek ----------

pub(crate) enum SyncMsg {
    /// lokální změna poznámek (debounce v `notesync::Sched`)
    Local,
    /// otevřený šuplík (throttle)
    Opened,
    /// změna `notes` v configu
    Config,
    /// „Synchronizovat teď“ — odpověď jde zpět kanálem
    Now(mpsc::Sender<Result<String, String>>),
}

pub(crate) fn send_sync(m: SyncMsg) {
    // propojení se složkou (`notes_folder`) se řídí stejnými příčinami: otevřený šuplík, změna configu
    match m {
        SyncMsg::Opened => super::notes_folder::opened(),
        SyncMsg::Config => super::notes_folder::config_changed(),
        _ => {}
    }
    let _ = core().sync_tx.lock().send(m);
}

/// Watcher: změna v kořeni poznámek, kterou neudělal sync sám (ten si během běhu a 2 s po něm
/// nastaví `sync_quiet_ms`).
pub(crate) fn notify_sync_local() {
    if now_ms() >= core().sync_quiet_ms.load(Ordering::Relaxed) {
        send_sync(SyncMsg::Local);
    }
}

/// (kořen, remote, interval), když má sync běžet.
pub(crate) fn sync_settings() -> Option<(PathBuf, String, Duration)> {
    let (on, remote, interval) = {
        let cfg = core().cfg.read();
        let s = &cfg.notes.sync;
        (cfg.notes.enabled && s.enabled, s.remote.trim().to_string(), Duration::from_secs(u64::from(s.interval_min.clamp(1, 120)) * 60))
    };
    if !on {
        return None;
    }
    Some((notes_root()?, remote, interval))
}

/// Vlákno `notes-sync`: v klidu blokuje na kanálu (`recv_timeout` do dalšího termínu, při vypnutém
/// syncu `recv` bez termínu) — žádné dotazování.
pub(crate) fn spawn_notes_sync(rx: mpsc::Receiver<SyncMsg>) {
    std::thread::Builder::new()
        .name("notes-sync".into())
        .spawn(move || {
            let pc = notesync::sanitize_pc(&std::env::var("COMPUTERNAME").unwrap_or_default());
            let mut syncer = notesync::Syncer::default();
            let mut sched = notesync::Sched::default();
            loop {
                let Some((_, _, interval)) = sync_settings() else {
                    let was = std::mem::take(&mut *core().notes_sync.lock());
                    if was != notesync::Status::default() {
                        mark_dirty();
                    }
                    match rx.recv() {
                        Ok(SyncMsg::Now(reply)) => {
                            let _ = reply.send(Err(crate::i18n::tr("Synchronizace poznámek je vypnutá v Nastavení.", "Notes sync is off in Settings.").into()));
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                    continue;
                };
                let now = Instant::now();
                let wait = sched.deadline(now, interval).saturating_duration_since(now);
                let mut reply = None;
                let run = match rx.recv_timeout(wait) {
                    Ok(m) => {
                        let w = match m {
                            SyncMsg::Local => notesync::Wake::Local,
                            SyncMsg::Opened => notesync::Wake::Opened,
                            SyncMsg::Config => notesync::Wake::Config,
                            SyncMsg::Now(r) => {
                                reply = Some(r);
                                notesync::Wake::Now
                            }
                        };
                        sched.on(w, Instant::now(), interval)
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => sched.on(notesync::Wake::Timer, Instant::now(), interval),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                if !run {
                    continue;
                }
                // config se mohl během čekání změnit (vypnout) → číst znovu
                let Some((root, remote, _)) = sync_settings() else {
                    if let Some(r) = reply {
                        let _ = r.send(Err(crate::i18n::tr("Synchronizace poznámek je vypnutá v Nastavení.", "Notes sync is off in Settings.").into()));
                    }
                    continue;
                };
                let outcome = run_notes_sync(&mut syncer, &root, &remote, &pc);
                sched.ran(Instant::now());
                if let Some(r) = reply {
                    let _ = r.send(notesync::outcome_text(&outcome));
                }
            }
        })
        .ok();
}

pub(crate) fn run_notes_sync(syncer: &mut notesync::Syncer, root: &Path, remote: &str, pc: &str) -> notesync::Outcome {
    let c = core();
    let root_s = root.to_string_lossy().into_owned();
    {
        let mut st = c.notes_sync.lock();
        st.state = notesync::State::Syncing;
        st.root = Some(root_s.clone());
    }
    mark_dirty();
    c.sync_quiet_ms.store(i64::MAX, Ordering::Relaxed);
    let outcome = syncer.sync(&notesync::Ctx { root, remote, pc, now: notes::now_local() });
    c.sync_quiet_ms.store(now_ms() + 2000, Ordering::Relaxed);
    for line in &syncer.notes {
        log(line);
    }
    let (prev, now_state) = {
        let mut st = c.notes_sync.lock();
        let prev = st.state;
        st.apply(&outcome, now_ms());
        st.root = Some(root_s);
        (prev, st.state)
    };
    match &outcome {
        notesync::Outcome::Ok { pulled, pushed, swapped } => {
            if *pulled || *pushed {
                log(&format!("sync poznámek: ok (staženo {pulled}, odesláno {pushed})"));
            }
            if swapped.is_some() {
                // kořen je nová složka → sledovat znovu (watcher pak přenačte)
                let _ = c.notes_tx.lock().send(NotesMsg::Reroot);
            } else if *pulled {
                request_notes_reload();
            }
        }
        // chyby do logu jen při změně stavu (offline každých 5 min by log zahltil)
        other if prev != now_state || matches!(other, notesync::Outcome::Konflikt { .. }) => {
            log(&format!("sync poznámek: {other:?}"));
        }
        _ => {}
    }
    mark_dirty();
    outcome
}
