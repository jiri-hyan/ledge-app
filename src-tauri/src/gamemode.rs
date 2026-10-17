//! Herní režim: ruční „mrtvý Ledge“ jako při fullscreen hře — šuplík zavřený, proužek pryč, zkratky,
//! peek ani připomínky nic, dokud ho šéf nevypne. Stav přežije restart (`game-mode.json` vedle configu).
//!
//! Zapíná se tlačítkem v Nastavení (Šuplík a proužek), položkou palety Ctrl+K nebo zkratkou
//! `hotkeys.gameMode`. Vypíná se (šuplík je v něm nedosažitelný, proto víc cest):
//! - zkratkou `hotkeys.gameMode` (přepíná),
//! - hlavní zkratkou `hotkeys.toggle` (vypne a otevře šuplík),
//! - dalším spuštěním Ledge (zástupce / Start; single-instance → vypne a otevře šuplík).
//!
//! Ve skutečné fullscreen hře tyhle cesty herní režim jen vypnou — šuplík se neotevře (fullscreen vyhrává).

use crate::drawer::{Opener, View};
use crate::state::{UiCmd, core, log, mark_dirty};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

/// Soubor se stavem (v `config::config_dir`); existuje a říká `"on": true` = herní režim zapnutý.
pub(crate) const FILE: &str = "game-mode.json";

/// Je Ledge mrtvý? Fullscreen hra (`Core.fullscreen`) nebo ruční herní režim.
pub(crate) fn is_dead(fullscreen: bool, game_mode: bool) -> bool {
    fullscreen || game_mode
}

/// Odkud přišel podnět.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Trigger {
    /// zkratka `hotkeys.gameMode`
    GameKey,
    /// hlavní zkratka `hotkeys.toggle`
    ToggleKey,
    /// zkratka rychlé poznámky
    NoteKey,
    /// druhé spuštění Ledge (single-instance)
    Relaunch,
}

/// Co se má stát.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Nothing,
    /// běžné chování (přepnout šuplík / poznámka / otevřít)
    Normal,
    GameOn,
    GameOff,
    /// vypnout herní režim a otevřít šuplík
    GameOffAndOpen,
}

/// Rozhodnutí podle stavu (tabulkový test). Zkratka herního režimu funguje vždy; hlavní zkratka a
/// další spuštění herní režim vypnou; zbytek je v mrtvém Ledge nic.
pub(crate) fn decide(t: Trigger, fullscreen: bool, game_mode: bool) -> Outcome {
    match (t, game_mode) {
        (Trigger::GameKey, true) => Outcome::GameOff,
        (Trigger::GameKey, false) => Outcome::GameOn,
        (Trigger::ToggleKey | Trigger::Relaunch, true) if fullscreen => Outcome::GameOff,
        (Trigger::ToggleKey | Trigger::Relaunch, true) => Outcome::GameOffAndOpen,
        (Trigger::NoteKey, true) => Outcome::Nothing,
        // dnešní chování mimo herní režim: zkratky ve fullscreen nic, druhé spuštění otevře vždy
        (Trigger::ToggleKey | Trigger::NoteKey, false) if fullscreen => Outcome::Nothing,
        (_, false) => Outcome::Normal,
    }
}

fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// Uložený stav; chybějící / nečitelný soubor = vypnuto.
pub(crate) fn load(dir: &Path) -> bool {
    std::fs::read(path_in(dir))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.get("on").and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

/// Zapnuto = soubor `{"on": true}`, vypnuto = soubor pryč.
pub(crate) fn save(dir: &Path, on: bool) -> std::io::Result<()> {
    let p = path_in(dir);
    if on {
        std::fs::create_dir_all(dir)?;
        std::fs::write(p, b"{\n  \"on\": true\n}\n")
    } else {
        match std::fs::remove_file(p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// Zapne / vypne herní režim za běhu (uloží, schová šuplík, překreslí proužek).
pub(crate) fn set(on: bool) {
    let c = core();
    if c.game_mode.swap(on, Ordering::SeqCst) == on {
        return;
    }
    log(&format!("herní režim: {on}"));
    if let Err(e) = save(&crate::config::config_dir(), on) {
        log(&format!("herní režim: stav nejde uložit: {e}"));
    }
    if on {
        c.send_ui(UiCmd::Hide);
    }
    crate::stripview::refresh_strip();
    // vypnutí → emit vlákno dožene peek připomínek a stav
    mark_dirty();
}

/// Provede rozhodnutí `decide`; `normal` = běžné chování podnětu.
pub(crate) fn handle(t: Trigger, normal: impl FnOnce()) {
    let c = core();
    match decide(t, c.fullscreen.load(Ordering::Relaxed), c.game_mode.load(Ordering::SeqCst)) {
        Outcome::Nothing => {}
        Outcome::Normal => normal(),
        Outcome::GameOn => set(true),
        Outcome::GameOff => set(false),
        Outcome::GameOffAndOpen => {
            set(false);
            c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_table() {
        for (fs, gm, want) in [(false, false, false), (true, false, true), (false, true, true), (true, true, true)] {
            assert_eq!(is_dead(fs, gm), want, "fullscreen {fs}, herní režim {gm}");
        }
    }

    #[test]
    fn decide_table() {
        use Outcome::*;
        use Trigger::*;
        // (podnět, fullscreen, herní režim) → výsledek
        let cases = [
            (GameKey, false, false, GameOn),
            (GameKey, true, false, GameOn),
            (GameKey, false, true, GameOff),
            (GameKey, true, true, GameOff),
            (ToggleKey, false, false, Normal),
            (ToggleKey, true, false, Nothing),
            (ToggleKey, false, true, GameOffAndOpen),
            (ToggleKey, true, true, GameOff),
            (NoteKey, false, false, Normal),
            (NoteKey, true, false, Nothing),
            (NoteKey, false, true, Nothing),
            (NoteKey, true, true, Nothing),
            (Relaunch, false, false, Normal),
            (Relaunch, true, false, Normal),
            (Relaunch, false, true, GameOffAndOpen),
            (Relaunch, true, true, GameOff),
        ];
        for (t, fs, gm, want) in cases {
            assert_eq!(decide(t, fs, gm), want, "{t:?} fullscreen {fs} herní {gm}");
        }
    }

    #[test]
    fn persists_in_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(!load(d.path()), "bez souboru vypnuto");
        save(d.path(), true).unwrap();
        assert!(load(d.path()));
        save(d.path(), true).unwrap();
        assert!(load(d.path()), "opakované zapnutí");
        save(d.path(), false).unwrap();
        assert!(!load(d.path()) && !d.path().join(FILE).exists());
        save(d.path(), false).unwrap();
        std::fs::write(d.path().join(FILE), "nesmysl").unwrap();
        assert!(!load(d.path()), "nečitelný = vypnuto");
        std::fs::write(d.path().join(FILE), r#"{"on":false}"#).unwrap();
        assert!(!load(d.path()));
        let nested = d.path().join("nova");
        save(&nested, true).unwrap();
        assert!(load(&nested), "složka se založí");
    }
}
