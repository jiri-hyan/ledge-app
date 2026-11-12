//! Config za běhu: načtení / čistý config nového uživatele (`load_or_seed`), aplikace změn (`apply_config` — index, sklo,
//! Claude, autostart, zkratky, poznámky, počasí, odkazy, update, vzhled) a vlákno `config-watch`.

use crate::config::{self, Config};
use crate::refresh::{maybe_refresh_weather, reclassify_links};
use crate::state::{core, log, mark_dirty};
use crate::threads::notes_sync::{SyncMsg, send_sync};
use crate::threads::notes_watch::{NotesMsg, notes_root_error};
use crate::{autostart, hotkeys::sync_hotkeys, onboarding};
use std::sync::mpsc;
use tauri::Emitter;
use std::time::Duration;

// ---------- config ----------

pub(crate) fn apply_config(mut new: Config) {
    // `drawer.sections` a `modes` jsou odvozené z `layout` (karta = na nějaké záložce)
    crate::layout::derive(&mut new);
    let c = core();
    let old = c.cfg.read().clone();
    crate::i18n::set_lang(new.ui.lang);
    if old.ui.lang != new.ui.lang {
        crate::tray::relabel();
    }
    *c.cfg.write() = new.clone();
    *c.cfg_error.lock() = notes_root_error(&new);
    if old.roots != new.roots || old.projects != new.projects {
        c.rebuild_index();
        c.git.lock().clear();
    }
    if old.drawer.glass != new.drawer.glass {
        c.drawer.set_glass(new.drawer.glass == config::Glass::Acrylic);
    }
    if old.appearance != new.appearance {
        // motiv se v šuplíku přepne bez restartu (src/theme); proužek překreslí `mark_dirty` níže
        let _ = c.app.emit_to(crate::drawer::LABEL, "appearance", &new.appearance);
    }
    c.claude.set_timeout(Duration::from_secs(new.claude.approval_timeout_s.max(5)));
    c.claude.set_enforce(new.claude.enforce_commit);
    c.claude.set_question_hold(new.claude.question_hold_s);
    c.claude.set_question_unseen(new.claude.question_unseen_s);
    c.claude.set_activity_tail(crate::layout::placed(&new.layout, crate::layout::ACTIVITY_CARD));
    if old.autostart != new.autostart
        && let Err(e) = autostart::apply(new.autostart)
    {
        log(&format!("autostart: {e}"));
    }
    sync_hotkeys();
    if old.notes != new.notes {
        let _ = c.notes_tx.lock().send(NotesMsg::Reroot);
        send_sync(SyncMsg::Config);
    }
    if old.personal.weather.coords() != new.personal.weather.coords() || old.layout != new.layout || old.drawer.sections.weather != new.drawer.sections.weather {
        maybe_refresh_weather(false);
    }
    crate::calendar::live::on_config(&old, &new);
    crate::incidents::on_config(&old, &new);
    crate::limitsfetch::on_config(&old, &new);
    crate::accounts::calsync::on_config(&old, &new);
    crate::accounts::mail::on_config(&old, &new);
    if old.personal.links != new.personal.links {
        reclassify_links();
    }
    if old.actions != new.actions {
        crate::actionstate::refresh(None);
    }
    if old.scripts != new.scripts {
        crate::scripts::refresh_async();
    }
    // widgety: jiná složka = sken znovu; nová karta / položka proužku = probudit plánovač
    crate::widgets::live::on_config(&old, &new);
    // widgety na ploše: motiv a jazyk do jejich oken
    crate::desk::win::on_config(&old, &new);
    crate::pins::win::on_config(&old, &new);
    if old.drawer.sections != new.drawer.sections || old.modes != new.modes {
        // právě hraje / hlasitost / baterie: vypnutá karta = konec odběrů (`media.rs` a spol.)
        crate::media::sync(&new);
        crate::volume::sync(&new);
        crate::battery::sync(&new);
    } else {
        // karta Rychlé přepínače (kolo 22) přidaná / odebraná beze změny `drawer.sections`
        if crate::volume::allowed(&old) != crate::volume::allowed(&new) {
            crate::volume::sync(&new);
        }
        if crate::battery::allowed(&old) != crate::battery::allowed(&new) {
            crate::battery::sync(&new);
        }
    }
    if crate::media::strip_wants(&old) != crate::media::strip_wants(&new) {
        // položka `media` v proužku drží odběry GSMTC i bez záložky Osobní
        crate::media::sync(&new);
    }
    if old.clipboard != new.clipboard {
        // historie schránky: zapnutí = listener, vypnutí = listener pryč + historie z paměti (`cliphist/`)
        crate::cliphist::sync(&new.clipboard);
    }
    if old.update != new.update {
        c.updater.set_config(new.update.clone());
    }
    // karty GitHub / Systém: nová volba = čerstvá data; položka proužku zapne / vypne plánovač
    crate::github::on_config(&old.github, &new.github);
    if crate::integrations::resync_needed(&old, &new) {
        crate::integrations::sync(&new);
    }
    mark_dirty();
    // proužek se přepočítá a překreslí hned na vlákně, které config uložilo (do ~1 snímku), ne až po průchodu
    // smyčkou `emit` (ta má 120ms odstup a řadu kroků před `refresh_strip`); bez změny modelu se nekreslí nic
    crate::stripview::refresh_strip();
}

/// Výsledek čtení configu z disku (čisté rozhodnutí bez zápisu stavu).
pub(crate) enum Load {
    /// soubor se dal přečíst (případně po sekcích)
    Use(Box<config::Partial>),
    /// soubor neexistuje: nový uživatel
    Fresh,
    /// nejde přečíst vůbec (rozbitý JSON): nic k aplikaci
    Unreadable(String),
}

pub(crate) fn decide(r: Result<Option<config::Partial>, String>) -> Load {
    match r {
        Ok(Some(p)) => Load::Use(Box::new(p)),
        Ok(None) => Load::Fresh,
        Err(e) => Load::Unreadable(e),
    }
}

/// Přečte config z disku. `Err` = soubor nejde přečíst a nic se neaplikuje (stav banneru se nemění).
fn read_disk() -> Result<Config, String> {
    let path = config::config_path();
    config::write_schema(&config::config_dir());
    match decide(config::load_lenient(&path)) {
        Load::Use(p) => {
            // vadná sekce nezahodí zbytek: běží na výchozích a banner řekne které (soubor se nemění)
            let warnings = config::recovery::range_warnings(&p.cfg);
            config::recovery::set_load(None, p.bad, warnings);
            Ok(p.cfg)
        }
        Load::Fresh => {
            // nový uživatel: žádné projekty, průvodce při prvním otevření šuplíku
            let cfg = onboarding::fresh_config();
            let _ = config::save(&path, &cfg);
            Ok(cfg)
        }
        Load::Unreadable(e) => Err(e),
    }
}

/// Start appky: nečitelný soubor = výchozí hodnoty a chyba v banneru (`recovery::error`).
pub(crate) fn load_or_seed() -> Config {
    read_disk().unwrap_or_else(|e| {
        config::recovery::set_load(Some(e), Vec::new(), Vec::new());
        Config::default()
    })
}

/// `/ctl/reload`: čte přísně jako hlídač (`watch_config`). Nečitelný soubor i vadná sekce = chyba, nic se
/// neaplikuje a v paměti zůstává poslední dobrá verze. Chybějící soubor = nový uživatel (jako start).
pub(crate) fn reload_from_disk() -> Result<Config, String> {
    let path = config::config_path();
    match config::load(&path) {
        Ok(Some(cfg)) => {
            // jako v hlídači: na disku je platný config, zastaralý banner pryč
            config::recovery::set_applied(&cfg);
            Ok(cfg)
        }
        Ok(None) => read_disk().map_err(|e| reload_failed(&path, &e)),
        Err(e) => Err(reload_failed(&path, &e)),
    }
}

/// Vadný ruční zápis / reload: v paměti zůstává poslední dobrá verze, banner to říká pravdivě
/// (ne „vadné běží na výchozích“). Vrací text banneru.
fn reload_failed(path: &std::path::Path, e: &str) -> String {
    let bad = match config::load_lenient(path) {
        Ok(Some(p)) => p.bad,
        _ => Vec::new(),
    };
    let msg = config::recovery::kept_message(&bad, e);
    config::recovery::set_load(Some(msg.clone()), Vec::new(), Vec::new());
    msg
}

pub(crate) fn watch_config() {
    use notify::{RecursiveMode, Watcher};
    std::thread::Builder::new()
        .name("config-watch".into())
        .spawn(|| {
            let (tx, rx) = mpsc::channel();
            let Ok(mut w) = notify::recommended_watcher(tx) else { return };
            let dir = config::config_dir();
            if w.watch(&dir, RecursiveMode::NonRecursive).is_err() {
                return;
            }
            let path = config::config_path();
            while let Ok(Ok(ev)) = rx.recv() {
                if !ev.paths.iter().any(|p| p.file_name() == path.file_name()) {
                    continue;
                }
                // debounce: editory zapisují víckrát za sebou
                std::thread::sleep(Duration::from_millis(150));
                while rx.try_recv().is_ok() {}
                match config::load(&path) {
                    Ok(Some(cfg)) => {
                        // soubor na disku je platný (i vrácený Ctrl+Z přesně do běžícího stavu): zastaralý banner pryč
                        config::recovery::set_applied(&cfg);
                        if cfg != *core().cfg.read() {
                            apply_config(cfg);
                        } else {
                            mark_dirty();
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        // v paměti zůstává poslední dobrá verze; banner řekne, které sekce jsou vadné
                        reload_failed(&path, &e);
                        mark_dirty();
                    }
                }
            }
            drop(w);
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nečitelný soubor: chyba a nic k aplikaci (`/ctl/reload` nesmí aplikovat výchozí); chybějící = nový uživatel.
    #[test]
    fn unreadable_file_is_error_nothing_to_apply() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        assert!(matches!(decide(config::load_lenient(&p)), Load::Fresh));
        std::fs::write(&p, "{rozbité").unwrap();
        assert!(matches!(decide(config::load_lenient(&p)), Load::Unreadable(e) if !e.is_empty()));
        std::fs::write(&p, "[1]").unwrap();
        assert!(matches!(decide(config::load_lenient(&p)), Load::Unreadable(_)));
        std::fs::write(&p, "{}").unwrap();
        assert!(matches!(decide(config::load_lenient(&p)), Load::Use(_)));
    }

    /// `/ctl/reload` na skutečné cestě (sandbox): vadná sekce i rozbitý JSON = `Err`, platný soubor = `Ok`.
    #[test]
    fn reload_from_disk_rejects_bad_section() {
        let (tmp, _g) = crate::dirs::testing::sandbox();
        let path = config::config_path();
        assert!(path.starts_with(tmp.path()), "sandbox nepřesměroval config_path: {path:?}");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"strip":{"opacity":7}}"#).unwrap();
        let e = reload_from_disk().unwrap_err();
        assert!(e.contains("strip") && e.contains("poslední platná verze"), "{e}");
        assert!(config::recovery::error().is_some_and(|b| b.contains("poslední platná verze")));
        std::fs::write(&path, "{rozbité").unwrap();
        assert!(reload_from_disk().is_err());
        std::fs::write(&path, r#"{"autostart":false}"#).unwrap();
        let cfg = reload_from_disk().unwrap();
        assert!(!cfg.autostart);
        assert_eq!(config::recovery::error(), None);
        config::recovery::set_load(None, Vec::new(), Vec::new());
    }
}
