//! Ledge — jádro: seznam modulů a `run()` (Tauri builder, založení `Core`, start vláken, registrace
//! příkazů). Stav je v `state.rs`, snapshot pro UI v `snapshot.rs`, příkazy v `commands/`, vlákna
//! v `threads/` — mapa: `docs/codemap/core.md`.
//!
//! Vlákna (všechna spí, když se nic neděje):
//! - `hover`   — poloha myši (100 ms daleko od okraje, 16 ms u něj), fullscreen (500 ms)
//! - `ui`      — sériově obsluhuje šuplík (animace nikdy neběží na hlavním vlákně)
//! - `emit`    — posílá stav do šuplíku max. ~8×/s a jen když je vidět; překresluje proužek; v klidu
//!   se budí jen na začátku minuty (hodiny) a v nejbližším termínu (`threads::emit::next_wake`)
//! - `strip`   — vlastní smyčka zpráv nativního proužku
//! - `rawpush` — raw myš (tlak na horní hranu), zaregistrovaná jen u hrany nad prohlížečem
//! - `claude-bridge` — HTTP pro hooky
//! - `notes-watch` — sleduje složku poznámek (události, žádný poll) a přepočítá připomínky
//! - počasí (záložka Osobní) — žádné vlákno navíc: jedno stažení na runtime Tauri, jen při otevření
//!   šuplíku na Osobní a datech starších 30 min

// první: makro `trf!` (texty podle jazyka) musí být definované před moduly, které ho používají
#[macro_use]
mod i18n;
mod actionicons;
mod actionrun;
mod actions;
mod actionstate;
mod appearance;
mod autostart;
mod claude;
mod cloning;
mod commands;
mod config;
mod configerr;
mod configio;
mod ctl;
mod drawer;
mod drawerfit;
mod edge;
mod gitinfo;
mod hooksetup;
mod hotkeys;
mod iconimg;
mod icons;
mod migrate;
mod notes;
mod notesync;
mod onboarding;
mod personal;
mod ports;
mod procman;
mod projectdetect;
mod rawpush;
mod refresh;
mod resolve;
mod screen;
mod sessionlive;
mod sessions;
mod settings;
mod snapshot;
mod state;
mod strip;
mod stripview;
mod subagents;
mod threads;
mod uiconf;
mod update;
mod updatenet;
mod weather;
mod winfocus;

pub(crate) use state::{log, screen_glass_capable};

use claude::Claude;
use commands::drawer::focus_in_own_popup;
use drawer::{Drawer, Opener, View};
use hotkeys::{on_shortcut, sync_hotkeys};
use parking_lot::{Condvar, Mutex, RwLock};
use procman::ProcMan;
use refresh::{on_claude_change, on_proc_event, reclassify_links};
use resolve::RepoIndex;
use settings::{load_or_seed, watch_config};
use state::{CORE, Core, MonitorCache, UiCmd, WeatherState, core, mark_dirty};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use strip::Strip;
use stripview::refresh_strip;
use tauri_plugin_global_shortcut::ShortcutState;
use threads::notes_sync::spawn_notes_sync;
use threads::notes_watch::{NotesState, notes_root_error, spawn_notes_watch};
use threads::{emit::spawn_emit_thread, hover::spawn_hover_thread, ui::spawn_ui_thread};

// ---------- start ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Automatizace (testy, CI): žádné okno, nic na obrazovce. Bariéru nikdy neobcházet.
    // `LEDGE_NO_GUI`, starší `NOTCH_NO_GUI` platí taky (skripty z doby před přejmenováním).
    if ["LEDGE_NO_GUI", "NOTCH_NO_GUI"].iter().any(|k| std::env::var(k).as_deref() == Ok("1")) {
        return;
    }
    tauri::Builder::default()
        // Jedna instance podle `identifier` (`cz.hyan.ledge`). Starý Notch (`cz.hyan.notch`) tím NENÍ
        // chycený — oba by běžely vedle sebe a pral by se o port mostu 47800 (`claude.rs`, hooky
        // v `~/.claude/settings.json` na něj míří, proto se nemění). Starý Notch před startem Ledge zavři.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // druhé spuštění = otevři šuplík
            let _ = app;
            if let Some(c) = CORE.get() {
                c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey));
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|_app, sc, ev| {
                    if ev.state() == ShortcutState::Pressed {
                        on_shortcut(sc);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Notch → Ledge: config a poznámky ze starých složek (jednou, idempotentně) — před
            // `load_or_seed`, jinak by založil čistý config a migrace by se už nespustila.
            for line in migrate::at_startup() {
                log(&line);
            }
            let (cfg, cfg_err) = load_or_seed();
            i18n::set_lang(cfg.ui.lang);
            let (tx, rx) = mpsc::channel();
            let (notes_tx, notes_rx) = mpsc::channel();
            let (sync_tx, sync_rx) = mpsc::channel();
            let procs = ProcMan::new(Arc::new(on_proc_event));
            let claude = Claude::new(Arc::new(on_claude_change), Duration::from_secs(cfg.claude.approval_timeout_s.max(5)));
            claude.set_enforce(cfg.claude.enforce_commit);
            claude.set_question_hold(cfg.claude.question_hold_s);
            let updater = update::Updater::start(cfg.update.clone(), Arc::new(mark_dirty));
            let core_arc = Arc::new(Core {
                app: app.handle().clone(),
                cfg: RwLock::new(cfg.clone()),
                cfg_error: Mutex::new(cfg_err.or_else(|| notes_root_error(&cfg))),
                procs,
                claude: claude.clone(),
                strip: Strip::start(),
                drawer: Drawer::new(app.handle().clone()),
                index: Mutex::new(RepoIndex::default()),
                git: Mutex::new(HashMap::new()),
                logos: Mutex::new(icons::Store::default()),
                mode: Mutex::new("prace".into()),
                dirty: Mutex::new(true),
                wake: Condvar::new(),
                flash: Mutex::new(None),
                fullscreen: AtomicBool::new(false),
                bar_window: AtomicBool::new(false),
                approve_keys: AtomicBool::new(false),
                ui: Mutex::new(tx),
                notes: Mutex::new(NotesState::default()),
                notes_tx: Mutex::new(notes_tx.clone()),
                notes_sync: Mutex::new(notesync::Status::default()),
                sync_tx: Mutex::new(sync_tx),
                sync_quiet_ms: std::sync::atomic::AtomicI64::new(0),
                pending_page: Mutex::new(None),
                hotkeys: Mutex::new(Vec::new()),
                weather: Mutex::new(WeatherState::default()),
                links: Mutex::new((0, HashMap::new())),
                links_gen: std::sync::atomic::AtomicU64::new(0),
                updater,
                monitors: Mutex::new(MonitorCache::default()),
            });
            let _ = CORE.set(core_arc.clone());
            core_arc.rebuild_index();
            reclassify_links();
            actionstate::refresh(None);

            core_arc.drawer.set_glass(cfg.drawer.glass == config::Glass::Acrylic);
            core_arc.drawer.set_on_blur(Arc::new(|| {
                let c = core();
                c.drawer.note_blur();
                if c.drawer.view() == View::Full && c.drawer.opener() == Opener::Hotkey && !focus_in_own_popup() {
                    c.send_ui(UiCmd::Hide);
                }
            }));

            sync_hotkeys();
            if cfg.claude.enabled
                && let Err(e) = claude::serve(cfg.claude.port, claude, ctl::status_extra(), ctl::ctl_handler())
            {
                *core_arc.cfg_error.lock() = Some(trf!("Most pro Claude neběží: {e}", "The Claude bridge isn't running: {e}"));
            }
            sessionlive::start(Arc::new(mark_dirty), Arc::new(|| core().drawer.view() == View::Full));
            spawn_ui_thread(rx);
            spawn_emit_thread();
            spawn_hover_thread();
            watch_config();
            spawn_notes_watch(notes_rx, notes_tx);
            spawn_notes_sync(sync_rx);
            if let Err(e) = autostart::apply(cfg.autostart) {
                log(&format!("autostart: {e}"));
            }
            refresh_strip();
            // šuplík připravit na pozadí (studený start WebView2 trvá sekundy) — ne v úsporném režimu
            if cfg.drawer.keep_warm_s == 0 {
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(1500));
                    core().send_ui(UiCmd::Warm);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::drawer::get_state,
            commands::projects::launch,
            commands::projects::stop,
            commands::projects::dismiss,
            commands::projects::logs,
            commands::sessions::decide,
            commands::drawer::set_mode,
            commands::drawer::report_size,
            commands::drawer::ui_ready,
            commands::drawer::anim_ack,
            commands::drawer::anim_stats,
            commands::drawer::close_drawer,
            commands::drawer::focus_drawer,
            commands::drawer::release_focus,
            commands::drawer::open_url,
            commands::projects::open_in,
            commands::projects::git_pull,
            commands::projects::project_logo,
            commands::projects::clone_project,
            commands::config::get_config,
            commands::config::save_config,
            commands::config::open_config_file,
            commands::settings::check_config,
            commands::settings::config_defaults,
            commands::settings::config_export,
            commands::settings::config_import_preview,
            commands::settings::pick_path,
            commands::settings::pick_save,
            commands::settings::project_detect,
            commands::settings::project_candidates,
            commands::settings::logo_preview,
            commands::settings::app_info,
            commands::settings::open_app_file,
            commands::notes::note_create,
            commands::notes::note_list,
            commands::notes::note_done,
            commands::notes::note_open,
            commands::notes::note_prefill,
            commands::notes::note_query,
            commands::notes::note_read,
            commands::notes::note_save,
            commands::notes::note_toggle_task,
            commands::notes::note_render,
            commands::notes::note_edit_draft_save,
            commands::notes::note_edit_draft_load,
            commands::notes::note_draft_load,
            commands::notes::note_draft_save,
            commands::notes::note_draft_images,
            commands::notes::note_draft_clear,
            commands::notes::notes_sync_now,
            commands::drawer::take_page,
            commands::drawer::open_link,
            commands::update::update_check_now,
            commands::update::update_install,
            commands::sessions::session_open,
            commands::sessions::hooks_status,
            commands::sessions::hooks_install,
            commands::sessions::hooks_remove,
            commands::sessions::question_answer,
            commands::sessions::question_release,
            commands::actions::run_action,
            commands::actions::action_icon,
            commands::actions::action_icon_refresh,
            commands::actions::action_consent,
            commands::actions::action_check,
            commands::onboarding::onboarding_repos,
            commands::onboarding::onboarding_apps,
            commands::onboarding::onboarding_icons,
            commands::onboarding::onboarding_claude,
            commands::onboarding::onboarding_pick_folder,
            commands::onboarding::onboarding_themes,
            commands::onboarding::onboarding_apply
        ])
        .build(tauri::generate_context!())
        .expect("ledge: start selhal")
        .run(|_app, ev| {
            if let tauri::RunEvent::Exit = ev
                && let Some(c) = CORE.get()
            {
                c.procs.stop_all();
            }
        });
}
