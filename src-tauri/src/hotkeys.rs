//! Globální zkratky: registrace podle configu (`sync_hotkeys`, plán v `config::rebind_plan`),
//! approve/deny jen při čekající žádosti a mimo hru (`sync_approve_keys`), průchozí režim lístků jen s lístkem na
//! ploše (`config::active_hotkeys_pins`) a obsluha stisku (`on_shortcut`).

use crate::claude::Decision;
use crate::config;
use crate::drawer::{Opener, View};
use crate::gamemode::{self, Trigger};
use crate::state::{UiCmd, core, log};
use std::sync::atomic::Ordering;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

// ---------- zkratky ----------

pub(crate) fn parse_shortcut(s: &str) -> Option<Shortcut> {
    s.parse::<Shortcut>().ok()
}

/// Mají být approve/deny zaregistrované? Jen s čekající žádostí a mimo hru: `RegisterHotKey` by kombinaci
/// hře sebral a stisk naslepo by rozhodl žádost, kterou člověk nevidí.
fn approve_keys_wanted(pending: bool, dead: bool) -> bool {
    pending && !dead
}

/// Rozhodnutí podle stisknuté zkratky; v mrtvém Ledge žádné (klávesa mohla přijít těsně před odregistrací).
fn key_decision(approve: bool, deny: bool, dead: bool) -> Option<Decision> {
    match (approve, deny, dead) {
        (_, _, true) => None,
        (true, _, false) => Some(Decision::Allow),
        (false, true, false) => Some(Decision::Deny),
        (false, false, false) => None,
    }
}

/// Srovná approve/deny se stavem; volá se při změně žádostí a při přechodu do hry a z ní (`attention::catch_up`).
pub(crate) fn sync_approve_keys() {
    let c = core();
    let want = approve_keys_wanted(!c.claude.pending().is_empty(), c.dead());
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
    // lístek na ploše = i zkratka průchozího režimu (`pins/`), i s vypnutými zkratkami
    let pins_open = crate::pins::win::any_open();
    let new = config::active_hotkeys_pins(&c.cfg.read(), c.approve_keys.load(Ordering::SeqCst), pins_open);
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
    let replanned = *reg != new;
    let mut failed = Vec::new();
    for k in regs {
        match parse_shortcut(&k) {
            Some(sc) => {
                if let Err(e) = gs.register(sc) {
                    log(&format!("zkratka {k}: registrace selhala (obsazená jinou aplikací?): {e}"));
                    failed.push(k);
                }
            }
            None => log(&format!("zkratka „{k}“ nejde přečíst")),
        }
    }
    // beze změny sady se nic nepřeregistrovalo → stav „obsazená“ z minula platí dál
    if replanned {
        *TAKEN.lock() = config::taken_actions(&new, &failed);
    }
    *reg = new;
}

/// Akce, jejichž zkratku při poslední registraci odmítl systém (obsazená jinou aplikací): UI to ukáže u zkratky
/// (`commands::config::hotkeys_taken`), jinak by Nastavení hlásilo „uloženo“ a zkratka by mlčky nefungovala.
static TAKEN: parking_lot::Mutex<Vec<(&'static str, String)>> = parking_lot::Mutex::new(Vec::new());

pub(crate) fn taken() -> Vec<(&'static str, String)> {
    TAKEN.lock().clone()
}

/// Je zkratka `k` zaregistrovaná Ledgem (obsazená jinou aplikací → ne)?
pub(crate) fn is_registered(k: &str) -> bool {
    parse_shortcut(k).is_some_and(|sc| core().app.global_shortcut().is_registered(sc))
}

pub(crate) fn on_shortcut(sc: &Shortcut) {
    let c = core();
    let hk = c.cfg.read().hotkeys.clone();
    // průchozí režim lístků: platí i s vypnutými zkratkami a v herním režimu (cesta ven z průchozího)
    if parse_shortcut(&hk.note_passthrough).as_ref() == Some(sc) && crate::pins::win::any_open() {
        crate::pins::win::toggle_through_all();
        return;
    }
    // rychlý nový lístek: platí i s vypnutými zkratkami (výchozí prázdná = neregistruje se), ve hře nic
    if !hk.note_pin.trim().is_empty() && parse_shortcut(&hk.note_pin).as_ref() == Some(sc) {
        crate::commands::pins::quick_new();
        return;
    }
    if !hk.enabled {
        return; // vypnuto za běhu: registrace už se ruší v `sync_hotkeys`
    }
    // fullscreen a herní režim rozhoduje `gamemode::decide` (zkratka herního režimu platí vždy,
    // hlavní zkratka herní režim vypne)
    if parse_shortcut(&hk.game_mode).as_ref() == Some(sc) {
        gamemode::handle(Trigger::GameKey, || {});
    } else if parse_shortcut(&hk.toggle).as_ref() == Some(sc) {
        gamemode::handle(Trigger::ToggleKey, || {
            if c.drawer.view() == View::Full {
                c.send_ui(UiCmd::Close);
            } else {
                c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey));
            }
        });
    } else if parse_shortcut(&hk.note).as_ref() == Some(sc) {
        if !c.cfg.read().notes.enabled {
            return;
        }
        gamemode::handle(Trigger::NoteKey, || c.send_ui(UiCmd::Note));
    } else if let Some(d) = key_decision(parse_shortcut(&hk.approve).as_ref() == Some(sc), parse_shortcut(&hk.deny).as_ref() == Some(sc), c.dead()) {
        c.claude.decide(None, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_keys_only_with_request_and_outside_game() {
        for (pending, dead, want) in [(true, false, true), (true, true, false), (false, false, false), (false, true, false)] {
            assert_eq!(approve_keys_wanted(pending, dead), want, "{pending} {dead}");
        }
    }

    #[test]
    fn key_decides_only_outside_game() {
        for (approve, deny, dead, want) in [
            (true, false, false, Some(Decision::Allow)),
            (false, true, false, Some(Decision::Deny)),
            (true, false, true, None),
            (false, true, true, None),
            (false, false, false, None),
            (false, false, true, None),
        ] {
            assert_eq!(key_decision(approve, deny, dead), want, "{approve} {deny} {dead}");
        }
    }
}
