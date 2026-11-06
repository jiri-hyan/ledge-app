//! Tauri příkazy Nastavení: čtení a uložení configu, otevření souboru configu.

use crate::config::{self, Config};
use crate::settings::apply_config;
use crate::state::core;
use std::os::windows::process::CommandExt;

#[tauri::command]
pub(crate) fn get_config() -> Config {
    core().cfg.read().clone()
}

/// Zápis z okna (šuplík / Studio): druhé okno dostane `config-changed`, ať se přenačte nebo ukáže konflikt.
/// Samo volající okno ne (vlastní zápis by si přebíralo jako cizí).
#[tauri::command]
pub(crate) fn save_config(window: tauri::WebviewWindow, cfg: Config) -> Result<(), String> {
    save_config_inner(cfg)?;
    let other = if window.label() == crate::studio::LABEL { Some(crate::drawer::LABEL) } else if crate::studio::window().is_some() { Some(crate::studio::LABEL) } else { None };
    if let Some(l) = other {
        use tauri::Emitter;
        let _ = core().app.emit_to(l, crate::mcp::host::EVENT, serde_json::json!({ "source": "window" }));
    }
    Ok(())
}

/// Zápis bez oznámení oknům (MCP si oznámí samo přes `notify_ui`).
pub(crate) fn save_config_inner(mut cfg: Config) -> Result<(), String> {
    // import staré zálohy nebo starý draft: odkazy z `personal.links` jako akce (jako při načtení)
    crate::linkmigrate::migrate(&mut cfg);
    config::save(&config::config_path(), &cfg)?;
    apply_config(cfg);
    Ok(())
}

#[tauri::command]
pub(crate) fn open_config_file() -> Result<(), String> {
    let path = config::config_path();
    std::process::Command::new("cmd")
        .raw_arg(format!("/d /c start \"\" \"{}\"", path.display()))
        .creation_flags(0x0800_0000)
        .spawn()
        .map(|_| ())
        .map_err(|e| crate::errors::spawn(crate::i18n::tr("Nastavení nejde otevřít", "Can't open the settings"), &e))
}

/// Obnoví config z kopie `config.json.broken-*`, kterou Ledge udělal při posledním zápisu (cestu zná jen
/// jádro, UI žádnou nepředává). Současný soubor zůstane v rotaci `.1`; kopie se nemaže.
#[tauri::command]
pub(crate) fn config_restore_backup() -> Result<(), String> {
    let bak = config::recovery::backup().ok_or_else(|| config::recovery::no_backup().to_string())?;
    let part = config::restore_backup(&config::config_path(), &bak)?;
    apply_config(part.cfg.clone());
    // po apply (nemění stav chyb): sekce, které ani kopie nezvládla, hned ukázat
    config::recovery::set_load(None, part.bad, config::recovery::range_warnings(&part.cfg));
    config::recovery::set_backup(None);
    crate::state::mark_dirty();
    Ok(())
}

/// „Zavřít“ u hlášky o kopii: kopie na disku zůstává.
#[tauri::command]
pub(crate) fn config_backup_dismiss() {
    config::recovery::set_backup(None);
    crate::state::mark_dirty();
}

/// Ukáže kopii v Průzkumníku.
#[tauri::command(async)]
pub(crate) fn config_backup_reveal() -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let bak = config::recovery::backup().ok_or_else(|| config::recovery::no_backup().to_string())?;
    core().app.opener().reveal_item_in_dir(&bak).map_err(|e| crate::errors::any(&bak.display().to_string(), &e))
}
