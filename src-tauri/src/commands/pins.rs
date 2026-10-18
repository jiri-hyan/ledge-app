//! Tauri příkazy lístků na plochu (`pins/`): z šuplíku připnout / odepnout / nový lístek / průchozí
//! u všech; z okna lístku (label `note-…`, ověřený z volajícího okna — jiné okno se za lístek
//! nevydává) start, volby, posun, odepnutí. Obsah poznámky jde přes příkazy poznámek (`note_read`,
//! `note_save`, `note_toggle_task`) — žádný druhý zápis.

use crate::i18n::tr;
use crate::notes;
use crate::pins::{self, PinPatch, PinPrefs, win};
use crate::state::{core, mark_dirty};
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{notes_root, reload_notes};
use tauri::WebviewWindow;

fn off() -> &'static str {
    tr("Poznámky jsou vypnuté v Nastavení.", "Notes are off in Settings.")
}

/// Label volajícího okna lístku (jiné okno → chyba).
fn pin_label(w: &WebviewWindow) -> Result<String, String> {
    let l = w.label();
    if l.starts_with(pins::LABEL_PREFIX) { Ok(l.to_string()) } else { Err(tr("Tohle umí jen okno lístku.", "Only a sticky window can do this.").into()) }
}

/// Šuplík: připnout poznámku na plochu, nebo ji odepnout. Vrací true = je připnutá.
#[tauri::command(async)]
pub(crate) fn pin_toggle(path: String) -> Result<bool, String> {
    if win::is_open(&path) {
        win::close(&path);
        return Ok(false);
    }
    win::open(&path, false)?;
    Ok(true)
}

/// Šuplík / paleta: nový lístek = nová poznámka (obecná, s datem v titulku) hned na ploše s fokusem.
#[tauri::command(async)]
pub(crate) fn pin_new() -> Result<String, String> {
    let root = notes_root().ok_or_else(off)?;
    let now = notes::now_local();
    let text = trf!("Lístek {}", "Sticky {}", now.format("%-d. %-m. %H:%M"));
    let path = notes::create(&root, &notes::NewNote { text, ..Default::default() }, &now)?;
    reload_notes();
    send_sync(SyncMsg::Local);
    let p = path.to_string_lossy().into_owned();
    win::open(&p, true)?;
    Ok(p)
}

/// Paleta / šuplík: průchozí režim u všech lístků přepnout. `None` = žádný lístek není na ploše.
#[tauri::command(async)]
pub(crate) fn pin_through_all() -> Option<bool> {
    win::toggle_through_all()
}

/// Co lístek potřebuje při startu stránky.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PinInit {
    path: String,
    prefs: PinPrefs,
    /// zkratka průchozího režimu a jestli se ji podařilo zaregistrovat (obsazená jinou aplikací?)
    key: String,
    key_ok: bool,
    /// nápověda „jak z průchozího ven“ už se ukázala
    hint_seen: bool,
    lang: crate::uiconf::Lang,
    /// právě vytvořený „Nový lístek“ → UI hned otevře „+ bod“
    fresh: bool,
}

#[tauri::command(async)]
pub(crate) fn pin_init(window: WebviewWindow) -> Result<PinInit, String> {
    let label = pin_label(&window)?;
    let path = win::path_of(&label).ok_or_else(|| tr("Lístek už není připnutý.", "The sticky is no longer pinned."))?;
    let prefs = win::prefs_of(&label).unwrap_or_default();
    let cfg = core().cfg.read().clone();
    let key = cfg.hotkeys.note_passthrough.trim().to_string();
    let key_ok = !key.is_empty() && crate::hotkeys::is_registered(&key);
    Ok(PinInit { path, prefs, key, key_ok, hint_seen: win::hint_seen(), lang: cfg.ui.lang, fresh: win::take_fresh(&label) })
}

/// Okno lístku: změna voleb (navrchu, zámek, průchozí, průhlednosti, sbalení).
#[tauri::command(async)]
pub(crate) fn pin_set(window: WebviewWindow, patch: PinPatch) -> Result<PinPrefs, String> {
    win::set(&pin_label(&window)?, &patch)
}

/// Okno lístku: tah za lištu = posun okna (zamčený lístek se nehne).
#[tauri::command]
pub(crate) fn pin_drag(window: WebviewWindow) -> Result<(), String> {
    let label = pin_label(&window)?;
    if win::prefs_of(&label).is_some_and(|p| p.locked || p.through) {
        return Ok(());
    }
    window.start_dragging().map_err(|e| e.to_string())
}

/// Okno lístku: ✕ = odepnout (poznámka zůstane v kartě Poznámky).
#[tauri::command(async)]
pub(crate) fn pin_unpin(window: WebviewWindow) -> Result<(), String> {
    let label = pin_label(&window)?;
    if let Some(path) = win::path_of(&label) {
        win::close(&path);
    } else {
        let _ = window.destroy();
    }
    mark_dirty();
    Ok(())
}

/// Okno lístku: nápověda k průchozímu režimu se ukázala (příště už ne).
#[tauri::command(async)]
pub(crate) fn pin_hint_seen(window: WebviewWindow) -> Result<(), String> {
    pin_label(&window)?;
    win::set_hint_seen();
    Ok(())
}
