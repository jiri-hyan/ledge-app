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
    // velikost pro každou záložku (`appearance.size.perPage`, kolo 23): jiná velikost = animovaně dorovnat
    let resize = {
        let s = &core().cfg.read().appearance.size;
        s.per_page && s.for_page(&crate::appearance::size::page()) != s.for_page(&id)
    };
    crate::appearance::size::set_page(&id);
    crate::appearance::size::set_live(None);
    *core().mode.lock() = id;
    if resize {
        core().send_ui(UiCmd::Resize(true));
    }
    maybe_refresh_weather(false);
    crate::incidents::maybe_refresh(false);
    crate::limitsfetch::maybe_refresh(false);
    crate::calendar::live::maybe_refresh(false);
    crate::widgets::live::wake_if_used(&core().cfg.read());
    mark_dirty();
}

/// Herní režim (Nastavení, paleta): zapnutí zavře šuplík; vypíná se zkratkou / dalším spuštěním (`gamemode.rs`).
#[tauri::command]
pub(crate) fn set_game_mode(on: bool) {
    crate::gamemode::set(on);
}

#[tauri::command]
pub(crate) fn report_size(view: View, w: f64, h: f64, diag: Option<String>) {
    // diagnostika nízké výšky (frontend ji pošle jen pod 300 px, jednou za změnu) — jeden řádek do logu
    if let Some(d) = diag {
        let d: String = d.chars().filter(|c| !c.is_control()).take(400).collect();
        crate::state::log(&format!("drawer: nízká výška {h:.0} ({view:?}): {d}"));
    }
    core().send_ui(UiCmd::Size(view, w, h));
}

/// Velikost šuplíku tažením za okraje v režimu úprav (`src/layout/drawerSize.ts`; kolo 23). `w`/`h` v
/// logických px, 0 = výchozí (šířka z předvolby, výška podle obsahu). `done = false` = jen okno během
/// tažení (výška se kreslí ve frontendu), `true` = oříznout a uložit do `appearance.size` (záložka
/// `page`, s „Pamatovat pro každou záložku“). Vrací skutečnou velikost po ořezu monitorem `[w, h]`.
#[tauri::command]
pub(crate) fn drawer_size(page: String, w: u32, h: u32, done: bool) -> Result<[u32; 2], String> {
    use crate::appearance::{geom, size};
    let want = size::clamp(size::PageSize { width: w, height: h });
    let c = core();
    let mon = c.monitor();
    let lim = crate::drawerfit::limits(mon.rect, mon.work, mon.dpi);
    // v úpravách může být vidět jiná záložka než `mode` → velikost okna podle té, kterou uživatel táhne
    let id = crate::layout::page_id(&c.cfg.read().layout, &page);
    let prev = size::page();
    size::set_page(&id);
    let eff = [
        if want.width > 0 { geom::drawer_width(want.width, mon.rect.right - mon.rect.left, mon.dpi) as u32 } else { 0 },
        size::fixed_height(want, lim.max_h).map_or(0, |v| v as u32),
    ];
    if !done {
        // přepnutí záložky uvnitř úprav (jiná velikost s `perPage`) = animovaně jako `set_mode`
        let animates = size::switch_animates(&c.cfg.read().appearance.size, &prev, &id);
        size::set_live(size::live_after(animates, want.width));
        c.send_ui(if animates { UiCmd::Resize(true) } else { UiCmd::ResizeLive });
        return Ok(eff);
    }
    size::set_live(None);
    let mut cfg = c.cfg.read().clone();
    cfg.appearance.size.set(&id, want);
    let res = if cfg.appearance.size == c.cfg.read().appearance.size { Ok(()) } else { crate::commands::config::save_config_inner(cfg) };
    c.send_ui(UiCmd::Resize(false));
    res.map(|_| eff)
}

/// Konec úprav (Hotovo / Zrušit): zahodit živou šířku z tažení, ať nezůstane viset (`endSizeEdit`).
#[tauri::command]
pub(crate) fn drawer_size_end() {
    crate::appearance::size::set_live(None);
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

/// Klik do náhledu po najetí (Compact / Island: prázdné místo, „▾ celý šuplík“) nebo jeho setrvání
/// (`drawer.hoverExpandMs`, odpočítává frontend) → celý šuplík stejným openerem. Fokus nebere (otevřený
/// najetím zůstane NOACTIVATE, zavře ho odjetí myši); z jiných pohledů nic.
#[tauri::command]
pub(crate) fn expand_drawer() {
    let c = core();
    if matches!(c.drawer.view(), View::Compact | View::Island) {
        c.send_ui(UiCmd::Show(View::Full, c.drawer.opener()));
    }
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
    core().app.opener().open_url(url, None::<&str>).map_err(|e| crate::errors::any(crate::i18n::tr("Odkaz nejde otevřít", "Can't open the link"), &e))
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
    .map_err(|e| crate::errors::any(&link.name, &e))?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// Stránka, na kterou se šuplík otevírá — pro studený start WebView, kdy událost `page` nedošla.
#[tauri::command]
pub(crate) fn take_page() -> Option<String> {
    core().pending_page.lock().take().map(String::from)
}
