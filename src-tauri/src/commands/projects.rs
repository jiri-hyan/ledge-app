//! Tauri příkazy karty projektů a Repa: spuštění/zastavení (`procman.rs`), logy, otevření
//! v terminálu/složce, `git pull --ff-only`, logo, klonování (`cloning.rs`).

use crate::i18n::tr;
use crate::config::ProjectKind;
use crate::procman::{AppSpec, DevSpec, LaunchOutcome};
use crate::refresh::refresh_git_async;
use crate::state::{UiCmd, core, find_project, mark_dirty};
use crate::gitinfo::{self, Captured, RunError};
use crate::{cloning, resolve};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[tauri::command(async)]
pub(crate) fn launch(id: String) -> Result<LaunchOutcome, String> {
    let c = core();
    let p = find_project(&id)?;
    match p.kind {
        ProjectKind::Dev => {
            let dir = c.project_dir(&p).ok_or_else(|| tr("Složka projektu nenalezena — doplň `path` nebo `remote`.", "Project folder not found — fill in `path` or `remote`."))?;
            c.procs.launch_dev(DevSpec {
                id: p.id.clone(),
                cwd: dir,
                run: p.run.clone().unwrap_or_default(),
                port: p.port,
                pass_port: p.pass_port,
                open_browser: p.open_browser,
                timeout: Duration::from_secs(90),
            })
        }
        ProjectKind::App => {
            let dir = c.project_dir(&p);
            let exe = match (&p.exe, &p.app_name) {
                (Some(e), _) => {
                    let ep = PathBuf::from(e);
                    if ep.is_absolute() { ep } else { dir.clone().ok_or_else(|| tr("Složka projektu nenalezena.", "Project folder not found."))?.join(ep) }
                }
                (None, Some(name)) => resolve::installed_app_exe(name)
                    .ok_or_else(|| trf!("{name} není nainstalované. Instalátor najdeš v odkazu „Vydání“.", "{name} isn't installed. You'll find the installer under the “Releases” link."))?,
                (None, None) => return Err(tr("Projekt nemá exe ani appName.", "The project has neither exe nor appName.").into()),
            };
            let prepare = if p.exe.is_some() { p.run.clone() } else { None };
            c.procs.launch_app(AppSpec { id: p.id.clone(), cwd: dir, prepare, exe, args: p.args.clone() })
        }
    }
}

#[tauri::command]
pub(crate) fn stop(id: String) {
    core().procs.stop(&id);
}

#[tauri::command]
pub(crate) fn dismiss(id: String) {
    core().procs.dismiss(&id);
}

#[tauri::command]
pub(crate) fn logs(id: String) -> Vec<String> {
    core().procs.log(&id)
}

#[tauri::command(async)]
pub(crate) fn open_in(id: String, target: String) -> Result<(), String> {
    let c = core();
    let p = find_project(&id)?;
    let dir = c.project_dir(&p).ok_or_else(|| tr("Složka projektu nenalezena.", "Project folder not found."))?;
    let detached = 0x0000_0008u32; // DETACHED_PROCESS
    let no_window = 0x0800_0000u32;
    let res = match target.as_str() {
        "terminal" => {
            let wt = std::process::Command::new("wt").arg("-d").arg(&dir).creation_flags(detached).spawn();
            match wt {
                Ok(c) => Ok(c),
                Err(_) => std::process::Command::new("cmd")
                    .raw_arg(format!("/d /c start \"\" /d \"{}\" cmd", dir.display()))
                    .creation_flags(no_window)
                    .spawn(),
            }
        }
        "folder" => std::process::Command::new("explorer").arg(&dir).spawn(),
        _ => return Err(tr("Neznámý cíl.", "Unknown target.").into()),
    };
    res.map(|_| ()).map_err(|e| e.to_string())?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// Termín pro `git pull` z karty Repa: pak se strom gitu zabije (`gitinfo::capture`).
const PULL_TIMEOUT: Duration = Duration::from_secs(120);

/// `git pull --ff-only` — jen rychlé posunutí, nikdy merge ani přepsání lokální práce. S termínem
/// (`PULL_TIMEOUT`, zabití celého stromu) a bez ptaní na heslo v terminálu, který stejně nikdo nevidí.
#[tauri::command(async)]
pub(crate) fn git_pull(id: String) -> Result<String, String> {
    let c = core();
    let p = find_project(&id)?;
    let dir = c.project_dir(&p).ok_or_else(|| tr("Složka projektu nenalezena.", "Project folder not found."))?;
    let res = gitinfo::run_env(&dir, &["pull", "--ff-only"], &[("GIT_TERMINAL_PROMPT", "0")], Instant::now() + PULL_TIMEOUT);
    if !matches!(res, Err(RunError::Spawn(_))) {
        c.git.lock().remove(&id);
        refresh_git_async();
    }
    pull_message(res)
}

/// Krátká odpověď pro `say()`: poslední neprázdný řádek výstupu, česky chyba / termín.
fn pull_message(res: Result<Captured, RunError>) -> Result<String, String> {
    let last = |s: &str| s.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("").to_string();
    match res {
        Ok(out) if out.success => Ok(last(&out.stdout)),
        Ok(out) => Err(trf!("git pull selhal: {}", "git pull failed: {}", last(&out.stderr))),
        Err(RunError::Timeout) => Err(trf!("git pull nedoběhl do {} min — přerušeno.", "git pull didn't finish within {} min — stopped.", PULL_TIMEOUT.as_secs() / 60)),
        Err(RunError::Spawn(e)) => Err(trf!("git nejde spustit: {e}", "can't run git: {e}")),
    }
}

/// Data URL loga projektu (UI si ho vyžádá jednou na `logoKey`).
#[tauri::command]
pub(crate) fn project_logo(id: String) -> Option<String> {
    core().logos.lock().url(&id)
}

/// „Naklonovat“ chybějící projekt do `roots[0]` (`cloning.rs`), pak přestavět repo index.
#[tauri::command(async)]
pub(crate) fn clone_project(id: String) -> Result<String, String> {
    let c = core();
    let p = find_project(&id)?;
    let remote = p.remote.clone().ok_or_else(|| tr("Projekt nemá `remote` — není co klonovat.", "The project has no `remote` — nothing to clone."))?;
    let path_ok = c.project_dir(&p).is_some_and(|d| d.is_dir());
    if !cloning::clonable(&p, path_ok, c.index.lock().clones(&remote)) {
        return Err(trf!("{} se klonovat nedá (už je na disku, nebo je klonů víc).", "{} can't be cloned (it's already on disk, or there are several clones).", p.name));
    }
    let root = c.cfg.read().roots.first().cloned().ok_or_else(|| tr("Config nemá `roots` — není kam klonovat.", "The config has no `roots` — nowhere to clone to."))?;
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(trf!("Kořen projektů {} neexistuje.", "Projects root {} doesn't exist.", root.display()));
    }
    let target = cloning::target_in(&root, &remote, Path::exists).ok_or_else(|| tr("Volné jméno složky nenalezeno.", "No free folder name found."))?;
    let busy = cloning::Busy::claim(&id).ok_or_else(|| tr("Klonování už běží.", "Cloning is already running."))?;
    mark_dirty();
    let res = cloning::run(&remote, &target);
    c.rebuild_index();
    c.git.lock().remove(&id);
    drop(busy);
    mark_dirty();
    res?;
    match c.project_dir(&p) {
        Some(_) => Ok(trf!("{} naklonováno do {}", "{} cloned into {}", p.name, target.display())),
        None => Ok(trf!(
            "Naklonováno do {}, ale podle remote se projekt nedohledal — doplň `path`.",
            "Cloned into {}, but the project wasn't found by its remote — fill in `path`.",
            target.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_message_reports_last_line_and_errors() {
        let out = |success: bool, stdout: &str, stderr: &str| Ok(Captured { success, stdout: stdout.into(), stderr: stderr.into() });
        type Case = (Result<Captured, RunError>, Result<&'static str, &'static str>);
        let cases: [Case; 5] = [
            (out(true, "Updating a..b
Fast-forward
 1 file changed

", ""), Ok(" 1 file changed")),
            (out(true, "", ""), Ok("")),
            (out(false, "", "hint: x
fatal: Not possible to fast-forward, aborting.
"), Err("git pull selhal: fatal: Not possible to fast-forward, aborting.")),
            (Err(RunError::Timeout), Err("git pull nedoběhl do 2 min — přerušeno.")),
            (Err(RunError::Spawn("nenalezeno".into())), Err("git nejde spustit: nenalezeno")),
        ];
        for (res, want) in cases {
            assert_eq!(pull_message(res), want.map(String::from).map_err(String::from));
        }
    }
}
