//! Tauri příkazy propojení poznámek se složkou (`notes::folder_sync`, vlákno `notes-folder`):
//! „Synchronizovat teď“, kontrola vybrané složky (kolik poznámek v ní je) a „Rozumím“ u konfliktů.

use crate::i18n::tr;
use crate::notes::folder_sync;
use crate::threads::notes_folder::{self, Msg};
use crate::threads::notes_watch::notes_root;
use std::sync::mpsc;
use std::time::Duration;

/// „Synchronizovat teď“: počká na doběhnutí a vrátí větu o výsledku. Jede podle ULOŽENÉHO nastavení.
#[tauri::command(async)]
pub(crate) fn note_folder_sync_now() -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    notes_folder::send(Msg::Now(tx));
    rx.recv_timeout(Duration::from_secs(300)).map_err(|_| tr("Synchronizace nestihla doběhnout.", "Sync didn't finish in time.").to_string())?
}

/// Po výběru složky: kolik poznámek (`.md`) v ní je (+ varování, když nevypadá jako vault), nebo proč ji propojit nejde.
#[tauri::command(async)]
pub(crate) fn note_folder_check(path: String) -> Result<folder_sync::FolderCheck, String> {
    let root = notes_root().ok_or_else(|| tr("Poznámky jsou vypnuté v Nastavení.", "Notes are off in Settings."))?;
    folder_sync::count_notes(&path, &root)
}

/// „Rozumím“ u konfliktů: seznam se vyprázdní, poznámky a kopie zůstanou.
#[tauri::command(async)]
pub(crate) fn note_folder_clear_conflicts() -> Result<(), String> {
    notes_folder::clear_conflicts()
}
