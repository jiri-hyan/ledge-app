//! Tauri příkazy karty projektů a Repa: spuštění/zastavení (`procman.rs`), logy, otevření
//! v terminálu/složce, `git pull --ff-only`, logo, klonování (`cloning.rs`).

use crate::i18n::tr;
use crate::config::ProjectKind;
use crate::procman::{AppSpec, DevSpec, LaunchOutcome};
use crate::refresh::refresh_git_async;
use crate::state::{UiCmd, core, find_project, mark_dirty};
use crate::gitinfo::{self, Captured, RunError};
use crate::{cloning, resolve};
use crate::resolve::Place;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Proč projekt nemá složku — srozumitelně a s tím, co udělat (dlaždice nabídne tytéž kroky).
pub(crate) fn place_error(place: &Place) -> String {
    match place {
        Place::Missing { path } => trf!(
            "Složka {path} neexistuje. Vyber jinou: ⋯ na dlaždici → Změnit složku…",
            "The folder {path} doesn't exist. Pick another one: ⋯ on the tile → Change folder…"
        ),
        Place::Choose { count } => trf!(
            "Našel jsem {count} složky s tímto repem — vyber na dlaždici, ze které se má spouštět.",
            "I found {count} folders with this repo — pick on the tile which one to run from."
        ),
        _ => tr(
            "Složka projektu nenalezena. Vyber ji: ⋯ na dlaždici → Změnit složku…",
            "Project folder not found. Pick it: ⋯ on the tile → Change folder…",
        )
        .into(),
    }
}

#[tauri::command(async)]
pub(crate) fn launch(id: String) -> Result<LaunchOutcome, String> {
    let c = core();
    let p = find_project(&id)?;
    match p.kind {
        ProjectKind::Dev => {
            let place = c.project_place(&p);
            let dir = c.project_dir(&p).ok_or_else(|| place_error(&place.0))?;
            let run = p.run.clone().unwrap_or_default();
            if run.trim().is_empty() {
                return Err(tr("Projekt nemá příkaz ke spuštění. Doplň ho: ⋯ na dlaždici → Změnit příkaz.", "The project has no command to run. Add one: ⋯ on the tile → Change command.").into());
            }
            c.procs.launch_dev(DevSpec {
                id: p.id.clone(),
                cwd: dir,
                run,
                port: p.port,
                pass_port: p.pass_port,
                open_browser: p.open_browser,
                timeout: Duration::from_secs(90),
            })
        }
        ProjectKind::App => {
            // aplikace ze Storu: bez exe, spustí ji shell; proces se nesleduje (okno si otevře sama)
            if p.exe.is_none() && p.app_name.is_none() && let Some(aumid) = p.aumid.as_deref() {
                crate::onboarding::store::launch(aumid)?;
                return Ok(LaunchOutcome::Started);
            }
            let dir = c.project_dir(&p);
            let exe = match (&p.exe, &p.app_name) {
                (Some(e), _) => {
                    let ep = PathBuf::from(e);
                    if ep.is_absolute() { ep } else { dir.clone().ok_or_else(|| place_error(&c.project_place(&p).0))?.join(ep) }
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

/// Jak dlouho čeká Restart, než zabitý server pustí port (jinak by nový běh dostal jiný port).
const RESTART_PORT_WAIT: Duration = Duration::from_secs(3);

/// Restart = Stop (celý strom) → počkat na uvolnění portu → spustit znovu stejně jako klik.
#[tauri::command(async)]
pub(crate) fn restart(id: String) -> Result<LaunchOutcome, String> {
    let c = core();
    let port = c.procs.snapshot().into_iter().find(|v| v.id == id).and_then(|v| v.port);
    c.procs.stop_wait(&id); // aplikace: počká na zavření, ať Restart nepustí 2. instanci
    if let Some(port) = port {
        let t0 = Instant::now();
        while crate::ports::owner(port).is_some() && t0.elapsed() < RESTART_PORT_WAIT {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    launch(id)
}

/// „Zastavit všechny servery“ (paleta, pilulka): jen to, co spustil Ledge — každý celý strom.
#[tauri::command]
pub(crate) fn stop_all_servers() -> usize {
    let c = core();
    let live: Vec<String> = c.procs.snapshot().into_iter().filter(|v| v.phase.is_live()).map(|v| v.id).collect();
    for id in &live {
        c.procs.stop(id);
    }
    live.len()
}

/// Složky s repem projektu pro výběr (nejčerstvější commit první). `current` = co je teď v `path`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Candidates {
    current: Option<String>,
    items: Vec<resolve::Candidate>,
}

/// Výběr „Našel jsem 2 složky s tímto repem…“: git se ptá jen tady (na výslovné otevření výběru).
#[tauri::command(async)]
pub(crate) fn project_folders(id: String) -> Result<Candidates, String> {
    let c = core();
    let p = find_project(&id)?;
    let paths: Vec<PathBuf> = match p.remote.as_deref() {
        Some(r) => c.index.lock().candidates(r).to_vec(),
        None => Vec::new(),
    };
    Ok(Candidates { current: p.path.clone(), items: resolve::candidates_dated(&paths) })
}

/// Ruční „Prohledat složky znovu“: přestaví index repozitářů v `roots`, vrátí počet nalezených repozitářů.
#[tauri::command(async)]
pub(crate) fn repos_rescan() -> usize {
    let c = core();
    let n = c.rescan_index();
    mark_dirty();
    n
}

/// Port projektu drží proces, který nespustil Ledge — jen ukázat (a ukončit až po potvrzení).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortHolder {
    id: String,
    port: u16,
    pid: u32,
    process: String,
}

/// Cizí procesy na portech projektů (`port` v configu), které neběží přes Ledge. Jen na otevření pilulky.
#[tauri::command(async)]
pub(crate) fn port_holders() -> Vec<PortHolder> {
    let c = core();
    let ours: Vec<u16> = c.procs.snapshot().into_iter().filter(|v| v.phase.is_live()).filter_map(|v| v.port).collect();
    let projects: Vec<(String, u16)> = c.cfg.read().projects.iter().filter(|p| p.enabled).filter_map(|p| Some((p.id.clone(), p.port?))).collect();
    let listeners = crate::ports::listeners();
    let mut out = Vec::new();
    for (id, port) in projects {
        if ours.contains(&port) {
            continue;
        }
        let Some(l) = listeners.iter().find(|l| l.port == port) else { continue };
        if c.procs.project_for_pid(l.pid).is_some() {
            continue;
        }
        out.push(PortHolder { id, port, pid: l.pid, process: crate::ports::describe(l.pid) });
    }
    out
}

/// Ukončí cizí proces na portu — UI se nejdřív zeptá. Zabije jen ten PID, který port drží TEĎ
/// (mezitím ho mohl převzít jiný proces → nic), a nikdy proces z vlastního jobu (na to je Stop).
#[tauri::command(async)]
pub(crate) fn kill_port_holder(port: u16, pid: u32) -> Result<(), String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};
    if crate::ports::owner(port) != Some(pid) {
        return Err(trf!("Port {port} už drží jiný proces — nic jsem neukončil.", "Port {port} is held by another process now — nothing was stopped."));
    }
    if core().procs.project_for_pid(pid).is_some() || pid == std::process::id() {
        return Err(tr("Tenhle server spustil Ledge — zastav ho tlačítkem Stop.", "Ledge started this server — use Stop.").into());
    }
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| crate::errors::any(tr("Proces nejde ukončit", "Can't stop the process"), &e))?;
        let res = TerminateProcess(h, 1);
        let _ = CloseHandle(h);
        res.map_err(|e| crate::errors::any(tr("Proces nejde ukončit", "Can't stop the process"), &e))?;
    }
    mark_dirty();
    Ok(())
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
    open_dir(&dir, &target)
}

/// Cesta z `@cesta` v poznámce → existující soubor UVNITŘ složky projektu (L9). Odmítne absolutní cesty,
/// `..`, jednotku/UNC a cokoli, co po vyřešení (včetně junction/symlinku) vede mimo `dir`; složka není soubor.
pub(crate) fn resolve_in_project(dir: &Path, rel: &str) -> Result<PathBuf, String> {
    use std::path::Component;
    let bad = || tr("Soubor je mimo složku projektu nebo neexistuje.", "The file is outside the project folder or doesn't exist.").to_string();
    let rel = rel.trim().replace('/', "\\");
    let p = Path::new(&rel);
    if rel.is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(bad());
    }
    let root = dir.canonicalize().map_err(|_| bad())?;
    let real = root.join(p).canonicalize().map_err(|_| bad())?;
    (real.starts_with(&root) && real.is_file()).then_some(real).ok_or_else(bad)
}

/// Přípona souboru z `@cesta` smí do výchozí aplikace jen z allowlistu odkazů (`personal::FILE_ALLOW`, bez `.exe`):
/// poznámku mohl napsat Claude přes MCP, takže `@x.bat`/`.lnk`/`.ps1` se po kliku nesmí spustit (N3).
fn openable_ext(p: &Path) -> Result<(), String> {
    let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if crate::personal::FILE_ALLOW.contains(&ext.as_str()) {
        return Ok(());
    }
    Err(if ext.is_empty() {
        tr("Soubor bez přípony se z poznámky neotevírá — ukázán ve složce.", "A file without an extension isn't opened from a note — shown in its folder.").to_string()
    } else {
        trf!("Typ .{ext} se z poznámky neotevírá (jen dokumenty, obrázky, média, zip) — ukázán ve složce.", "Type .{ext} isn't opened from a note (only documents, images, media, zip) — shown in its folder.")
    })
}

/// Klik na `@cesta` v náhledu poznámky: otevře soubor projektu výchozí aplikací (cesta musí zůstat ve složce
/// projektu, `resolve_in_project`). Šuplík se schová jako u `note_open`; fokus bere až otevřená aplikace.
#[tauri::command(async)]
pub(crate) fn project_open_file(id: String, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let c = core();
    let p = find_project(&id)?;
    let dir = c.project_dir(&p).ok_or_else(|| tr("Složka projektu nenalezena.", "Project folder not found."))?;
    let real = crate::notes::strip_verbatim(&resolve_in_project(&dir, &path)?);
    if let Err(why) = openable_ext(&real) {
        // spustitelný / neznámý typ (`.bat`, `.lnk`, `.ps1`…) se nespouští — jen se ukáže v Průzkumníku (N3)
        let _ = c.app.opener().reveal_item_in_dir(&real);
        c.send_ui(UiCmd::Hide);
        return Err(why);
    }
    c.app.opener().open_path(real.to_string_lossy(), None::<&str>).map_err(|e| crate::errors::any(tr("Soubor nejde otevřít", "Can't open the file"), &e))?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// Otevře `dir` v terminálu (`wt`, záložně `cmd`) nebo v Průzkumníku a schová šuplík. Sdílí karta
/// projektů/Repa (`open_in`) a panel Git v kartě Claude (`commands/sessiongit.rs`).
pub(crate) fn open_dir(dir: &Path, target: &str) -> Result<(), String> {
    let c = core();
    let detached = 0x0000_0008u32; // DETACHED_PROCESS
    let no_window = crate::procman::CREATE_NO_WINDOW;
    let res = match target {
        "terminal" => {
            let wt = std::process::Command::new("wt").arg("-d").arg(dir).creation_flags(detached).spawn();
            match wt {
                Ok(c) => Ok(c),
                Err(_) => std::process::Command::new("cmd")
                    .raw_arg(format!("/d /c start \"\" /d \"{}\" cmd", dir.display()))
                    .creation_flags(no_window)
                    .spawn(),
            }
        }
        "folder" => std::process::Command::new("explorer").arg(dir).spawn(),
        _ => return Err(tr("Neznámý cíl.", "Unknown target.").into()),
    };
    res.map(|_| ()).map_err(|e| crate::errors::spawn(tr("Složku nejde otevřít", "Can't open the folder"), &e))?;
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
        Ok(out) => Err(crate::errors::git(tr("Git pull selhal", "git pull failed"), out.code, &out.stderr)),
        Err(RunError::Timeout) => Err(trf!("git pull nedoběhl do {} min — přerušeno.", "git pull didn't finish within {} min — stopped.", PULL_TIMEOUT.as_secs() / 60)),
        Err(RunError::Spawn(e)) => Err(crate::errors::spawn_text(tr("Git nejde spustit", "Can't run git"), &e)),
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
    let root = c.cfg.read().roots.first().cloned().ok_or_else(|| tr("Nastavení nemá `roots` — není kam klonovat.", "The settings have no `roots` — nowhere to clone to."))?;
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
    fn resolve_in_project_stays_inside() {
        let t = tempfile::tempdir().unwrap();
        let proj = t.path().join("proj");
        std::fs::create_dir_all(proj.join("src")).unwrap();
        std::fs::write(proj.join("src").join("a.ts"), "x").unwrap();
        std::fs::write(t.path().join("tajne.txt"), "x").unwrap();
        let abs_out = t.path().join("tajne.txt").to_string_lossy().to_string();
        for (rel, ok) in [
            ("src/a.ts", true),
            ("src\\a.ts", true),
            ("./src/a.ts", true),
            (" src/a.ts ", true),
            ("src/../src/a.ts", false),
            ("../tajne.txt", false),
            ("src/../../tajne.txt", false),
            (abs_out.as_str(), false),
            ("C:/Windows/notepad.exe", false),
            ("//server/share/x", false),
            ("src", false),
            ("src/neni.ts", false),
            ("", false),
        ] {
            let r = resolve_in_project(&proj, rel);
            assert_eq!(r.is_ok(), ok, "{rel:?}: {r:?}");
        }
        assert!(resolve_in_project(&proj, "src/a.ts").unwrap().ends_with("src\\a.ts"));
        assert!(resolve_in_project(&proj.join("neni"), "src/a.ts").is_err());
    }

    #[test]
    fn openable_ext_allowlist() {
        for (f, ok) in [
            ("a.md", true), ("A.PDF", true), ("x.png", true), ("b.zip", true),
            ("x.bat", false), ("x.cmd", false), ("x.lnk", false), ("x.ps1", false), ("x.exe", false),
            ("x.vbs", false), ("x.js", false), ("x.url", false), ("x.hta", false), ("x.scr", false), ("Makefile", false), ("a.txt.bat", false),
        ] {
            assert_eq!(openable_ext(Path::new(f)).is_ok(), ok, "{f}");
        }
    }

    #[test]
    fn pull_message_reports_last_line_and_errors() {
        let out = |success: bool, stdout: &str, stderr: &str| Ok(Captured { success, code: Some(i32::from(!success)), stdout: stdout.into(), stderr: stderr.into() });
        type Case = (Result<Captured, RunError>, Result<&'static str, &'static str>);
        let cases: [Case; 5] = [
            (out(true, "Updating a..b
Fast-forward
 1 file changed

", ""), Ok(" 1 file changed")),
            (out(true, "", ""), Ok("")),
            (out(false, "", "hint: x
fatal: Not possible to fast-forward, aborting.
"), Err("Git pull selhal: větev se rozešla se vzdálenou. Rychlé posunutí nejde — sluč ji ručně.")),
            (Err(RunError::Timeout), Err("git pull nedoběhl do 2 min — přerušeno.")),
            (Err(RunError::Spawn("nenalezeno (os error 2)".into())), Err("Git nejde spustit: program nejde spustit. Zkontroluj, že je nainstalovaný a cesta k němu je správná.")),
        ];
        for (res, want) in cases {
            assert_eq!(pull_message(res), want.map(String::from).map_err(String::from));
        }
    }
}
