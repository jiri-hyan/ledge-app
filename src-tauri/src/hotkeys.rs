//! Globální zkratky: registrace podle configu (`sync_hotkeys`, plán v `config::rebind_plan`),
//! approve/deny jen při čekající žádosti (`sync_approve_keys`) a obsluha stisku (`on_shortcut`).

use crate::claude::Decision;
use crate::config;
use crate::drawer::{Opener, View};
use crate::state::{UiCmd, core, log};
use std::sync::atomic::Ordering;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

// ---------- zkratky ----------

pub(crate) fn parse_shortcut(s: &str) -> Option<Shortcut> {
    s.parse::<Shortcut>().ok()
}

/// approve/deny jen když čeká žádost Claude.
pub(crate) fn sync_approve_keys() {
    let c = core();
    let want = !c.claude.pending().is_empty();
    if c.approve_keys.swap(want, Ordering::SeqCst) == want {
        return;
    }
    sync_hotkeys();
}

/// Srovná zaregistrované globální zkratky s configem (`config::active_hotkeys`). Při změně nejdřív
/// odregistruje VŠECHNY staré, pak zaregistruje novou sadu (`config::rebind_plan`) — prohození
/// dvou zkratek tak nikdy neskončí bez registrace. Chyby (obsazená zkratka, nečitelný zápis) do logu.
pub(crate) fn sync_hotkeys() {
    let c = core();
    let new = config::active_hotkeys(&c.cfg.read(), c.approve_keys.load(Ordering::SeqCst));
    let mut reg = c.hotkeys.lock();
    let (unreg, regs) = config::rebind_plan(&reg, &new);
    let gs = c.app.global_shortcut();
    for k in unreg {
        if let Some(sc) = parse_shortcut(&k)
            && let Err(e) = gs.unregister(sc)
        {
            log(&format!("zkratka {k}: odregistrace selhala: {e}"));
        }
    }
    for k in regs {
        match parse_shortcut(&k) {
            Some(sc) => {
                if let Err(e) = gs.register(sc) {
                    log(&format!("zkratka {k}: registrace selhala (obsazená jinou aplikací?): {e}"));
                }
            }
            None => log(&format!("zkratka „{k}“ nejde přečíst")),
        }
    }
    *reg = new;
}

pub(crate) fn on_shortcut(sc: &Shortcut) {
    let c = core();
    let hk = c.cfg.read().hotkeys.clone();
    if !hk.enabled {
        return; // vypnuto za běhu: registrace už se ruší v `sync_hotkeys`
    }
    if parse_shortcut(&hk.toggle).as_ref() == Some(sc) {
        if c.fullscreen.load(Ordering::Relaxed) {
            return;
        }
        if c.drawer.view() == View::Full {
            c.send_ui(UiCmd::Hide);
        } else {
            c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey));
        }
    } else if parse_shortcut(&hk.note).as_ref() == Some(sc) {
        if c.fullscreen.load(Ordering::Relaxed) || !c.cfg.read().notes.enabled {
            return;
        }
        c.send_ui(UiCmd::Note);
    } else if parse_shortcut(&hk.approve).as_ref() == Some(sc) {
        c.claude.decide(None, Decision::Allow);
    } else if parse_shortcut(&hk.deny).as_ref() == Some(sc) {
        c.claude.decide(None, Decision::Deny);
    }
}
