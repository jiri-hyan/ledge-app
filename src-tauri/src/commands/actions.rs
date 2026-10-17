//! Tauri příkazy rychlých akcí: spuštění (`run_action` — znovu ověří cíl a potvrzení), ikona
//! (`action_icon`, `action_icon_refresh`), souhlas s příkazem z Nastavení (`action_consent`),
//! kontrola rozepsané akce (`action_check`) a výběr cesty (`pick_path`). Logika v `actions.rs`,
//! `actionrun.rs`, `actionstate.rs`. Výběr cesty = `commands::settings::pick_path` (dialog plugin).

use crate::i18n::tr;
use crate::actionrun::{self, CONSENTS, Launch, Outcome, RunGuard};
use crate::actions::{self, Action};
use crate::state::{UiCmd, core, log, mark_dirty};
use crate::actionstate;
use serde::Serialize;
use tauri_plugin_opener::OpenerExt;

/// Spustí akci `id`. `target` = cíl, který UI ukazovalo (config se mohl mezitím změnit → odmítne).
/// Potřebuje-li akce potvrzení, `confirmed` musí být přesně text, který člověk v potvrzení viděl.
#[tauri::command(async)]
pub(crate) fn run_action(id: String, target: String, confirmed: Option<String>) -> Result<Outcome, String> {
    let c = core();
    let a = c.cfg.read().actions.iter().find(|a| a.id == id && a.target == target).cloned().ok_or_else(|| tr("Akce se mezitím změnila — zkus to znovu.", "The action changed meanwhile — try again."))?;
    let consented = a.risky() && CONSENTS.lock().has(&a);
    if actions::needs_confirm(&a, consented) && confirmed.as_deref() != Some(actions::confirm_text(&a).as_str()) {
        return Err(trf!("{}: akce se mezitím změnila nebo nebyla potvrzena — zkus to znovu.", "{}: the action changed meanwhile or wasn't confirmed — try again.", a.label));
    }
    let launch = actionrun::resolve(&a, actionrun::check_fs(&a)).map_err(|e| format!("{}: {e}", a.label))?;
    let opener = c.app.opener();
    let opened = |r: Result<(), tauri_plugin_opener::Error>| {
        r.map(|_| Outcome { message: trf!("{} — otevřeno", "{} — opened", a.label), output: None, hide: true }).map_err(|e| format!("{}: {e}", a.label))
    };
    let res = match launch {
        Launch::Url(u) => opened(opener.open_url(u, None::<&str>)),
        Launch::Panel(u) => crate::webpanel::open(&c.app, &a, &u),
        Launch::Path(p) => opened(opener.open_path(p.to_string_lossy(), None::<&str>)),
        Launch::Exe(p) => actionrun::run_exe(&a, &p),
        Launch::Command if a.terminal => actionrun::run_terminal(&a),
        Launch::Command => {
            let _guard = RunGuard::take(&a.id).ok_or_else(|| trf!("{} už běží.", "{} is already running.", a.label))?;
            mark_dirty();
            actionrun::run_hidden(&a)
        }
    };
    log(&format!("akce {}: {}", a.id, match &res {
        Ok(o) => o.message.clone(),
        Err(e) => e.lines().next().unwrap_or("").to_string(),
    }));
    if res.as_ref().is_ok_and(|o| o.hide) {
        c.send_ui(UiCmd::Hide);
    }
    res
}

/// Data URL ikony akce (UI si ho vyžádá jednou na `iconKey`).
#[tauri::command]
pub(crate) fn action_icon(id: String) -> Option<String> {
    actionstate::icon_url(&id)
}

/// „Obnovit ikonu“: favicon se stáhne znovu, lokální ikony se přepočítají.
#[tauri::command]
pub(crate) fn action_icon_refresh(id: String) {
    actionstate::refresh(Some(&id));
}

/// Výslovný souhlas z Nastavení se spouštěním příkazu bez ptaní. Platí jen pro akci, která je
/// v configu přesně v tomhle tvaru (otisk cíle, argumentů, složky, shellu).
#[tauri::command]
pub(crate) fn action_consent(action: Action) -> Result<(), String> {
    let same = core().cfg.read().actions.iter().any(|a| a.id == action.id && actions::fingerprint(a) == actions::fingerprint(&action));
    if !same {
        return Err(tr("Akce není uložená v tomhle tvaru — nejdřív ji ulož.", "The action isn't saved in this form — save it first.").into());
    }
    CONSENTS.lock().grant(&action)?;
    mark_dirty();
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActionCheck {
    /// config by ji odmítl
    error: Option<String>,
    /// uložit jde, ale spustit zatím ne (cesta chybí, typ nesedí)
    warning: Option<String>,
}

/// Kontrola rozepsané akce v Nastavení: tvar (jako `validate`) + ověření na disku.
#[tauri::command(async)]
pub(crate) fn action_check(action: Action) -> ActionCheck {
    if let Err(e) = actions::validate_one(&action) {
        return ActionCheck { error: Some(e), warning: None };
    }
    let warning = match actionrun::check_fs(&action) {
        Some(checked) => actionrun::resolve(&action, Some(checked)).err(),
        None => None,
    }
    .or_else(|| action.cwd.as_deref().filter(|c| !c.trim().is_empty()).and_then(|_| actionrun::work_dir(&action).err()));
    ActionCheck { error: None, warning }
}
