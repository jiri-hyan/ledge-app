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
    if old.drawer.sections != new.drawer.sections || old.modes != new.modes {
        // právě hraje / hlasitost / baterie: vypnutá karta = konec odběrů (`media.rs` a spol.)
        crate::media::sync(&new);
        crate::volume::sync(&new);
        crate::battery::sync(&new);
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
    if old.strip != new.strip || old.github != new.github || old.system != new.system {
        crate::integrations::sync(&new);
    }
    mark_dirty();
}

pub(crate) fn load_or_seed() -> (Config, Option<String>) {
    let path = config::config_path();
    config::write_schema(&config::config_dir());
    match config::load(&path) {
        Ok(Some(cfg)) => (cfg, None),
        Ok(None) => {
            // nový uživatel: žádné projekty, průvodce při prvním otevření šuplíku
            let cfg = onboarding::fresh_config();
            let _ = config::save(&path, &cfg);
            (cfg, None)
        }
        Err(e) => (Config::default(), Some(e)),
    }
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
                        if cfg != *core().cfg.read() {
                            apply_config(cfg);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        *core().cfg_error.lock() = Some(e);
                        mark_dirty();
                    }
                }
            }
            drop(w);
        })
        .ok();
}
