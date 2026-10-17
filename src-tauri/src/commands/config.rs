//! Tauri příkazy Nastavení: čtení a uložení configu, otevření souboru configu.

use crate::config::{self, Config};
use crate::settings::apply_config;
use crate::state::core;
use std::os::windows::process::CommandExt;

#[tauri::command]
pub(crate) fn get_config() -> Config {
    core().cfg.read().clone()
}

#[tauri::command]
pub(crate) fn save_config(cfg: Config) -> Result<(), String> {
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
        .map_err(|e| e.to_string())
}
