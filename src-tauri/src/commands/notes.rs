//! Tauri příkazy poznámek: tenké obálky nad `notes.rs` / `notes::edit` (I/O jen tady, výběry
//! z paměti `NotesState`), koncepty, „Synchronizovat teď“, předvyplnění projektu.

use crate::i18n::tr;
use crate::notes;
use crate::state::{UiCmd, core, log};
use crate::strip::Rgba;
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{notes_root, reload_notes};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
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
    let st = core().notes.lock();
    let all = if q.archived { &st.archived } else { &st.all };
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

/// Návrh připomínky z textu poznámky („zítra v 10“ → `{label: "zítra 10:00", iso}`), `None` když v textu
/// čas není. Čistá funkce `notes::when` (bez I/O) — editor ji volá s debounce, jen když poznámka nemá
/// připomínku.
#[tauri::command]
pub(crate) fn note_suggest_remind(text: String) -> Option<notes::when::Suggest> {
    notes::when::suggest(&text, &notes::now_local())
}

/// Index těla poznámek pro Ctrl+K — staví se líně při prvním hledání po změně poznámek (`rev`).
static SEARCH: parking_lot::Mutex<Option<notes::search::Index>> = parking_lot::Mutex::new(None);

/// Ctrl+K: poznámky, v jejichž titulku nebo těle jsou všechna slova dotazu (bez ohledu na velikost
/// písmen a diakritiku), s úryvkem. Dotaz kratší než 2 znaky = nic. Hotové a archivované se nehledají.
#[tauri::command(async)]
pub(crate) fn note_search(query: String, limit: Option<usize>) -> Vec<notes::search::Hit> {
    let Some(root) = notes_root() else { return Vec::new() };
    if query.trim().chars().count() < 2 {
        return Vec::new();
    }
    let mut idx = SEARCH.lock();
    let (rev, views) = {
        let st = core().notes.lock();
        let rev = st.rev;
        (rev, if idx.as_ref().is_some_and(|i| i.rev() == rev) { Vec::new() } else { st.all.clone() })
    };
    if idx.as_ref().is_none_or(|i| i.rev() != rev) {
        *idx = Some(notes::search::Index::build(&root, &views, rev));
    }
    idx.as_ref().map(|i| i.search(&query, limit.unwrap_or(8).clamp(1, 30))).unwrap_or_default()
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

/// Exportuje poznámku do souboru: `format` = `md` (Markdown bez frontmatteru, s titulkem) | `html`
/// (samostatný dokument s obrázky jako data URL). Dialog „Uložit jako“ (výchozí jméno = slug
/// poznámky); zrušení = `None`, jinak vrací uloženou cestu. Poznámka se nemění.
#[tauri::command(async)]
pub(crate) fn note_export(path: String, format: String) -> Result<Option<String>, String> {
    let root = notes_root().ok_or_else(off)?;
    let fmt = notes::export::Format::parse(&format)?;
    // `read` ověří cestu přes `check_note_path` (jen soubor v úložišti) a načte tělo i obrázky
    let note = notes::edit::read(&root, Path::new(&path))?;
    let c = core();
    let filter = match fmt {
        notes::export::Format::Md => "Markdown",
        notes::export::Format::Html => "HTML",
    };
    let mut d = c.app.dialog().file().add_filter(filter, &[fmt.ext()]).set_file_name(notes::export::default_name(&note.body, fmt));
    if let Some(w) = c.app.get_webview_window(crate::drawer::LABEL) {
        d = d.set_parent(&w);
    }
    let Some(target) = d.blocking_save_file().and_then(|p| p.into_path().ok()) else { return Ok(None) };
    let target = std::path::PathBuf::from(notes::export::with_ext(&target.to_string_lossy(), fmt));
    let out = notes::export::render(fmt, &note.view.title, &note.body, &note.image_data);
    notes::write_atomic(&target, out.as_bytes())?;
    log(&format!("poznámka exportována: {}", target.display()));
    Ok(Some(target.to_string_lossy().into_owned()))
}

/// Aplikace, které má Windows registrované pro `.md` a jejichž exe existuje (přesně ty, které
/// `note_open_with` spustí). Seřazené podle jména, bez duplicit stejného exe.
fn open_with_registry() -> Vec<(String, String, String)> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CLASSES_ROOT;
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let Ok(ids) = hkcr.open_subkey(notes::openwith::MD_PROGIDS_KEY) else { return Vec::new() };
    let env = |k: &str| std::env::var(k).ok();
    let mut out: Vec<(String, String, String)> = Vec::new(); // (progid, jméno, příkaz)
    for (id, _) in ids.enum_values().flatten() {
        let Ok(class) = hkcr.open_subkey(&id) else { continue };
        let Some(cmd) = class
            .open_subkey(r"shell\open\command")
            .and_then(|k| k.get_value::<String, _>(""))
            .ok()
            .map(|c| notes::openwith::expand_env(&c, env))
        else {
            continue;
        };
        let Some((exe, _)) = notes::openwith::command_for(&cmd, Path::new("x.md")) else { continue };
        if !exe.is_file() || out.iter().any(|(_, _, c)| notes::openwith::command_for(c, Path::new("x.md")).is_some_and(|(e, _)| e == exe)) {
            continue;
        }
        let app_name = class.open_subkey("Application").and_then(|k| k.get_value::<String, _>("ApplicationName")).ok();
        let desc = class.get_value::<String, _>("").ok();
        let name = notes::openwith::display_name(app_name.as_deref(), desc.as_deref(), &exe);
        out.push((id, name, cmd));
    }
    out.sort_by_key(|a| a.1.to_lowercase());
    out
}

#[derive(serde::Serialize)]
pub(crate) struct OpenWithApp {
    id: String,
    name: String,
}

/// Nabídka „Otevřít v…“ pro poznámky (menu ⋯ editoru i lístku).
#[tauri::command(async)]
pub(crate) fn note_open_with_apps() -> Vec<OpenWithApp> {
    open_with_registry().into_iter().map(|(id, name, _)| OpenWithApp { id, name }).collect()
}

/// Otevře poznámku v aplikaci `app` (ProgID z `note_open_with_apps`) jejím registrovaným příkazem.
/// Co aplikace se souborem udělá, je její věc — některé ho upravují na místě, jiné si ho naimportují
/// jako kopii (pak se změny do `.md` nevrací).
#[tauri::command(async)]
pub(crate) fn note_open_with(path: String, app: String) -> Result<(), String> {
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let root = notes_root().ok_or_else(off)?;
    let real = notes::strip_verbatim(&notes::check_note_path(&root, Path::new(&path))?);
    let (_, name, cmd) = open_with_registry()
        .into_iter()
        .find(|(id, _, _)| *id == app)
        .ok_or_else(|| tr("Ta aplikace už pro .md registrovaná není", "That app is no longer registered for .md").to_string())?;
    let (exe, args) = notes::openwith::command_for(&cmd, &real).ok_or_else(|| tr("Neplatný příkaz aplikace", "Invalid app command").to_string())?;
    std::process::Command::new(exe)
        .args(args)
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| trf!("{name} nejde spustit: {e}", "Can't start {name}: {e}"))?;
    core().send_ui(UiCmd::Hide);
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
