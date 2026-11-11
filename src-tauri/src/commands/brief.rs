//! Tauri příkazy zadání pro Claude Code (kontrakt `src/api.ts` z větve UI kola 19): `brief_send`,
//! `brief_resume`, `brief_stop` a API klíč `apikey_set` / `apikey_status` / `apikey_clear`. Jen na
//! výslovný klik. Rozhodnutí v `briefrun::plan`, argumenty v `briefrun::args`, proces v `briefrun::proc`.
//! Klíč nikdy do logu, chyby ani argumentů.

use crate::accounts::vault::WinCred;
use crate::agentview::Program;
use crate::briefrun::{args, key, note, plan, proc, stream};
use crate::i18n::tr;
use crate::state::{core, find_project, log};
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{notes_root, reload_notes};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

fn off() -> String {
    tr("Poznámky jsou vypnuté.", "Notes are turned off.").into()
}

#[tauri::command]
pub(crate) fn apikey_set(key: String) -> Result<(), String> {
    key::set(&WinCred, &key)?;
    log("apikey: uložen do Správce pověření");
    Ok(())
}

#[tauri::command]
pub(crate) fn apikey_status() -> bool {
    key::get(&WinCred).is_some()
}

#[tauri::command]
pub(crate) fn apikey_clear() -> Result<(), String> {
    key::clear(&WinCred)?;
    log("apikey: smazán");
    Ok(())
}

/// Verze CLI jednou za běh Ledge (líně, při prvním zadání).
fn cli_version() -> Option<(u32, u32, u32)> {
    static V: OnceLock<Option<(u32, u32, u32)>> = OnceLock::new();
    *V.get_or_init(|| {
        let out = crate::agentrun::run(&["--version".to_string()], None, Duration::from_secs(15), 1024).ok()?;
        stream::parse_version(&String::from_utf8_lossy(&out.stdout))
    })
}

fn ledge_exe() -> String {
    std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "ledge.exe".into())
}

fn session_uuid() -> Result<String, String> {
    let r = crate::accounts::pkce::random(16)?;
    let mut b = [0u8; 16];
    b.copy_from_slice(&r[..16]);
    Ok(args::uuid_v4(b))
}

fn changed() {
    reload_notes();
    send_sync(SyncMsg::Local);
}

#[tauri::command(async)]
pub(crate) fn brief_send(path: String) -> Result<(), String> {
    launch(&path, false)
}

#[tauri::command(async)]
pub(crate) fn brief_resume(path: String) -> Result<(), String> {
    launch(&path, true)
}

#[tauri::command]
pub(crate) fn brief_stop(path: String) -> Result<(), String> {
    let root = notes_root().ok_or_else(off)?;
    let real = crate::notes::check_note_path(&root, Path::new(&path))?;
    if !proc::stop(&real) {
        // `running` v poznámce bez procesu (Ledge mezitím skončil / spadl) → zpět na koncept
        let p = Path::new(&path);
        if note::load(&root, p)?.status.as_deref() == Some("running") {
            let now = crate::notes::now_local();
            note::meta(&root, p, &[("status", Some("draft"))], &now)?;
            note::log(&root, p, "Běh už neexistoval (Ledge mezitím skončil) — zadání vráceno do konceptu.", &now)?;
            changed();
            return Ok(());
        }
        return Err(tr("Zadání neběží.", "The brief isn't running.").into());
    }
    // stav a log zapíše konec běhu (`finish` se `stopped`)
    Ok(())
}

fn launch(path: &str, resume: bool) -> Result<(), String> {
    // dva rychlé kliky nesmí projít kontrolou „neběží“ oba
    static LAUNCH: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
    let _one = LAUNCH.lock();
    let root = notes_root().ok_or_else(off)?;
    let note_path = PathBuf::from(path);
    let info = note::load(&root, &note_path)?;
    if proc::running(&info.real) {
        return Err(tr("Zadání už běží.", "The brief is already running.").into());
    }
    let cfg = core().cfg.read().clone();
    let key = key::get(&WinCred);
    let env = plan::Env { mcp_enabled: cfg.mcp.enabled, has_key: key.is_some(), default_model: cfg.claude.brief.model.clone(), default_budget: cfg.claude.brief.budget_usd };
    let p = plan::plan(&info, resume, &env)?;
    let key = key.ok_or_else(|| tr("Chybí API klíč.", "No API key.").to_string())?;
    let project = find_project(info.project.as_deref().unwrap_or_default())?;
    let cwd = core().project_dir(&project).filter(|d| d.is_dir()).ok_or_else(|| tr("Složka projektu nenalezena.", "Project folder not found."))?;
    let exe = match crate::agentrun::program()? {
        Program::Exe(e) => e,
        Program::Batch(_) => {
            return Err(tr(
                "Claude Code je tu jen jako claude.cmd — zadání přes cmd.exe neposílám. Nainstaluj nativní Claude Code (claude.exe).",
                "Claude Code is only available as claude.cmd here — briefs aren't sent through cmd.exe. Install native Claude Code (claude.exe).",
            )
            .into());
        }
    };
    let cli = cli_version();
    let session = match &p.resume_session {
        Some(s) => s.clone(),
        None => session_uuid()?,
    };
    let brief = args::Brief { session: session.clone(), resume, model: p.model.clone(), budget_left: p.left, ledge_exe: ledge_exe() };
    let (env_set, env_remove) = args::build_env(&key);
    let msg = if resume { args::resume_message(path) } else { args::first_message(path) };
    let now = crate::notes::now_local();
    let budget = format!("{}", p.budget);
    note::meta(&root, &note_path, &[("status", Some("running")), ("session", Some(&session)), ("model", Some(&p.model)), ("budget", Some(&budget))], &now)?;
    let start = proc::Start {
        real: info.real.clone(),
        exe,
        args: args::build_args(&brief),
        env_set,
        env_remove,
        cwd: cwd.clone(),
        stdin_line: args::user_line(&msg),
        key,
    };
    let (prev_cost, end_root, end_path) = (info.cost, root.clone(), note_path.clone());
    let started = proc::start(start, move |ended| finish(&end_root, &end_path, prev_cost, cli, resume, ended));
    if let Err(e) = started {
        let now = crate::notes::now_local();
        let _ = note::meta(&root, &note_path, &[("status", Some("draft"))], &now);
        let _ = note::log(&root, &note_path, &format!("Běh nejde spustit: {e}"), &now);
        changed();
        return Err(e);
    }
    let name = note_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    log(&format!("brief: {} {name} v {} (model {}, strop ${:.2})", if resume { "pokračuje" } else { "start" }, cwd.display(), p.model, p.left));
    changed();
    Ok(())
}

/// Konec běhu: náklad, stav (`waiting` / `review` / `draft`) a případný řádek do `## Log`.
fn finish(root: &Path, path: &Path, prev_cost: f64, cli: Option<(u32, u32, u32)>, resumed: bool, ended: stream::Ended) {
    let open = note::load(root, path).map(|i| i.open_questions).unwrap_or(false);
    let fin = stream::finish(&ended, open);
    let cost = ended.result.as_ref().and_then(|r| r.0).map(|c| format!("{:.4}", stream::next_cost(prev_cost, c, cli, resumed)));
    let now = crate::notes::now_local();
    let mut pairs = vec![("status", Some(fin.status))];
    if let Some(c) = cost.as_deref() {
        pairs.push(("cost", Some(c)));
    }
    if let Err(e) = note::meta(root, path, &pairs, &now) {
        log(&format!("brief: zápis stavu selhal: {e}"));
    }
    if let Some(l) = &fin.log
        && let Err(e) = note::log(root, path, l, &now)
    {
        log(&format!("brief: zápis logu selhal: {e}"));
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    log(&format!("brief: konec {name} → {} (náklad {})", fin.status, cost.as_deref().unwrap_or("?")));
    changed();
}
