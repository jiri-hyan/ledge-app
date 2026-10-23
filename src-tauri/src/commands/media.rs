//! Záložka Osobní — právě hraje, hlasitost, baterie (`media.rs`, `volume.rs`, `battery.rs`).
//! `personal_live` volá UI při zobrazení záložky Osobní: až tehdy vzniknou odběry událostí
//! (líný start, nic v klidu). Ovládání jen na výslovný klik / tah v kartě.

use crate::{battery, media, mediaconf, volume};
use tauri_plugin_opener::OpenerExt;

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

/// Klik na oblíbenou položku karty Právě hraje: otevře její odkaz (Spotify `spotify:…` / prohlížeč
/// `http(s)://…`) stejným openerem jako ostatní odkazy. `index` + `uri` musí sedět s configem
/// (jinak se změnil a neotevře se jiný cíl, než na který člověk klikl); tvar odkazu se ověří znovu.
/// Žádný nový ShellExecute, žádný fokus pro Ledge — přehrávání pak ovládá GSMTC jako dosud.
#[tauri::command(async)]
pub(crate) fn media_favorite_open(index: usize, uri: String) -> Result<(), String> {
    let c = crate::state::core();
    let fav = c
        .cfg
        .read()
        .media
        .favorites
        .get(index)
        .filter(|f| f.uri == uri)
        .cloned()
        .ok_or_else(|| crate::i18n::tr("Oblíbené se mezitím změnily — zkus to znovu.", "The favorites changed meanwhile — try again."))?;
    if mediaconf::source_of(&fav.uri).is_none() {
        return Err(crate::i18n::tr("Tenhle odkaz Ledge neotevře.", "Ledge won't open this link.").into());
    }
    c.app.opener().open_url(fav.uri, None::<&str>).map_err(|e| crate::errors::any(crate::i18n::tr("Odkaz nejde otevřít", "Can't open the link"), &e))
}

/// Ztlumit / zapnout výchozí mikrofon (výslovný klik v kartě Zvuk). Vrací nový stav.
#[tauri::command(async)]
pub(crate) fn volume_mic_mute() -> Result<bool, String> {
    volume::mic_mute_toggle()
}

/// Přepnout výchozí výstup na zařízení ze seznamu `VolumeView.devices` (výslovný klik).
#[tauri::command(async)]
pub(crate) fn volume_set_device(id: String) -> Result<(), String> {
    volume::set_default_output(&id)
}
