//! Tauri příkazy panelu Git v kartě Claude (`sessiongit/`). UI posílá jen id session (a cestu souboru);
//! cwd, kořen repa, worktree i stavy session v té složce si Rust zjistí sám — destruktivní akce se
//! blokují tady, ne jen v UI. Všechno jen na výslovný klik.

use crate::i18n::tr;
use crate::refresh::refresh_git_async;
use crate::sessiongit::{self, Repo, View, Waits, plan};
use crate::sessions::Status;
use crate::snapshot::session_rows;
use crate::state::{core, mark_dirty};
use crate::claude::preview::Preview;
use crate::config;
use std::path::Path;

/// Všechny řádky session (jako `session_open`): (cwd session, (cwd, stav) všech).
fn session(id: &str) -> Result<(String, Vec<(String, Status)>), String> {
    let c = core();
    let projects: Vec<config::Project> = c.cfg.read().projects.iter().filter(|p| p.enabled).cloned().collect();
    let dirs: Vec<(String, String)> = projects.iter().filter_map(|p| Some((p.id.clone(), c.project_dir(p)?.to_string_lossy().into_owned()))).collect();
    let name_of = |id: &str| projects.iter().find(|p| p.id == id).map(|p| p.name.clone());
    let rows = session_rows(&dirs, &name_of);
    let cwd = rows.iter().find(|r| r.id == id).map(|r| r.cwd.clone()).ok_or_else(|| tr("Session už v seznamu není.", "The session is no longer in the list."))?;
    Ok((cwd, rows.into_iter().map(|r| (r.cwd, r.status)).collect()))
}

/// Repo session + stavy session, jejichž cwd v něm leží.
fn repo(id: &str) -> Result<(Repo, Vec<Status>), String> {
    let (cwd, rows) = session(id)?;
    if cwd.is_empty() {
        return Err(tr("Session nemá pracovní složku.", "The session has no working folder.").into());
    }
    let repo = sessiongit::read_repo(Path::new(&cwd), Waits::default())?.ok_or_else(|| tr("Pracovní složka session není v gitu.", "The session's working folder isn't in git."))?;
    let here = plan::statuses_in(&repo.root, &rows);
    Ok((repo, here))
}

/// Po zápisu: karta Repa a hlídač nedokončené práce (●/↑) si stav přečtou znovu.
fn changed() {
    refresh_git_async();
    mark_dirty();
}

/// Rozbalení panelu Git. `None` = cwd není v gitu (nebo neexistuje).
#[tauri::command(async)]
pub(crate) fn session_git(id: String) -> Result<Option<View>, String> {
    let (cwd, rows) = session(&id)?;
    if cwd.is_empty() {
        return Ok(None);
    }
    let Some(repo) = sessiongit::read_repo(Path::new(&cwd), Waits::default())? else { return Ok(None) };
    Ok(Some(sessiongit::view(&repo, &plan::statuses_in(&repo.root, &rows))))
}

#[tauri::command(async)]
pub(crate) fn session_git_diff(id: String, path: String, staged: bool) -> Result<Preview, String> {
    let (repo, _) = repo(&id)?;
    sessiongit::file_diff(&repo, &path, staged, Waits::default())
}

#[tauri::command(async)]
pub(crate) fn session_git_commit(id: String, message: String, paths: Vec<String>) -> Result<String, String> {
    let (repo, _) = repo(&id)?;
    let res = sessiongit::commit(&repo, &message, &paths, Waits::default());
    changed();
    res
}

#[tauri::command(async)]
pub(crate) fn session_git_push(id: String) -> Result<String, String> {
    let (repo, _) = repo(&id)?;
    let res = sessiongit::push(&repo, Waits::default());
    changed();
    res
}

#[tauri::command(async)]
pub(crate) fn session_git_pull(id: String) -> Result<String, String> {
    let (repo, _) = repo(&id)?;
    let res = sessiongit::pull(&repo, Waits::default());
    changed();
    res
}

#[tauri::command(async)]
pub(crate) fn session_git_discard(id: String, path: String) -> Result<(), String> {
    let (repo, here) = repo(&id)?;
    let res = sessiongit::discard(&repo, &path, &here, Waits::default());
    changed();
    res
}

#[tauri::command(async)]
pub(crate) fn session_git_remove_worktree(id: String) -> Result<String, String> {
    let (repo, here) = repo(&id)?;
    let res = sessiongit::remove_worktree(&repo, &here, Waits::default());
    changed();
    res
}

/// Kořen repa session v terminálu / Průzkumníku (`open_dir` jako u projektů).
#[tauri::command(async)]
pub(crate) fn session_git_open(id: String, target: String) -> Result<(), String> {
    let (repo, _) = repo(&id)?;
    // `cmd start` (záloha bez `wt`) skládá příkazovou řádku — cesta s `"` / `%` tam nesmí
    if repo.root.contains(['"', '%']) {
        return Err(tr("Neobvyklá cesta repa — otevři ho ručně.", "Unusual repo path — open it manually.").into());
    }
    crate::commands::projects::open_dir(Path::new(&repo.root.replace('/', "\\")), &target)
}
