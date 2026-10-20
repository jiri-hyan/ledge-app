//! Vlákno `notes-watch`: sleduje kořen poznámek (události `notify`, žádný poll), přenačítá
//! `NotesState` a hlídá termíny připomínek (`check_reminders`, volá emit vlákno). Formát v `notes.rs`.

use super::notes_sync::notify_sync_local;
use crate::config::Config;
use crate::drawer::{Opener, View};
use crate::notes;
use crate::state::{UiCmd, core, log, mark_dirty, now_ms};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ---------- poznámky a připomínky ----------

#[derive(Default)]
pub(crate) struct NotesState {
    /// metadata všech poznámek, nejnovější první — přepočet jen při změně souborů / configu
    pub(crate) all: Vec<notes::NoteView>,
    /// nehotové připomínky po termínu
    pub(crate) due: Vec<notes::NoteView>,
    /// nejbližší budoucí termín (ms) — emit vlákno jen porovná čas
    pub(crate) next_due: Option<i64>,
    /// připomínky, pro které už vyjel peek (cesta + termín)
    pub(crate) peeked: HashSet<String>,
    /// nehotové podle kategorie (záložky karty) — počítá se jen při přenačtení
    pub(crate) kind_counts: std::collections::BTreeMap<String, usize>,
    /// archivované poznámky (`<složka>\_archiv`) — jen pro Archiv na stránce Poznámky, nikdy v kartě ani připomínkách
    pub(crate) archived: Vec<notes::NoteView>,
    /// číslo přenačtení (UI podle něj obnoví záložku)
    pub(crate) rev: u64,
}

pub(crate) enum NotesMsg {
    /// změna `.md` v úložišti
    Fs,
    /// změnil se `notes` v configu → sledovat jiný kořen
    Reroot,
    /// chyba watcheru (složka smazaná / přesunutá) → přenačíst a sledovat znovu
    Lost,
}

/// Kořen poznámek; neplatný `notes.root` → výchozí složka (chybu ukazuje `notes_root_error`).
pub(crate) fn notes_root() -> Option<PathBuf> {
    let cfg = core().cfg.read();
    cfg.notes.enabled.then(|| notes::resolve_root(&cfg.notes.root).unwrap_or_else(|_| notes::default_root()))
}

/// Chyba `notes.root` pro banner v šuplíku (config se jinak aplikuje, poznámky jedou z výchozí složky).
pub(crate) fn notes_root_error(cfg: &Config) -> Option<String> {
    if !cfg.notes.enabled {
        return None;
    }
    notes::resolve_root(&cfg.notes.root).err().map(|e| trf!("{e} Poznámky zatím ukládám do výchozí složky.", "{e} For now notes are saved to the default folder."))
}

/// Požádá vlákno `notes-watch` o přenačtení (asynchronně, s debounce).
pub(crate) fn request_notes_reload() {
    let _ = core().notes_tx.lock().send(NotesMsg::Fs);
}

/// Znovu načte poznámky z disku a přepočítá připomínky. Nic po termínu → peek připomínky zmizí.
pub(crate) fn reload_notes() {
    let root = notes_root();
    let all = root.as_deref().map(notes::scan).unwrap_or_default();
    let archived = root.as_deref().map(notes::lifecycle::scan_archive).unwrap_or_default();
    let (due, next) = notes::due_split(&all, now_ms());
    let c = core();
    let none_due = due.is_empty();
    {
        let mut st = c.notes.lock();
        st.kind_counts = notes::edit::open_kind_counts(&all);
        st.rev += 1;
        st.all = all;
        st.archived = archived;
        st.due = due;
        st.next_due = next;
    }
    if none_due && !c.claude.attention() && c.drawer.view() == View::Peek {
        c.send_ui(UiCmd::Hide);
    }
    // lístky na plochu si podle otisku přečtou svou poznámku znovu (jen když nějaký je)
    crate::pins::win::on_notes_changed();
    mark_dirty();
}

/// Volá emit vlákno (probudí se nejpozději v `next_due`, po konci fullscreenu ho budí `hover`):
/// nastal nejbližší termín? (jen porovnání času v paměti). Nová připomínka po termínu → peek bez
/// fokusu, stejně jako čekající schválení. Při fullscreen nic — peek počká.
/// Které už peekem vyjely, se pamatuje v `<root>\.state.json` → po restartu jen jantarový proužek.
pub(crate) fn check_reminders() {
    let c = core();
    let now = now_ms();
    let mut changed = false;
    let mut persist = None;
    {
        let mut st = c.notes.lock();
        if st.next_due.is_some_and(|t| now >= t) {
            let (due, next) = notes::due_split(&st.all, now);
            st.due = due;
            st.next_due = next;
            changed = true;
        }
        if !c.dead() {
            let new: Vec<String> = st.due.iter().map(notes::reminder_key).filter(|k| !st.peeked.contains(k)).collect();
            if !new.is_empty() {
                st.peeked.extend(new);
                persist = Some((st.peeked.clone(), st.due.clone()));
            }
        }
    }
    let fresh = persist.is_some();
    if let (Some((peeked, due)), Some(root)) = (persist, notes_root())
        && let Err(e) = notes::save_peeked(&root, &peeked, &due)
    {
        log(&format!("poznámky: stav připomínek nejde uložit: {e}"));
    }
    if fresh && c.drawer.view() == View::Hidden {
        c.send_ui(UiCmd::Show(View::Peek, Opener::Hover));
    }
    if changed || fresh {
        mark_dirty();
    }
}

/// Událost watcheru, na kterou stojí za to přenačíst: cokoli kromě konceptu, stavu, obrázků, tmp
/// a vnitřku `.git` (i smazání / přejmenování složky — ta nemá příponu `.md`).
pub(crate) fn notes_event_relevant(paths: &[PathBuf]) -> bool {
    paths.is_empty()
        || paths.iter().any(|p| {
            let s = p.to_string_lossy().to_lowercase().replace('/', "\\");
            !(s.contains("\\.draft") || s.contains("\\.smazane") || in_git_dir(&s) || s.ends_with(".state.json") || s.ends_with(".tmp") || s.ends_with(".png"))
        })
}

pub(crate) fn in_git_dir(s: &str) -> bool {
    s.contains("\\.git\\") || s.ends_with("\\.git")
}

/// Událost, kterou má vidět synchronizace (jako `notes_event_relevant`, ale i obrázky).
pub(crate) fn notes_event_syncable(paths: &[PathBuf]) -> bool {
    paths.is_empty()
        || paths.iter().any(|p| {
            let s = p.to_string_lossy().to_lowercase().replace('/', "\\");
            !(s.contains("\\.draft") || s.contains("\\.smazane") || in_git_dir(&s) || s.ends_with(".state.json") || s.ends_with(".tmp"))
        })
}

/// Sleduje kořen poznámek (rekurzivně, jen události) — i poznámky, které zapíše Claude.
pub(crate) fn spawn_notes_watch(rx: mpsc::Receiver<NotesMsg>, tx: mpsc::Sender<NotesMsg>) {
    use notify::{RecursiveMode, Watcher};
    std::thread::Builder::new()
        .name("notes-watch".into())
        .spawn(move || {
            let mut _watcher: Option<notify::RecommendedWatcher> = None;
            let mut rewatch = true;
            let mut last_rewatch = Instant::now() - Duration::from_secs(60);
            let mut swept = false;
            loop {
                if rewatch {
                    _watcher = None;
                    last_rewatch = Instant::now();
                    let root = notes_root();
                    core().notes.lock().peeked = root.as_deref().map(notes::load_peeked).unwrap_or_default();
                    if let Some(root) = root {
                        let _ = std::fs::create_dir_all(&root);
                        let tx = tx.clone();
                        let w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                            if res.as_ref().is_ok_and(|ev| notes_event_syncable(&ev.paths)) {
                                notify_sync_local();
                            }
                            let msg = match res {
                                Ok(ev) if notes_event_relevant(&ev.paths) => NotesMsg::Fs,
                                Ok(_) => return,
                                // watcher přišel o složku (smazaná / přesunutá) → sledovat znovu
                                Err(_) => NotesMsg::Lost,
                            };
                            let _ = tx.send(msg);
                        });
                        match w.and_then(|mut w| w.watch(&root, RecursiveMode::Recursive).map(|_| w)) {
                            Ok(w) => _watcher = Some(w),
                            Err(e) => log(&format!("poznámky: nejde sledovat {}: {e}", root.display())),
                        }
                    }
                    reload_notes();
                    if !std::mem::replace(&mut swept, true) {
                        // zbytky smazání z minula (pád uprostřed lhůty „Zpět“) jdou do Koše
                        std::thread::spawn(crate::commands::noteslife::sweep_trash);
                    }
                }
                let Ok(msg) = rx.recv() else { break };
                // chyby watcheru můžou chodit v sérii → kvůli nim znovu sledovat nejvýš jednou za 5 s
                let lost_ok = last_rewatch.elapsed() > Duration::from_secs(5);
                let wants = |m: &NotesMsg| matches!(m, NotesMsg::Reroot) || (matches!(m, NotesMsg::Lost) && lost_ok);
                rewatch = wants(&msg);
                // debounce: jeden zápis = několik událostí (tmp, rename)
                std::thread::sleep(Duration::from_millis(200));
                while let Ok(m) = rx.try_recv() {
                    rewatch |= wants(&m);
                }
                if !rewatch {
                    reload_notes();
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_events_ignore_git_internals() {
        let p = |s: &str| vec![PathBuf::from(s)];
        let root = r"C:\Users\x\Ledge\notes";
        // vnitřek .git (každá operace syncu) ani lokální stav nepřenačítá a nespouští sync
        for s in [r"\.git\index", r"\.git\refs\heads\main", r"\.git", r"\.draft\draft.json", r"\.state.json", r"\_obecne\a.md.1-0.tmp", r"\.smazane\1-1\app\a.md"] {
            let full = p(&format!("{root}{s}"));
            assert!(!notes_event_relevant(&full) && !notes_event_syncable(&full), "{s}");
        }
        // poznámka: obojí; obrázek: jen sync (reload ho nepotřebuje); .gitignore není vnitřek .git
        assert!(notes_event_relevant(&p(&format!(r"{root}\_obecne\a.md"))) && notes_event_syncable(&p(&format!(r"{root}\_obecne\a.md"))));
        assert!(!notes_event_relevant(&p(&format!(r"{root}\_obecne\a-1.png"))) && notes_event_syncable(&p(&format!(r"{root}\_obecne\a-1.png"))));
        assert!(notes_event_syncable(&p(&format!(r"{root}\.gitignore"))));
        assert!(!notes_event_syncable(&p("C:/Users/x/Ledge/notes/.git/HEAD")), "i s lomítky");
        assert!(notes_event_relevant(&[]) && notes_event_syncable(&[]));
    }
}
