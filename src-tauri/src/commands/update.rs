//! Tauri příkazy aktualizace: kontrola na klik, instalace a návrat k předchozí verzi (s potvrzením
//! běžících serverů), „Zkusit znovu“ po selhaném tichém stažení, přečtení „Co je nového“.

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
    let exe = install_target()?;
    let what = c.updater.install(&exe)?;
    log(&format!("update: instaluji {what}"));
    exit_soon();
    Ok(InstallOutcome::Started)
}

/// „Zkusit znovu“ po selhaném tichém stažení — průběh i výsledek přijdou ve snapshotu (`update.fetch`).
#[tauri::command]
pub(crate) fn update_retry() {
    core().updater.retry_fetch();
}

/// Upozornění „Co je nového“ přečteno (klik na „Rozumím“ / zavření).
#[tauri::command]
pub(crate) fn update_notes_read() -> Result<(), String> {
    core().updater.mark_notes_read()
}

/// „Vrátit předchozí verzi“ (UI se ptá předem). Stejné potvrzení běžících serverů jako instalace; pak
/// spustí ověřený instalátor předchozí verze (z cache, jinak stažený z vydání podle tagu) a Ledge ukončí.
#[tauri::command(async)]
pub(crate) fn update_rollback(confirmed: Vec<String>) -> Result<InstallOutcome, String> {
    let running = running_names();
    if running.iter().any(|n| !confirmed.contains(n)) {
        return Ok(InstallOutcome::Confirm { running });
    }
    let exe = install_target()?;
    let what = core().updater.rollback(&exe)?;
    log(&format!("update: {what}"));
    exit_soon();
    Ok(InstallOutcome::Started)
}

fn install_target() -> Result<std::path::PathBuf, String> {
    autostart::install_target().ok_or_else(|| crate::i18n::tr("Nevím, kam se Ledge instaluje (chybí %LOCALAPPDATA%).", "I don't know where Ledge installs to (%LOCALAPPDATA% is missing).").into())
}

/// Odpověď ať stihne dojít do UI; instalátor čeká ~2 s, než začne.
fn exit_soon() {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
        core().app.exit(0);
    });
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
