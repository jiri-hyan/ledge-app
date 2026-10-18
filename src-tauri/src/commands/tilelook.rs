//! Vlastní obrázek zástupce (`tilelook.rs`): výběr souboru → zmenšený obrázek v cache → klíč do configu
//! (`look.icon = {kind:"image", value:klíč}` uloží UI); data URL podle klíče (UI si ho vyžádá jednou).

use crate::tilelook;
use std::path::Path;

/// Převede vybraný obrázek do cache `looks` (WIC, ≤ 128 px) → klíč. Disk a COM → mimo UI vlákno.
#[tauri::command(async)]
pub(crate) fn look_image_add(path: String) -> Result<String, String> {
    tilelook::import_image(&tilelook::image_dir(), Path::new(path.trim()))
}

/// Data URL obrázku z cache; neznámý klíč = `null` (UI ukáže Auto).
#[tauri::command(async)]
pub(crate) fn look_image(key: String) -> Option<String> {
    tilelook::image_url(&tilelook::image_dir(), &key)
}
