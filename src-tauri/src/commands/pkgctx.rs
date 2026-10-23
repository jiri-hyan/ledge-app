//! Tauri příkazy bubliny MSIX (`pkgctx.rs`): stav pro Nastavení a šuplík, „Restartovat mimo“.

use crate::commands::update::{InstallOutcome, running_names};
use crate::i18n::tr;
use crate::outside::{self, After};
use crate::pkgctx;
use crate::state::{core, log};
use std::sync::mpsc;
use std::time::Duration;

/// Balíček, ve kterém Ledge běží (nebo `package: null`), a kde fyzicky leží config z bubliny.
#[tauri::command]
pub(crate) fn pkg_context() -> pkgctx::View {
    pkgctx::view()
}

/// „Restartovat mimo“: spustí Ledge mimo bublinu a tuhle instanci ukončí. `confirmed` = běžící servery,
/// které šéf odsouhlasil zastavit (Job Object — umřou s Ledge); jinak vrátí `Confirm` a nic nedělá.
/// Plánovač úloh: nová instance dostane `--after-pid <tento PID>` a počká, až tahle skončí. Záloha
/// Explorer (bez argumentů): nejdřív uvolní zámek single-instance, aby nová instance nepředala
/// spuštění téhle (končící) a neskončila.
#[tauri::command(async)]
pub(crate) fn relaunch_outside(confirmed: Vec<String>) -> Result<InstallOutcome, String> {
    if pkgctx::current().is_none() {
        return Err(tr("Ledge už běží mimo balíček — není co restartovat.", "Ledge already runs outside the package — nothing to restart.").into());
    }
    let running = running_names();
    if running.iter().any(|n| !confirmed.contains(n)) {
        return Ok(InstallOutcome::Confirm { running });
    }
    let exe = std::env::current_exe().map_err(|e| crate::errors::io(tr("Restart nejde provést", "Can't restart"), &e))?;
    let target = pkgctx::relaunch_target(&exe, pkgctx::twin_of(&exe), |p| p.is_file());
    let dir = target.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let args = format!("{} {}", pkgctx::AFTER_PID, std::process::id());
    let via = match outside::run_task(outside::TASK_RELAUNCH, &target, &args, &dir, After::Delete) {
        Ok(()) => "Plánovač úloh",
        Err(e) => {
            log(&format!("relaunch: Plánovač úloh selhal ({e}), zkouším Explorer"));
            release_single_instance();
            outside::run_explorer(&target).map_err(|x| {
                log(&format!("relaunch: Explorer selhal ({x})"));
                tr("Restart mimo balíček se nepovedl. Spusť Ledge z nabídky Start.", "Restarting outside the package failed. Start Ledge from the Start menu.").to_string()
            })?;
            "Explorer"
        }
    };
    log(&format!("relaunch: {} mimo balíček přes {via}, končím", target.display()));
    std::thread::spawn(|| {
        // odpověď ať stihne dojít do UI; nová instance čeká na konec téhle (`--after-pid`)
        std::thread::sleep(Duration::from_millis(400));
        core().app.exit(0);
    });
    Ok(InstallOutcome::Started)
}

/// Zámek single-instance (mutex + okno) pustit na hlavním vlákně (okno patří jemu); počká ≤ 2 s.
fn release_single_instance() {
    let app = core().app.clone();
    let (tx, rx) = mpsc::channel();
    let h = app.clone();
    if app
        .run_on_main_thread(move || {
            tauri_plugin_single_instance::destroy(&h);
            let _ = tx.send(());
        })
        .is_ok()
    {
        let _ = rx.recv_timeout(Duration::from_secs(2));
    }
}
