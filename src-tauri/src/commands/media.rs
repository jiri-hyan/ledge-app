//! Záložka Osobní — právě hraje, hlasitost, baterie (`media.rs`, `volume.rs`, `battery.rs`).
//! `personal_live` volá UI při zobrazení záložky Osobní: až tehdy vzniknou odběry událostí
//! (líný start, nic v klidu). Ovládání jen na výslovný klik / tah v kartě.

use crate::{battery, media, volume};

/// Záložka Osobní je vidět: spustit odběry karet, které smí běžet (idempotentní).
#[tauri::command]
pub(crate) fn personal_live() {
    media::ensure();
    volume::ensure();
    battery::ensure();
}

/// Přehrát/pauza, další, předchozí, přetočit (`{"seek": ms}`), náhodně (`{"shuffle": bool}`), opakovat
/// (`{"repeat": "none"|"track"|"list"}`) v aktuální mediální session — jen co zdroj umí.
#[tauri::command]
pub(crate) fn media_control(action: media::Control) -> Result<(), String> {
    media::control(action)
}

/// Obal aktuální skladby jako data URL (UI si ho vyžádá jednou na `coverKey`).
#[tauri::command(async)]
pub(crate) fn media_cover(key: String) -> Option<String> {
    media::cover(&key)
}

/// Hlasitost výchozího výstupu 0–100 (posuvník; UI posílá při tahu omezeně).
#[tauri::command]
pub(crate) fn volume_set(level: u8) -> Result<(), String> {
    volume::set(level)
}

#[tauri::command]
pub(crate) fn volume_mute(muted: bool) -> Result<(), String> {
    volume::mute(muted)
}
