//! Tauri příkazy Script Commands a stránek Nastavení Windows: seznam skriptů (`scripts_list` —
//! sken při otevření palety / panelu), spuštění (`run_script` — soubor se znovu přečte a zahashuje,
//! první spuštění a změna obsahu = potvrzení, které se uloží jako souhlas s tímhle obsahem),
//! „Otevřít složku skriptů“ (`scripts_open_dir`, výslovný klik — založí ji) a `win_settings_open`.
//! Logika v `scripts/*`, `winsettings.rs`, spouštění přes `actionrun`.

use crate::actionrun::{self, CONSENTS, Outcome, RunGuard};
use crate::i18n::tr;
use crate::scripts::{self, Listing, meta::Mode, run};
use crate::state::{UiCmd, core, log, mark_dirty};
use crate::winsettings::{self, WinTarget};
use tauri_plugin_opener::OpenerExt;

/// Skripty ve složce (stat + jen změněné soubory). Paleta volá při otevření.
#[tauri::command(async)]
pub(crate) fn scripts_list() -> Listing {
    let cfg = core().cfg.read().scripts.clone();
    scripts::refresh(&cfg)
}

/// Spustí skript `id`. Potřebuje-li potvrzení (poprvé, po změně souboru, `@ledge.confirm`),
/// `confirmed` musí být přesně text, který člověk viděl — obsahuje otisk obsahu, takže potvrzení
/// starší verze souboru neplatí. Potvrzení bez `@ledge.confirm` se zapamatuje (`Consents`).
#[tauri::command(async)]
pub(crate) fn run_script(id: String, confirmed: Option<String>) -> Result<Outcome, String> {
    let c = core();
    let cfg = c.cfg.read().scripts.clone();
    let (s, interp) = scripts::fresh(&cfg, &id)?;
    let meta = s.meta.clone()?;
    let fp = s.fingerprint().ok_or_else(|| tr("Skript nejde ověřit.", "Can't verify the script."))?;
    let consented = CONSENTS.lock().has_fp(&fp);
    if (meta.confirm || !consented) && confirmed.as_deref() != Some(s.confirm_text().as_str()) {
        scripts::refresh_async();
        return Err(trf!("{}: skript se mezitím změnil nebo nebyl potvrzen — zkus to znovu.", "{}: the script changed meanwhile or wasn't confirmed — try again.", meta.title));
    }
    if !consented {
        CONSENTS.lock().grant_fp(fp)?;
    }
    let dir = s.path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
    let mut cmd = run::plan(&s.path, &interp, &meta.args, meta.mode == Mode::Terminal)?.command(&dir);
    let res = match meta.mode {
        Mode::Terminal => actionrun::spawn_console(&mut cmd, &meta.title),
        mode => {
            let _guard = RunGuard::take(&s.id).ok_or_else(|| trf!("{} už běží.", "{} is already running.", meta.title))?;
            mark_dirty();
            actionrun::capture(&mut cmd, &meta.title, meta.timeout_s, mode == Mode::Output)
        }
    };
    log(&format!("skript {}: {}", s.file, match &res {
        Ok(o) => o.message.clone(),
        Err(e) => e.lines().next().unwrap_or("").to_string(),
    }));
    if res.as_ref().is_ok_and(|o| o.hide) {
        c.send_ui(UiCmd::Hide);
    }
    if cfg.in_actions {
        scripts::refresh_async(); // souhlas / běží → dlaždice
    }
    res
}

/// „Otevřít složku skriptů“ (Nastavení): založí ji (výslovný klik) a otevře v Průzkumníkovi.
#[tauri::command(async)]
pub(crate) fn scripts_open_dir() -> Result<String, String> {
    let c = core();
    let cfg = c.cfg.read().scripts.clone();
    let dir = scripts::resolve_dir(&cfg)?;
    std::fs::create_dir_all(&dir).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", dir.display()), &e))?;
    c.app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| crate::errors::any(tr("Složku nejde otevřít", "Can't open the folder"), &e))?;
    scripts::refresh_async();
    Ok(dir.display().to_string())
}

/// Položka palety „Nastavení Windows: …“ — jen ověřený tvar cíle (`winsettings::parse`).
#[tauri::command]
pub(crate) fn win_settings_open(target: String) -> Result<(), String> {
    let c = core();
    match winsettings::parse(&target)? {
        WinTarget::Settings(uri) => c.app.opener().open_url(uri, None::<&str>).map_err(|e| crate::errors::any(tr("Nastavení Windows nejde otevřít", "Can't open Windows Settings"), &e))?,
        WinTarget::Control(name) => {
            std::process::Command::new(winsettings::control_exe()).args(["/name", &name]).spawn().map_err(|e| crate::errors::spawn(tr("Ovládací panel nejde otevřít", "Can't open the Control Panel"), &e))?;
        }
    }
    log(&format!("nastavení windows: {target}"));
    c.send_ui(UiCmd::Hide);
    Ok(())
}
