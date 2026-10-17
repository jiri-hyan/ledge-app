//! Tauri příkazy poznámek: tenké obálky nad `notes.rs` / `notes::edit` (I/O jen tady, výběry
//! z paměti `NotesState`), koncepty, „Synchronizovat teď“, předvyplnění projektu.

use crate::i18n::tr;
use crate::notes;
use crate::state::{UiCmd, core, log};
use crate::strip::Rgba;
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{notes_root, reload_notes};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri_plugin_opener::OpenerExt;

/// Chyba „poznámky vypnuté“ (`ok_or_else(off)`).
fn off() -> &'static str {
    tr("Poznámky jsou vypnuté v Nastavení.", "Notes are off in Settings.")
}

/// Uloží rychlou poznámku (text, projekt, připomínka z `datetime-local` = lokální čas, PNG base64).
#[tauri::command(async)]
pub(crate) fn note_create(note: notes::NewNote) -> Result<String, String> {
    let root = notes_root().ok_or_else(off)?;
    let path = notes::create(&root, &note, &notes::now_local())?;
    log(&format!("poznámka: {}", path.display()));
    // šuplík se po uložení hned zavírá → potvrzení krátkým zeleným bliknutím proužku
    *core().flash.lock() = Some((Rgba::GREEN, Instant::now()));
    reload_notes();
    send_sync(SyncMsg::Local);
    Ok(path.to_string_lossy().into_owned())
}

/// Nejnovější poznámky napříč projekty (z paměti, bez I/O).
#[tauri::command]
pub(crate) fn note_list(limit: Option<usize>) -> Vec<notes::NoteView> {
    core().notes.lock().all.iter().take(limit.unwrap_or(50)).cloned().collect()
}

#[tauri::command(async)]
pub(crate) fn note_done(path: String, done: bool) -> Result<(), String> {
    let root = notes_root().ok_or_else(off)?;
    notes::set_done(&root, Path::new(&path), done)?;
    reload_notes();
    send_sync(SyncMsg::Local);
    Ok(())
}

/// Výběr poznámek pro záložku karty / stránku Poznámky (z paměti, bez I/O).
#[tauri::command]
pub(crate) fn note_query(q: notes::edit::NoteQuery) -> notes::edit::QueryResult {
    let all = &core().notes.lock().all;
    notes::edit::query(all, &q, notes::end_of_day_ms(&notes::now_local()))
}

/// Poznámka k úpravě v Ledge: tělo, otisk (ochrana proti přepsání) a obrázky jako data URL.
#[tauri::command(async)]
pub(crate) fn note_read(path: String) -> Result<notes::edit::NoteDetail, String> {
    notes::edit::read(&notes_root().ok_or_else(off)?, Path::new(&path))
}

/// Uloží úpravu (`copy` = jako novou poznámku vedle původní). Jiný obsah na disku → `changed`.
#[tauri::command(async)]
pub(crate) fn note_save(note: notes::edit::SaveNote, copy: bool) -> Result<notes::edit::SaveOutcome, String> {
    let root = notes_root().ok_or_else(off)?;
    let now = notes::now_local();
    let out = if copy {
        let path = notes::edit::save_copy(&root, &note, &now)?;
        notes::edit::SaveOutcome::Saved { note: Box::new(notes::edit::read(&root, &path)?) }
    } else {
        notes::edit::save(&root, &note, &now)?
    };
    if matches!(out, notes::edit::SaveOutcome::Saved { .. }) {
        reload_notes();
        send_sync(SyncMsg::Local);
    }
    Ok(out)
}

/// Klik na úkol v náhledu: `- [ ]` ↔ `- [x]` na řádku `line` těla (s ochranou proti přepsání).
#[tauri::command(async)]
pub(crate) fn note_toggle_task(path: String, line: usize, checked: bool, expected_hash: String) -> Result<notes::edit::SaveOutcome, String> {
    let root = notes_root().ok_or_else(off)?;
    let out = notes::edit::toggle_task(&root, Path::new(&path), line, checked, &expected_hash, &notes::now_local())?;
    if matches!(out, notes::edit::SaveOutcome::Saved { .. }) {
        reload_notes();
        send_sync(SyncMsg::Local);
    }
    Ok(out)
}

/// Náhled Markdownu (bezpečné HTML bez raw HTML ze vstupu, viz `notes::md`).
#[tauri::command]
pub(crate) fn note_render(body: String) -> String {
    notes::md::to_html(&body)
}

/// Rozepsaná úprava existující poznámky (`<root>\.draft\edit.json`); `None` = smazat.
#[tauri::command(async)]
pub(crate) fn note_edit_draft_save(draft: Option<notes::edit::EditDraft>) -> Result<(), String> {
    notes::edit::edit_draft_save(&notes_root().ok_or_else(off)?, draft.as_ref())
}

#[tauri::command(async)]
pub(crate) fn note_edit_draft_load() -> Option<notes::edit::EditDraft> {
    notes::edit::edit_draft_load(&notes_root()?)
}

/// „Synchronizovat teď“ (Nastavení): počká na doběhnutí a vrátí krátký výsledek.
#[tauri::command(async)]
pub(crate) fn notes_sync_now() -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    core().sync_tx.lock().send(SyncMsg::Now(tx)).map_err(|_| tr("Vlákno synchronizace neběží.", "The sync thread isn't running.").to_string())?;
    rx.recv_timeout(Duration::from_secs(300)).map_err(|_| tr("Synchronizace nestihla doběhnout.", "Sync didn't finish in time.").to_string())?
}

/// Otevře `.md` výchozí aplikací — jen soubor uvnitř úložiště poznámek.
#[tauri::command]
pub(crate) fn note_open(path: String) -> Result<(), String> {
    let root = notes_root().ok_or_else(off)?;
    // otevírá se ověřená kanonická cesta (ne řetězec z UI), bez prefixu \\?\, který shell neumí
    let real = notes::strip_verbatim(&notes::check_note_path(&root, Path::new(&path))?);
    let c = core();
    c.app.opener().open_path(real.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// Rozepsaná poznámka (`<root>\.draft\`) — obnoví se při otevření stránky „note“.
#[tauri::command(async)]
pub(crate) fn note_draft_load() -> Option<notes::Draft> {
    notes::draft_load(&notes_root()?)
}

/// Text / projekt / připomínka konceptu (frontend volá s debounce ~500 ms).
#[tauri::command(async)]
pub(crate) fn note_draft_save(meta: notes::DraftMeta) -> Result<(), String> {
    notes::draft_save_meta(&notes_root().ok_or_else(off)?, &meta)
}

/// Obrázky konceptu — jen když se změnily (PNG base64).
#[tauri::command(async)]
pub(crate) fn note_draft_images(images: Vec<String>) -> Result<(), String> {
    notes::draft_save_images(&notes_root().ok_or_else(off)?, &images)
}

#[tauri::command(async)]
pub(crate) fn note_draft_clear() -> Result<(), String> {
    match notes_root() {
        Some(r) => notes::draft_clear(&r),
        None => Ok(()),
    }
}

/// Předvyplněný projekt rychlé poznámky: podle cwd naposledy aktivní Claude session, jinak nic.
#[tauri::command]
pub(crate) fn note_prefill() -> Option<String> {
    let c = core();
    let cfg = c.cfg.read().clone();
    let dirs: Vec<(String, String)> = cfg
        .projects
        .iter()
        .filter(|p| p.enabled)
        .filter_map(|p| c.project_dir(p).map(|d| (p.id.clone(), d.to_string_lossy().into_owned())))
        .collect();
    let sessions: Vec<(String, u64)> = c.claude.sessions().into_iter().filter(|s| !s.ended).map(|s| (s.cwd, s.updated_at)).collect();
    notes::preselect(&dirs, &sessions)
}
