//! Ikona Ledge v oznamovací oblasti: jediný viditelný důkaz, že Ledge běží, a jediná cesta, jak ho
//! ukončit. Levý klik = otevřít šuplík, pravé tlačítko = nabídka Otevřít · Nastavení · Ukončit Ledge.
//!
//! - Žádné vlákno ani polling: ikonu i nabídku obsluhuje smyčka zpráv hlavního vlákna (Tauri `tray-icon`).
//! - Klik uživatele je výslovný, takže fokus brát smí (`Opener::Hotkey`, jako zkratka); „nekrade fokus“ platí
//!   pro automatické akce. Otevření jde přes `gamemode::handle(Relaunch)` — ruční herní režim tím vypne,
//!   stejně jako druhé spuštění Ledge.
//! - Ukončení = `AppHandle::exit(0)` → `RunEvent::Exit` v `lib.rs` (zastaví spravované procesy, uloží lístky
//!   na ploše) — tutéž cestu používá aktualizace; Tauri při ní ikonu z oznamovací oblasti sundá.
//! - Jazyk popisků z `ui.lang`; po změně jazyka je přepíše [`relabel`].

use crate::drawer::{Opener, View};
use crate::gamemode::{self, Trigger};
use crate::state::{UiCmd, core, log};
use crate::uiconf::Lang;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter};

const TRAY_ID: &str = "ledge";
const ID_OPEN: &str = "tray-open";
const ID_SETTINGS: &str = "tray-settings";
const ID_QUIT: &str = "tray-quit";

/// Co uživatel v ikoně vybral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Settings,
    Quit,
}

/// Položka nabídky (id) → akce; neznámé id = nic. Čistá funkce.
pub(crate) fn action_for(id: &str) -> Option<Action> {
    match id {
        ID_OPEN => Some(Action::Open),
        ID_SETTINGS => Some(Action::Settings),
        ID_QUIT => Some(Action::Quit),
        _ => None,
    }
}

/// Popisky ikony v jednom jazyce.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Texts {
    pub tooltip: &'static str,
    pub open: &'static str,
    pub settings: &'static str,
    pub quit: &'static str,
}

/// Popisky podle jazyka (čistá funkce — testy nesahají na globální jazyk).
pub(crate) fn texts(lang: Lang) -> Texts {
    match lang {
        Lang::Cs => Texts { tooltip: "Ledge — najeď myší k hornímu okraji", open: "Otevřít", settings: "Nastavení", quit: "Ukončit Ledge" },
        Lang::En => Texts { tooltip: "Ledge — move the mouse to the top edge", open: "Open", settings: "Settings", quit: "Quit Ledge" },
    }
}

fn build_menu(app: &AppHandle, t: &Texts) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, ID_OPEN, t.open, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, ID_SETTINGS, t.settings, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, ID_QUIT, t.quit, true, None::<&str>)?;
    Menu::with_items(app, &[&open, &settings, &sep, &quit])
}

/// Založí ikonu (jednou, v `setup`). Selhání se jen zaloguje — Ledge bez ikony běží dál.
pub(crate) fn init(app: &AppHandle) {
    if let Err(e) = build(app) {
        log(&format!("tray: ikona se nepovedla: {e}"));
    }
}

fn build(app: &AppHandle) -> tauri::Result<()> {
    let t = texts(crate::i18n::lang());
    let menu = build_menu(app, &t)?;
    let mut b = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(t.tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|_app, ev| {
            if let Some(a) = action_for(ev.id.as_ref()) {
                run(a);
            }
        })
        .on_tray_icon_event(|_tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                run(Action::Open);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}

/// Po změně jazyka přepíše tooltip a nabídku (nabídka se staví na hlavním vlákně).
pub(crate) fn relabel() {
    let app = core().app.clone();
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = h.tray_by_id(TRAY_ID) else { return };
        let t = texts(crate::i18n::lang());
        let _ = tray.set_tooltip(Some(t.tooltip));
        if let Ok(menu) = build_menu(&h, &t) {
            let _ = tray.set_menu(Some(menu));
        }
    });
}

fn run(a: Action) {
    let c = core();
    match a {
        Action::Open => gamemode::handle(Trigger::Relaunch, || c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey))),
        Action::Settings => gamemode::handle(Trigger::Relaunch, || {
            crate::studio::open_or(None, true, || {
                let c = core();
                *c.pending_page.lock() = Some("settings");
                c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey));
                let _ = c.app.emit_to(crate::drawer::LABEL, "page", "settings");
            })
        }),
        Action::Quit => {
            log("tray: ukončuji na přání uživatele");
            std::thread::spawn(|| {
                // mimo vlákno nabídky; `Exit` v `lib.rs` zastaví procesy a uloží lístky
                std::thread::sleep(Duration::from_millis(50));
                core().app.exit(0);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_map_to_actions() {
        for (id, want) in [(ID_OPEN, Some(Action::Open)), (ID_SETTINGS, Some(Action::Settings)), (ID_QUIT, Some(Action::Quit)), ("", None), ("tray-nic", None), ("open", None)] {
            assert_eq!(action_for(id), want, "{id:?}");
        }
    }

    #[test]
    fn texts_exist_in_both_languages() {
        for lang in [Lang::Cs, Lang::En] {
            let t = texts(lang);
            for s in [t.tooltip, t.open, t.settings, t.quit] {
                assert!(!s.trim().is_empty(), "{lang:?}");
            }
            assert!(t.tooltip.starts_with("Ledge"), "tooltip jmenuje aplikaci: {}", t.tooltip);
            assert!(t.quit.contains("Ledge"), "Ukončit pojmenuje, co ukončí: {}", t.quit);
        }
        assert_ne!(texts(Lang::Cs), texts(Lang::En));
        // všechny tři položky mají různé popisky (jinak by nabídka nedávala smysl)
        let t = texts(Lang::Cs);
        assert!(t.open != t.settings && t.settings != t.quit && t.open != t.quit);
    }
}
