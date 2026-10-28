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
//! - `shelf-drop` — cíl přetažení přihrádky (OLE); vznikne až při prvním tahu souborů na proužek
//! - `cliphist` — historie schránky: jen se zapnutým `clipboard.enabled`, spí do `WM_CLIPBOARDUPDATE`
//! - `claude-bridge` — HTTP pro hooky a `/mcp` (MCP server pro Clauda, `mcp/`; každý požadavek vlastní krátké vlákno)
//! - `notes-watch` — sleduje složku poznámek (události, žádný poll) a přepočítá připomínky
//! - `pins-save` — zpožděný zápis stavu lístků na plochu (`pins/`); vznikne s prvním lístkem, spí na kanálu
//! - `desk-save` — totéž pro widgety na ploše (`desk/`); vznikne s prvním widgetem, spí na kanálu
//! - `media` / `volume` (záložka Osobní) — vzniknou až při prvním zobrazení záložky (`personal_live`),
//!   pak jen čekají na události GSMTC / Core Audio; baterie = callback Windows, žádné vlákno
//! - počasí (záložka Osobní) — žádné vlákno navíc: jedno stažení na runtime Tauri, jen při otevření
//!   šuplíku na Osobní a datech starších 30 min

// první: makro `trf!` (texty podle jazyka) musí být definované před moduly, které ho používají
#[macro_use]
mod i18n;
#[cfg(test)]
mod hostile;
#[cfg(test)]
mod testrng;
mod accounts;
mod actionicons;
mod actionrun;
mod actions;
mod actionstate;
mod agentrun;
mod agentview;
mod appearance;
mod approvalsound;
mod attention;
mod autostart;
mod calendar;
mod battery;
mod claude;
mod cliphist;
mod cloning;
mod dataexport;
mod commands;
mod config;
mod costslive;
mod configerr;
mod configio;
mod ctl;
mod ctxsize;
mod desk;
mod drawer;
mod drawerfit;
mod edge;
mod errors;
mod focusreturn;
mod gamemode;
mod gitinfo;
mod github;
mod hooksetup;
mod hotkeys;
mod integrations;
mod hovermode;
mod iconimg;
mod icons;
mod incidents;
mod limits;
mod layout;
mod limitslive;
mod looks;
mod mcp;
mod linkmigrate;
mod media;
mod mediaconf;
mod mediatime;
mod migrate;
mod notes;
mod notesync;
mod onboarding;
mod outside;
mod paletteconf;
mod smartconf;
mod personal;
mod pins;
mod pkgctx;
mod ports;
mod procman;
mod projectdetect;
mod rawpush;
mod refresh;
mod resolve;
mod runguard;
mod screen;
mod scripts;
mod sessiongit;
mod sessionlive;
mod sessions;
mod settings;
mod shelf;
mod snapshot;
mod state;
mod statusline;
mod statuslinesetup;
mod strip;
mod stripconf;
mod stripdraw;
mod stripview;
mod subagents;
mod suggest;
mod sysmon;
mod threads;
mod tilelook;
mod timer;
mod toast;
mod tray;
mod uiconf;
mod update;
mod usage;
mod volume;
mod updatenet;
mod updater;
mod updcache;
mod updseen;
mod updsign;
#[cfg(test)]
mod ipcacl;
mod weather;
mod webpanel;
mod widgets;
mod winfocus;
mod winquick;
mod winquickwin;
mod winsettings;

pub(crate) use state::{log, screen_glass_capable};

/// `ledge.exe --statusline` (wrapper stavového řádku Claude Code) — volá `main.rs` místo `run()`.
pub fn statusline_main() -> i32 {
    statusline::main()
}

/// `ledge.exe --mcp` (stdio proxy MCP pro Claude Code / Desktop, `mcp/proxy.rs`) — volá `main.rs` místo `run()`.
pub fn mcp_main() -> i32 {
    mcp::proxy::main()
}

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
    // „Restartovat mimo“ (bublina MSIX, `pkgctx.rs`): počkat, až stará instance skončí — jinak by ji
    // single-instance chytil a tahle by hned skončila
    pkgctx::wait_for_predecessor();
    tauri::Builder::default()
        // Jedna instance podle `identifier` (`cz.hyan.ledge`). Starý Notch (`cz.hyan.notch`) tím NENÍ
        // chycený — oba by běžely vedle sebe a pral by se o port mostu 47800 (`claude.rs`, hooky
        // v `~/.claude/settings.json` na něj míří, proto se nemění). Starý Notch před startem Ledge zavři.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // druhé spuštění = otevři šuplík (v herním režimu ho vypne — `gamemode::decide`)
            let _ = app;
            if let Some(c) = CORE.get() {
                gamemode::handle(gamemode::Trigger::Relaunch, || c.send_ui(UiCmd::Show(View::Full, Opener::Hotkey)));
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
            // bublina MSIX (Ledge spuštěný z Claude desktop): zalogovat; zbylé jednorázové úlohy uklidit
            log(&pkgctx::startup_line(pkgctx::ctx()));
            outside::cleanup_later();
            // běžící odpočet / stopky z minula (`timer.json`; zapisuje se jen při akci uživatele)
            timer::init(&config::config_dir());
            let (tx, rx) = mpsc::channel();
            let (notes_tx, notes_rx) = mpsc::channel();
            let (sync_tx, sync_rx) = mpsc::channel();
            let procs = ProcMan::new(Arc::new(on_proc_event));
            let claude = Claude::new(Arc::new(on_claude_change), Duration::from_secs(cfg.claude.approval_timeout_s.max(5)));
            claude.set_enforce(cfg.claude.enforce_commit);
            claude.set_question_hold(cfg.claude.question_hold_s);
            claude.set_question_unseen(cfg.claude.question_unseen_s);
            claude.set_activity_tail(layout::placed(&cfg.layout, layout::ACTIVITY_CARD));
            let updater = update::Updater::start(cfg.update.clone(), actionrun::local_dir().join(updcache::DIR), Arc::new(mark_dirty));
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
                game_mode: AtomicBool::new(gamemode::load(&config::config_dir())),
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
                if c.drawer.view().covers_strip() && c.drawer.opener() == Opener::Hotkey && !focus_in_own_popup() {
                    c.send_ui(UiCmd::Hide);
                }
            }));

            sync_hotkeys();
            // ikona v oznamovací oblasti: důkaz, že Ledge běží, a cesta k ukončení (`tray.rs`)
            tray::init(app.handle());
            // nový uživatel: šuplík se jednou otevře sám s Vítej (bez fokusu, ne ve hře) — `onboarding/first_run.rs`
            onboarding::first_run::schedule();
            if cfg.claude.enabled
                && let Err(e) = claude::serve(cfg.claude.port, claude, ctl::status_extra(), ctl::ctl_handler(), mcp::host::endpoint())
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
            threads::notes_folder::spawn_notes_folder();
            limitslive::start_watch();
            // právě hraje v proužku (`strip.items` + `media`): odběry GSMTC hned, ne až po záložce Osobní
            media::sync(&core().cfg.read());
            // položky proužku GitHub / Systém: plánovač vznikne jen s nimi (`integrations.rs`)
            let cfg_now = core().cfg.read().clone();
            integrations::sync(&cfg_now);
            // historie schránky: listener jen se zapnutou funkcí (výchozí vypnuto)
            let clip = core().cfg.read().clipboard.clone();
            cliphist::sync(&clip);
            // widgety ze skriptů: vlákno jen, když je nějaký widget na záložce nebo v proužku
            widgets::live::wake_if_used(&core().cfg.read());
            if let Err(e) = autostart::apply(cfg.autostart) {
                log(&format!("autostart: {e}"));
            }
            refresh_strip();
            // lístky na plochu z minula zpátky na místa (po předehřátí šuplíku — jeden proces WebView2)
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(2500));
                pins::win::restore_all();
                // widgety na ploše (`desk/`): bez `desk.json` se nic neotevře
                desk::win::restore_all();
            });
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
            commands::projects::restart,
            commands::projects::stop_all_servers,
            commands::projects::project_folders,
            commands::projects::port_holders,
            commands::projects::kill_port_holder,
            commands::projects::dismiss,
            commands::projects::logs,
            commands::sessions::decide,
            commands::drawer::set_mode,
            commands::drawer::set_game_mode,
            commands::drawer::report_size,
            commands::drawer::drawer_size,
            commands::drawer::drawer_size_end,
            commands::drawer::ui_ready,
            commands::drawer::anim_ack,
            commands::drawer::anim_stats,
            commands::drawer::close_drawer,
            commands::drawer::focus_drawer,
            commands::drawer::expand_drawer,
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
            commands::settings::launcher_catalog_read,
            commands::settings::launcher_import_check,
            commands::pkgctx::pkg_context,
            commands::pkgctx::relaunch_outside,
            commands::notes::note_create,
            commands::notes::note_list,
            commands::notes::note_done,
            commands::notes::note_open,
            commands::notes::note_open_with_apps,
            commands::notes::note_open_with,
            commands::notes::note_export,
            commands::notes::note_prefill,
            commands::notes::note_query,
            commands::notes::note_search,
            commands::notes::note_suggest_remind,
            commands::notes::note_read,
            commands::notes::note_copy_text,
            commands::notes::note_save,
            commands::notes::note_toggle_task,
            commands::noteslife::note_archive,
            commands::noteslife::note_unarchive,
            commands::noteslife::note_delete,
            commands::noteslife::note_delete_undo,
            commands::notes::note_render,
            commands::notes::note_edit_draft_save,
            commands::notes::note_edit_draft_load,
            commands::notes::note_draft_load,
            commands::notes::note_draft_save,
            commands::notes::note_draft_images,
            commands::notes::note_draft_clear,
            commands::notes::notes_sync_now,
            commands::notefolder::note_folder_sync_now,
            commands::notefolder::note_folder_check,
            commands::notefolder::note_folder_clear_conflicts,
            commands::pins::pin_toggle,
            commands::pins::pin_new,
            commands::pins::pin_through_all,
            commands::pins::pin_init,
            commands::pins::pin_set,
            commands::pins::pin_drag,
            commands::pins::pin_unpin,
            commands::pins::pin_edit_in_ledge,
            commands::pins::pin_hint_seen,
            commands::pins::pin_open,
            commands::pins::pin_look_all,
            commands::pins::pin_hide,
            commands::notelinks::note_set_meta,
            commands::notelinks::note_tags,
            commands::notelinks::note_link_resolve,
            commands::notelinks::note_link_suggest,
            commands::notelinks::note_link_create,
            commands::desk::desk_pin,
            commands::desk::desk_unpin,
            commands::desk::desk_init,
            commands::desk::desk_set,
            commands::desk::desk_drag,
            commands::desk::desk_menu,
            commands::desk::desk_open_drawer,
            commands::desk::desk_profile_save,
            commands::desk::desk_profile_apply,
            commands::desk::desk_profile_delete,
            commands::desk::desk_hide_all,
            commands::drawer::take_page,
            commands::drawer::open_link,
            commands::update::update_check_now,
            commands::update::update_install,
            commands::update::update_retry,
            commands::update::update_notes_read,
            commands::update::update_rollback,
            commands::sessions::session_open,
            commands::sessiongit::session_git,
            commands::sessions::session_context,
            commands::sessiongit::session_git_diff,
            commands::sessiongit::session_git_commit,
            commands::sessiongit::session_git_push,
            commands::sessiongit::session_git_pull,
            commands::sessiongit::session_git_discard,
            commands::sessiongit::session_git_remove_worktree,
            commands::sessiongit::session_git_open,
            commands::sessions::hooks_status,
            commands::sessions::hooks_install,
            commands::sessions::hooks_remove,
            commands::sessions::question_answer,
            commands::sessions::question_release,
            commands::sessions::question_seen,
            commands::agents::agent_action,
            commands::agents::agent_spawn,
            commands::limits::statusline_status,
            commands::limits::statusline_install,
            commands::limits::statusline_remove,
            commands::timer::timer,
            commands::costs::costs_refresh,
            commands::smart::export_usage,
            commands::media::personal_live,
            commands::media::media_control,
            commands::media::media_cover,
            commands::media::media_favorite_open,
            commands::media::volume_set,
            commands::media::volume_mute,
            commands::media::volume_mic_mute,
            commands::media::volume_set_device,
            commands::winquick::wq_state,
            commands::winquick::wq_config_set,
            commands::winquick::wq_terminal,
            commands::winquick::wq_folder,
            commands::winquick::wq_theme_toggle,
            commands::winquick::wq_radio_set,
            commands::winquick::wq_settings,
            commands::winquick::wq_output_set,
            commands::winquick::wq_shot,
            commands::calendar::calendar_test,
            commands::geo::geo_search,
            commands::integrations::system_view,
            commands::integrations::system_taskmgr,
            commands::integrations::github_view,
            commands::integrations::github_open,
            commands::integrations::github_copy_login,
            commands::calendar::calendar_join,
            commands::calendar::calendar_outlook_probe,
            commands::calendar::calendar_outlook_load,
            commands::accounts::accounts_status,
            commands::accounts::account_connect,
            commands::accounts::account_cancel,
            commands::accounts::account_disconnect,
            commands::accounts::account_google_calendars,
            commands::actions::run_action,
            commands::shelf::shelf_list,
            commands::shelf::shelf_icon,
            commands::shelf::shelf_copy,
            commands::shelf::shelf_open,
            commands::shelf::shelf_reveal,
            commands::shelf::shelf_remove,
            commands::shelf::shelf_clear,
            commands::shelf::shelf_drag_out,
            commands::shelf::shelf_paste,
            commands::shelf::shelf_copy_content,
            commands::shelf::shelf_resize_image,
            commands::cliphist::clip_list,
            commands::cliphist::clip_paste,
            commands::cliphist::clip_pin,
            commands::cliphist::clip_remove,
            commands::cliphist::clip_clear,
            commands::cliphist::clip_enable,
            commands::cliphist::clip_apps,
            commands::actions::action_icon,
            commands::tilelook::look_image_add,
            commands::tilelook::look_image,
            commands::actions::action_icon_refresh,
            commands::actions::action_consent,
            commands::actions::action_check,
            commands::scripts::scripts_list,
            commands::scripts::run_script,
            commands::scripts::scripts_open_dir,
            commands::scripts::win_settings_open,
            commands::widgets::widgets_list,
            commands::widgets::widget_consent,
            commands::widgets::widget_refresh,
            commands::widgets::widget_output,
            commands::widgets::widget_edit,
            commands::widgets::widgets_open_dir,
            commands::widgets::widget_templates,
            commands::widgets::widget_install,
            commands::widgets::widget_click,
            commands::onboarding::onboarding_repos,
            commands::onboarding::onboarding_apps,
            commands::onboarding::onboarding_icons,
            commands::onboarding::onboarding_claude,
            commands::onboarding::onboarding_pick_folder,
            commands::onboarding::onboarding_themes,
            commands::onboarding::onboarding_apply,
            commands::mcp::mcp_state,
            commands::mcp::mcp_set_enabled,
            commands::mcp::mcp_rotate_key,
            commands::mcp::mcp_reveal_key
        ])
        .build(tauri::generate_context!())
        .expect("ledge: start selhal")
        .run(|_app, ev| {
            // zavřené poslední okno (okno s webem, když šuplík úsporně neexistuje) Ledge neukončí;
            // výslovné `exit(kód)` (aktualizace) má kód a projde
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = &ev {
                api.prevent_exit();
            }
            if let tauri::RunEvent::Exit = ev
                && let Some(c) = CORE.get()
            {
                c.procs.stop_all();
                // poslední posun lístku mohl čekat na zpožděný zápis
                pins::win::flush();
                desk::save::flush();
            }
        });
}
