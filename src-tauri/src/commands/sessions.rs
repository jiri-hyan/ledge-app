//! Tauri příkazy Claude: rozhodnutí o žádosti (jen klik člověka), odpověď na otázku Clauda,
//! otevření session, hooky.

use crate::claude::{Reply, rules};
use crate::snapshot::session_rows;
use crate::state::{UiCmd, core, log, mark_dirty};
use crate::{config, hooksetup, sessions, winfocus};

/// `extra` (volitelné, aditivní): „Vždy povolit“ do cíle / důvod zamítnutí (`claude::rules`). Bez něj
/// jako dřív. Pravidlo bere Rust z čekající žádosti, z UI jde jen cíl.
#[tauri::command]
pub(crate) fn decide(id: Option<u64>, allow: bool, extra: Option<rules::Extra>) -> bool {
    core().claude.decide_with(id, allow, &extra.unwrap_or_default())
}

/// Klik na session v kartě Claude: přepnout do jejího okna, nebo skončenou obnovit v terminálu.
/// Rozhodnutí je čisté (`sessions::decide_open`), Win32 ve `winfocus.rs`. Chyba = česky pro `say()`.
#[tauri::command(async)]
pub(crate) fn session_open(id: String) -> Result<(), String> {
    let c = core();
    let projects: Vec<config::Project> = c.cfg.read().projects.iter().filter(|p| p.enabled).cloned().collect();
    let dirs: Vec<(String, String)> = projects.iter().filter_map(|p| Some((p.id.clone(), c.project_dir(p)?.to_string_lossy().into_owned()))).collect();
    let name_of = |id: &str| projects.iter().find(|p| p.id == id).map(|p| p.name.clone());
    let row = session_rows(&dirs, &name_of).into_iter().find(|r| r.id == id).ok_or_else(|| crate::i18n::tr("Session už v seznamu není.", "The session is no longer in the list."))?;
    match row.open_action() {
        sessions::OpenAction::SwitchDesktop { pid } => winfocus::switch_to(pid, true)?,
        sessions::OpenAction::SwitchTerminal { pid } => winfocus::switch_to(pid, false)?,
        sessions::OpenAction::Resume => winfocus::launch(&row.cwd, &row.id)?,
        sessions::OpenAction::Unavailable(why) => return Err(why.into()),
    }
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// Odpověď šéfa na otázku Clauda (klik v Ledge) — `answers` v pořadí otázek. Chyba = česky.
#[tauri::command]
pub(crate) fn question_answer(id: u64, answers: Vec<Reply>) -> Result<(), String> {
    core().claude.answer_question(id, &answers)
}

/// „Odpovědět v Claude“: Ledge otázku pustí do okna Claude.
#[tauri::command]
pub(crate) fn question_release(id: u64) -> bool {
    core().claude.release_question(id)
}

#[tauri::command]
pub(crate) fn hooks_status() -> hooksetup::HooksStatus {
    hooksetup::status_cached(core().cfg.read().claude.port)
}

/// Zapojí hooky Ledge do `~/.claude/settings.json` (jen na výslovný klik v Nastavení / kartě).
#[tauri::command]
pub(crate) fn hooks_install() -> Result<hooksetup::HooksStatus, String> {
    let port = core().cfg.read().claude.port;
    let path = hooksetup::settings_path().ok_or_else(|| crate::i18n::tr("Nevím, kde je složka .claude.", "I don't know where the .claude folder is."))?;
    hooksetup::apply(&path, |t| hooksetup::merge(t, port))?;
    log(&format!("hooky Ledge zapojeny do {}", path.display()));
    mark_dirty();
    Ok(hooksetup::status_cached(port))
}

#[tauri::command]
pub(crate) fn hooks_remove() -> Result<hooksetup::HooksStatus, String> {
    let port = core().cfg.read().claude.port;
    let path = hooksetup::settings_path().ok_or_else(|| crate::i18n::tr("Nevím, kde je složka .claude.", "I don't know where the .claude folder is."))?;
    hooksetup::apply(&path, hooksetup::remove)?;
    log(&format!("hooky Ledge odpojeny z {}", path.display()));
    mark_dirty();
    Ok(hooksetup::status_cached(port))
}
