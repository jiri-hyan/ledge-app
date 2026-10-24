//! Tauri příkazy widgetů na ploše (`desk/`): ze šuplíku připnout / odepnout / volby / profily / schovat vše;
//! z okna widgetu (label `desk-…`, ověřený z volajícího okna — jiné okno se za widget nevydává) start,
//! volby, posun, odepnutí a „otevřít v šuplíku“. Widget sám nic neschvaluje: schválení a otázky Clauda
//! jen otevřou šuplík (náhled se schválením), rozhoduje dál člověk v šuplíku.

use crate::desk::{self, Kind, Patch, live, win};
use crate::drawer::{self, Opener, View};
use crate::i18n::tr;
use crate::state::{UiCmd, core};
use tauri::{Emitter, WebviewWindow};

/// Id widgetu z labelu volajícího okna (jiné okno → chyba).
fn desk_id(w: &WebviewWindow) -> Result<String, String> {
    desk::id_of(w.label()).map(String::from).ok_or_else(|| tr("Tohle umí jen okno widgetu.", "Only a widget window can do this.").into())
}

/// Šuplík / paleta: připnout druh na plochu (`limits` · `waiting` · `dev` · `agenda` · `widget:<soubor>`).
/// Už připnutý se jen ukáže. Vrací id widgetu.
#[tauri::command(async)]
pub(crate) fn desk_pin(kind: String) -> Result<String, String> {
    let k = Kind::parse(&kind).ok_or_else(|| tr("Tohle na plochu připnout nejde.", "This can't be pinned to the desktop."))?;
    win::pin(&k)
}

/// Šuplík (Nastavení) nebo okno widgetu: odepnout. Z okna widgetu se `id` nebere — rozhoduje label.
#[tauri::command(async)]
pub(crate) fn desk_unpin(window: WebviewWindow, id: Option<String>) -> Result<(), String> {
    let id = match desk_id(&window) {
        Ok(own) => own,
        Err(e) => id.filter(|i| desk::valid_id(i)).ok_or(e)?,
    };
    win::close(&id);
    Ok(())
}

/// Co okno widgetu potřebuje při startu stránky.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeskInit {
    widget: desk::DeskWidget,
    lang: crate::uiconf::Lang,
    data: live::DeskData,
}

#[tauri::command(async)]
pub(crate) fn desk_init(window: WebviewWindow) -> Result<DeskInit, String> {
    let id = desk_id(&window)?;
    let w = win::widget(&id).ok_or_else(|| tr("Widget už není na ploše.", "The widget is no longer on the desktop."))?;
    let kind = w.kind().ok_or_else(|| tr("Neznámý widget.", "Unknown widget."))?;
    Ok(DeskInit { data: live::data_for(&kind), widget: w, lang: core().cfg.read().ui.lang })
}

/// Volby widgetu: z okna widgetu (label), ze šuplíku podle `id`.
#[tauri::command(async)]
pub(crate) fn desk_set(window: WebviewWindow, id: Option<String>, patch: Patch) -> Result<desk::DeskWidget, String> {
    let id = match desk_id(&window) {
        Ok(own) => own,
        Err(e) => id.filter(|i| desk::valid_id(i)).ok_or(e)?,
    };
    win::set(&id, &patch)
}

/// Okno widgetu: tah za lištu = posun (zamčený / průchozí se nehne).
#[tauri::command]
pub(crate) fn desk_drag(window: WebviewWindow) -> Result<(), String> {
    let id = desk_id(&window)?;
    if win::widget(&id).is_some_and(|w| !desk::movable(&w)) {
        return Ok(());
    }
    window.start_dragging().map_err(|e| crate::errors::any(tr("Widget nejde přesunout", "Can't move the widget"), &e))
}

/// Okno widgetu: volby (⋯) otevřené / zavřené — okno se na tu dobu zvětší, ať se volby vejdou.
#[tauri::command(async)]
pub(crate) fn desk_menu(window: WebviewWindow, open: bool) -> Result<(), String> {
    win::menu(&desk_id(&window)?, open);
    Ok(())
}

/// Okno widgetu: otevřít šuplík (výslovný klik). `waiting` = náhled se schválením / otázkou Clauda (rozhoduje
/// se tam), `settings` = Nastavení, jinak celý šuplík.
#[tauri::command(async)]
pub(crate) fn desk_open_drawer(window: WebviewWindow, target: String) -> Result<(), String> {
    desk_id(&window)?;
    let c = core();
    match target.as_str() {
        "waiting" => c.send_ui(UiCmd::Show(View::Peek, Opener::Hotkey)),
        "settings" => {
            c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey));
            let _ = c.app.emit_to(drawer::LABEL, "page", "settings");
        }
        _ => c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey)),
    }
    Ok(())
}

/// Nastavení / paleta: uložit dnešní rozmístění jako profil (stejné jméno = přepsat).
#[tauri::command(async)]
pub(crate) fn desk_profile_save(name: String) -> Result<String, String> {
    win::profile_save(&name)
}

/// Nastavení / paleta: obnovit profil jedním klikem.
#[tauri::command(async)]
pub(crate) fn desk_profile_apply(name: String) -> Result<(), String> {
    win::profile_apply(&name)
}

#[tauri::command(async)]
pub(crate) fn desk_profile_delete(name: String) -> bool {
    win::profile_delete(&name)
}

/// Paleta / Nastavení: schovat / ukázat všechny widgety (`None` = přepnout). Vrací „jsou schované“.
#[tauri::command(async)]
pub(crate) fn desk_hide_all(hide: Option<bool>) -> bool {
    win::hide_all(hide)
}
