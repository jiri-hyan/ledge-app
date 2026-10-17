//! Tauri příkazy aktualizace: kontrola na klik a instalace (s potvrzením běžících serverů).

use crate::autostart;
use crate::state::{core, log};
use serde::Serialize;
use std::time::Duration;

/// „Zkontrolovat teď“ — výsledek přijde ve snapshotu (`update`).
#[tauri::command]
pub(crate) fn update_check_now() {
    core().updater.check_now();
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub(crate) enum InstallOutcome {
    /// běží spravované procesy — s Ledge (Job Object) by umřely; UI chce druhé potvrzení s tímhle seznamem
    Confirm { running: Vec<String> },
    /// instalátor ověřen a spuštěn, Ledge se za okamžik ukončí
    Started,
}

/// Instalace dostupné verze na klik. `confirmed` = jména běžících serverů, která šéf odsouhlasil
/// zastavit; když běží něco, co v seznamu není (i nově spuštěné mezitím), vrátí `Confirm` a nic nedělá.
/// Pak stáhne + ověří SHA256 + odpojeně spustí instalátor (viz `update.rs`) a Ledge ukončí — servery
/// se zastaví v `RunEvent::Exit` (`stop_all`).
#[tauri::command(async)]
pub(crate) fn update_install(confirmed: Vec<String>) -> Result<InstallOutcome, String> {
    let c = core();
    let running = running_names();
    if running.iter().any(|n| !confirmed.contains(n)) {
        return Ok(InstallOutcome::Confirm { running });
    }
    let exe = autostart::install_target().ok_or_else(|| crate::i18n::tr("Nevím, kam se Ledge instaluje (chybí %LOCALAPPDATA%).", "I don't know where Ledge installs to (%LOCALAPPDATA% is missing)."))?;
    let what = c.updater.install(&exe)?;
    log(&format!("update: instaluji {what}"));
    std::thread::spawn(|| {
        // odpověď ať stihne dojít do UI; instalátor čeká ~2 s, než začne
        std::thread::sleep(Duration::from_millis(400));
        core().app.exit(0);
    });
    Ok(InstallOutcome::Started)
}

/// Jména (z configu, jinak id) spravovaných procesů v živé fázi — ty by aktualizace zastavila.
pub(crate) fn running_names() -> Vec<String> {
    let c = core();
    let cfg = c.cfg.read();
    c.procs
        .snapshot()
        .into_iter()
        .filter(|p| p.phase.is_live())
        .map(|p| cfg.projects.iter().find(|x| x.id == p.id).map(|x| x.name.clone()).unwrap_or(p.id))
        .collect()
}
