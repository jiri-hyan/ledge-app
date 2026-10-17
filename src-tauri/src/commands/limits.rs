//! Příkazy Nastavení → Claude Code → „Zapojit stavový řádek“ (limity 5h / týden). Logika
//! v `statuslinesetup.rs`, wrapper v `statusline.rs`, stav za běhu v `limitslive.rs`.

use crate::state::log;
use crate::statuslinesetup::{self, StatusLineStatus};
use std::path::PathBuf;

fn settings() -> Result<PathBuf, String> {
    crate::hooksetup::settings_path().ok_or_else(|| crate::i18n::tr("Nevím, kde je složka .claude.", "I don't know where the .claude folder is.").into())
}

#[tauri::command]
pub(crate) fn statusline_status() -> Result<StatusLineStatus, String> {
    Ok(statuslinesetup::status_at(&settings()?, &statuslinesetup::store_path()))
}

/// Zapojí wrapper Ledge (jen na výslovný klik). Cesta = právě běžící exe (umí `--statusline`).
#[tauri::command]
pub(crate) fn statusline_install() -> Result<StatusLineStatus, String> {
    let path = settings()?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let ours = statuslinesetup::our_command(&exe, crate::statusline::git_bash().is_some());
    statuslinesetup::install_at(&path, &statuslinesetup::store_path(), &ours)?;
    log(&format!("stavový řádek Ledge zapojen do {} ({ours})", path.display()));
    crate::limitslive::invalidate();
    statusline_status()
}

#[tauri::command]
pub(crate) fn statusline_remove() -> Result<StatusLineStatus, String> {
    let path = settings()?;
    statuslinesetup::remove_at(&path, &statuslinesetup::store_path())?;
    log(&format!("stavový řádek Ledge odpojen z {}", path.display()));
    crate::limitslive::invalidate();
    statusline_status()
}
