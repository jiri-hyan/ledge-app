//! Tauri příkazy Agent View: akce nad session na pozadí a spuštění nové (`claude --bg`). Jen na
//! výslovný klik v kartě Claude / dlaždici / paletě. Rust znovu najde řádek a ověří, že akce je
//! mezi nabídnutými (`Row.job.actions`) — UI nemůže poslat jiné id ani akci. Logika v `agentview.rs`,
//! I/O v `agentrun.rs`.

use crate::agentview::{self, JobAction};
use crate::i18n::tr;
use crate::snapshot::session_rows;
use crate::state::{UiCmd, core, find_project, log};
use crate::{agentrun, config, sessionlive, sessions};
use serde::Serialize;
use std::time::Duration;

/// Termín `claude --bg` (start session + předání supervisoru).
const BG_WAIT: Duration = Duration::from_secs(45);
/// Strop výpisu `claude --bg` / výstupu akcí kromě logů.
const SHORT_MAX: usize = 8 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentOut {
    /// výstup (`logs`) nebo krátká zpráva CLI
    pub text: String,
    /// začátek výstupu se nevešel do `agentview::LOG_MAX`
    pub cut: bool,
}

fn find_row(id: &str) -> Result<sessions::Row, String> {
    let c = core();
    let projects: Vec<config::Project> = c.cfg.read().projects.iter().filter(|p| p.enabled).cloned().collect();
    let dirs: Vec<(String, String)> = projects.iter().filter_map(|p| Some((p.id.clone(), c.project_dir(p)?.to_string_lossy().into_owned()))).collect();
    let name_of = |id: &str| projects.iter().find(|p| p.id == id).map(|p| p.name.clone());
    session_rows(&dirs, &name_of).into_iter().find(|r| r.id == id).ok_or_else(|| tr("Session už v seznamu není.", "The session is no longer in the list.").to_string())
}

/// Akce nad session na pozadí. `confirmed` = člověk potvrdil v kartě (Zastavit / Odebrat / Restartovat).
#[tauri::command(async)]
pub(crate) fn agent_action(id: String, action: JobAction, confirmed: bool) -> Result<AgentOut, String> {
    let row = find_row(&id)?;
    let job = row.job.ok_or_else(|| tr("Tahle session neběží na pozadí.", "This session isn't a background session."))?;
    if !job.actions.contains(&action) {
        return Err(tr("Tuhle akci teď u session nejde provést.", "This action isn't available for the session right now.").into());
    }
    if action.needs_confirm() && !confirmed {
        return Err(tr("Akce čeká na potvrzení.", "The action needs confirmation.").into());
    }
    if action == JobAction::Attach {
        agentrun::attach(&row.cwd, &job.id)?;
        core().send_ui(UiCmd::Hide);
        return Ok(AgentOut { text: String::new(), cut: false });
    }
    let args = agentview::cli_args(action, &job.id).ok_or_else(|| tr("Neobvyklé id session.", "Unusual session id."))?;
    let cap = if action == JobAction::Logs { agentview::LOG_MAX } else { SHORT_MAX };
    let out = agentrun::run(&args, None, action.wait(), cap)?;
    if action != JobAction::Logs {
        log(&format!("claude {} {} → {}", args[0], job.id, if out.ok { "ok" } else { "chyba" }));
        // stav se změnil → čerstvý seznam (šuplík je otevřený, klik právě proběhl)
        sessionlive::request(sessionlive::Wake::Open);
    }
    if !out.ok {
        let why = out.first_line();
        return Err(if why.is_empty() { tr("Claude Code akci odmítl.", "Claude Code refused the action.").into() } else { why });
    }
    let text = if action == JobAction::Logs { agentview::clean_output(&out.stdout) } else { out.first_line() };
    Ok(AgentOut { text, cut: out.cut && action == JobAction::Logs })
}

/// Nová session na pozadí v projektu: `claude --bg [--name N] "<úkol>"` ve složce projektu (pole
/// argumentů, žádný shell). Vrací krátkou zprávu pro hlášku.
#[tauri::command(async)]
pub(crate) fn agent_spawn(project_id: String, prompt: String, name: Option<String>) -> Result<String, String> {
    let c = core();
    let p = find_project(&project_id)?;
    let dir = c.project_dir(&p).ok_or_else(|| tr("Složka projektu nenalezena.", "Project folder not found."))?;
    let args = agentview::bg_args(&prompt, name.as_deref())?;
    let out = agentrun::run(&args, Some(&dir), BG_WAIT, SHORT_MAX)?;
    log(&format!("claude --bg v {} → {}", dir.display(), if out.ok { "ok" } else { "chyba" }));
    sessionlive::request(sessionlive::Wake::Open);
    if !out.ok {
        let why = out.first_line();
        return Err(if why.is_empty() { tr("Claude Code session nespustil.", "Claude Code didn't start the session.").into() } else { why });
    }
    let text = agentview::clean_output(&out.stdout);
    Ok(match agentview::backgrounded_id(&text) {
        Some(id) => trf!("Claude běží na pozadí · {id}", "Claude is running in the background · {id}"),
        None => tr("Claude běží na pozadí.", "Claude is running in the background.").into(),
    })
}
