//! Tauri příkazy šuplíku a okna: stav, režim, velikost, potvrzení animací, fokus (jen na
//! výslovný klik), odkazy (http, rychlé odkazy Osobní), stránka po studeném startu.

use crate::drawer::{self, View};
use crate::personal;
use crate::refresh::maybe_refresh_weather;
use crate::snapshot::{Snapshot, snapshot};
use crate::state::{UiCmd, core, log, mark_dirty};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub(crate) fn get_state() -> Snapshot {
    snapshot()
}

#[tauri::command]
pub(crate) fn set_mode(mode: String) {
    // režim = id záložky z `layout`; neznámé (smazaná záložka) = první
    let id = crate::layout::page_id(&core().cfg.read().layout, &mode);
    *core().mode.lock() = id;
    maybe_refresh_weather(false);
    crate::calendar::live::maybe_refresh(false);
    mark_dirty();
}

/// Herní režim (Nastavení, paleta): zapnutí zavře šuplík; vypíná se zkratkou / dalším spuštěním (`gamemode.rs`).
#[tauri::command]
pub(crate) fn set_game_mode(on: bool) {
    crate::gamemode::set(on);
}

#[tauri::command]
pub(crate) fn report_size(view: View, w: f64, h: f64) {
    core().send_ui(UiCmd::Size(view, w, h));
}

#[tauri::command]
pub(crate) fn ui_ready() {
    core().drawer.ack("ready");
    log("drawer: ui ready");
}

/// Potvrzení fází morfu z frontendu ("prepared" / "closed").
#[tauri::command]
pub(crate) fn anim_ack(kind: String) {
    core().drawer.ack(&kind);
}

/// Naměřená plynulost poslední animace (requestAnimationFrame) — do /status.
#[tauri::command]
pub(crate) fn anim_stats(stats: drawer::AnimStats) {
    core().drawer.set_anim_stats(stats);
}

/// Výslovné zavření z UI (Esc, ✕, uložení poznámky, paleta „Zavřít“) → `UiCmd::Close`: fokus se vrátí
/// oknu, odkud si ho Ledge vzal (`focusreturn`). Spouštění, která otevírají okno, posílají `Hide`.
#[tauri::command]
pub(crate) fn close_drawer() {
    core().send_ui(UiCmd::Close);
}

/// Klik uživatele na vstup do poznámky (dlaždice, hlavička, „+ Nová“): šuplík otevřený najetím je
/// NOACTIVATE, takže by do textarea nešlo psát. Výslovný klik = úmysl psát → aktivovat okno a
/// přepnout opener na `Hotkey`, aby ho blur zavřel jako po zkratce. Z automatických cest (peek,
/// připomínka, hover) se NEVOLÁ — Ledge nekrade fokus.
#[tauri::command]
pub(crate) fn focus_drawer() {
    core().send_ui(UiCmd::Focus);
}

/// Odchod ze stránky poznámky zpět do šuplíku: opener na hodnotu před `focus_drawer`.
#[tauri::command]
pub(crate) fn release_focus() {
    core().send_ui(UiCmd::Release);
}

#[tauri::command]
pub(crate) fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(crate::i18n::tr("Jen http(s) odkazy.", "Only http(s) links.").into());
    }
    core().app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Fokus odešel do vyskakovacího okna vlastněného šuplíkem (kalendář `datetime-local`, `<select>`)?
/// Pak blur šuplík nezavře. Neověřeno naživo — WebView2 popupy nemusí vůbec brát aktivaci.
pub(crate) fn focus_in_own_popup() -> bool {
    use tauri::Manager;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GA_ROOTOWNER, GetAncestor, GetForegroundWindow};
    let Some(w) = core().app.get_webview_window(drawer::LABEL) else { return false };
    let Ok(h) = w.hwnd() else { return false };
    let ours = h.0 as isize;
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || fg.0 as isize == ours {
            return false;
        }
        let owner: HWND = GetAncestor(fg, GA_ROOTOWNER);
        owner.0 as isize == ours
    }
}

/// Otevře rychlý odkaz záložky Osobní: `index` do `personal.links` + `target`, který UI ukazovalo —
/// když se config mezitím změnil, odmítne (neotevře jiný cíl, než na který šéf klikl). Cíl se
/// znovu ověří na disku (web jen http(s), cesta existuje, allowlist přípon, `.exe` bez argumentů).
#[tauri::command(async)]
pub(crate) fn open_link(index: usize, target: String) -> Result<(), String> {
    let c = core();
    let link = c
        .cfg
        .read()
        .personal
        .links
        .get(index)
        .filter(|l| l.target == target)
        .cloned()
        .ok_or_else(|| crate::i18n::tr("Odkazy se mezitím změnily — zkus to znovu.", "The links changed meanwhile — try again."))?;
    let opener = c.app.opener();
    let checked = personal::classify_fs(&link.target);
    {
        // ověření mohlo dopadnout jinak než cache (složka zmizela / přibyla) → opravit kartu
        let mut cache = c.links.lock();
        if cache.1.get(&link.target) != Some(&checked) {
            cache.1.insert(link.target.clone(), checked.clone());
            drop(cache);
            mark_dirty();
        }
    }
    match checked {
        personal::LinkTarget::Web(url) => opener.open_url(url, None::<&str>),
        personal::LinkTarget::Folder(p) | personal::LinkTarget::File(p) | personal::LinkTarget::App(p) => {
            opener.open_path(p.to_string_lossy(), None::<&str>)
        }
        personal::LinkTarget::Invalid(why) => return Err(format!("{}: {why}", link.name)),
    }
    .map_err(|e| e.to_string())?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// Stránka, na kterou se šuplík otevírá — pro studený start WebView, kdy událost `page` nedošla.
#[tauri::command]
pub(crate) fn take_page() -> Option<String> {
    core().pending_page.lock().take().map(String::from)
}
