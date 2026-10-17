# Codemapa: jádro (stav, vlákna, příkazy, okna, proužek, procesy)

`lib.rs` je jen seznam modulů a `run()` (Tauri builder, založení `Core`, start vláken, registrace
příkazů `commands::…`). Všechno ostatní je v modulech níže — čti mapu, pak jen jmenovanou funkci.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `lib.rs` | `mod` seznam (první `#[macro_use] mod i18n`), `run()` (`i18n::set_lang` po `load_or_seed`), re-export `crate::log` a `crate::screen_glass_capable` | `run`, `generate_handler!` |
| `i18n.rs` | jazyk textů jádra (hlášky, chyby, stavy ve snapshotu, proužek): globální atomic z `ui.lang` (`set_lang` při startu a v `apply_config`), výchozí čeština; text v obou jazycích na místě. Testy: jazyk per vlákno (`with_lang`) | `tr(cs, en)`, `trf!(cs, en, args)`, `lang`, `set_lang`, `with_lang` |
| `state.rs` | sdílený stav a jeho přístup | `Core` (pole `pub(crate)`), `CORE`, `core()`, `mark_dirty`, `UiCmd`, `WeatherState`, `Core::{monitor, monitor_fresh, project_dir, rebuild_index, send_ui, strip_logical}`, `MonitorCache`, `MONITOR_TTL`, `FLASH`, `now_ms`, `find_project`, `log` |
| `snapshot.rs` | stav pro UI (`get_state`, událost `state`) — jen paměť, žádné I/O | `snapshot`, `Snapshot`, `ProjectView`, `PersonalView`, `WeatherView`, `LinkView`, `session_rows`, `personal_view`, `monogram`, `github_url` |
| `stripview.rs` | obsah proužku podle `strip.items`, ambient, viditelnost | `refresh_strip`, `seg`, `project_for_cwd`, `local_clock` |
| `refresh.rs` | obnovy na pozadí + reakce na události | `maybe_refresh_weather`, `reclassify_links`, `refresh_logos`, `refresh_git_async`, `on_claude_change`, `on_proc_event` |
| `hotkeys.rs` | globální zkratky | `sync_hotkeys`, `sync_approve_keys`, `on_shortcut` |
| `settings.rs` | config za běhu + vlákno `config-watch`; nový uživatel → `onboarding::fresh_config` | `load_or_seed`, `apply_config`, `watch_config` |
| `ctl.rs` | most `/status` doplněk a `/ctl/*` (open, close, settings, launch, stop, reload) | `status_extra`, `ctl_handler` |
| `threads/ui.rs` | vlákno `ui`: sériově `UiCmd` → `Drawer` | `spawn_ui_thread` |
| `threads/emit.rs` | vlákno `emit`: stav do šuplíku, proužek, připomínky, git | `spawn_emit_thread`, `next_wake`, `GIT_EVERY` |
| `threads/hover.rs` | vlákno `hover`: myš, fullscreen, lištové okno, tlak → otevřít/zavřít | `spawn_hover_thread` |
| `threads/notes_watch.rs`, `notes_sync.rs` | vlákna poznámek (watcher, git sync) — viz [notes.md](notes.md) | `NotesState`, `reload_notes`, `check_reminders`, `SyncMsg`, `spawn_notes_sync` |
| `commands/*.rs` | Tauri příkazy po oblastech: `drawer` (stav, režim, fokus, odkazy), `projects` (launch, open_in, `git_pull`, klon), `notes`, `sessions` (decide, hooky), `config`, `update` | `git_pull` + `pull_message`, `focus_in_own_popup`, `open_link`, `update_install` |
| `drawer.rs` | okno šuplíku (WebView2), argumenty Chromia, handshake morfu, suspend, sklo, pozice u hrany (`appearance`) | `Drawer::{show, hide, warm, ack, set_dormant, take_focus, release_focus, destroy_if_idle, set_anchor}`, `BROWSER_ARGS`, `View`, `Opener`, `LABEL` |
| `strip.rs` | nativní proužek: vrstvené okno, D2D, click-through, vlákno `strip`; vzhled z `appearance::geom` ([appearance.md](appearance.md)) | `Strip::{start, set}`, `StripModel`, `Segment`, `Rgba` |
| `edge.rs` | čistá logika „otevřít myší?“ (tabulkové testy) | `armed`, `PushTracker`, `BarLatch`, `is_bar_window`, `edge_passes_through`, `Delays` |
| `rawpush.rs` | raw myš (tlak na hranu), vlákno `rawpush`, aktivní jen u hrany | `RawPush::{start, arm, take}` |
| `screen.rs` | monitory, DPI, kurzor, fullscreen (QUNS + okno v popředí) | `monitors`, `pick`, `fullscreen_on`, `ForegroundProbe` |
| `procman.rs` | dev servery / appky v Job Objectu, URL z logu, porty | `ProcMan::{launch_dev, launch_app, stop, stop_all, snapshot}`, `Phase`, `ProcEvent` |
| `config.rs` | `config.json` + schéma, validace (s polem: `validate_field` → `configerr::FieldError`), zkratky | `Config`, `load`, `save`, `validate`, `validate_field`, `active_hotkeys`, `rebind_plan`, `config_dir` |
| `drawerfit.rs` | strop výšky šuplíku (pracovní plocha monitoru) a širší Full pro Nastavení → [settings.md](settings.md) | `limits`, `fit_full` |
| `autostart.rs` / `migrate.rs` | HKCU Run (míří na instalovanou kopii) / Notch → Ledge při startu | `apply`, `decide`, `install_target` / `at_startup`, `plan_notes` |
| `update.rs` / `updatenet.rs` | nová verze (veřejné API, záložně `gh`), vlákno `update`, instalace na klik / HTTP dotaz a stažení | `Updater::{start, check_now, install, view}`, `evaluate`, `fetch_release` / `latest`, `download`, `download_ok` |
| `actions.rs` · `actionrun.rs` · `actionicons.rs` · `actionstate.rs` | rychlé akce → [actions.md](actions.md) | `Action`, `run_action`, `actionstate::refresh`, `views` |
| `personal.rs` / `weather.rs` | rychlé odkazy (nedůvěryhodný cíl) / Open-Meteo | `classify_fs`, `LinkTarget` / `fetch`, `needs_fetch`, `loading_live`, `status` |

## Vlákna a buzení

- `emit` spí na `Core.wake` (`mark_dirty`) s timeoutem `next_wake`: začátek další minuty (hodiny),
  `NotesState.next_due`, konec `FLASH`, git za `GIT_EVERY`, watchdog počasí. V klidu ~80 probuzení/h.
  `hover` ho budí i po změně fullscreenu (peek připomínky).
- `hover`: 16 ms u hrany / 100 ms daleko / 500 ms ve fullscreenu; co 3 s `monitor_fresh` + výčet monitorů.
- `ui`, `notes-watch`, `notes-sync`, `config-watch`, `strip`, `rawpush`, `update`: blokují na kanálu /
  událostech. `logos`, `links`, git (`refresh_git_async`) jsou krátká jednorázová vlákna.

## Invarianty a pasti

- **Fokus:** jen `UiCmd::Focus` z výslovného kliku (`focus_drawer`); hover/peek/připomínka nikdy.
- **Fullscreen:** `Core.fullscreen` (hover) → šuplík se schová, proužek zmizí, zkratky a peek nic.
- **Rozpočet:** žádné I/O v `snapshot`/`refresh_strip` (jen cache); git/spawn jen s termínem
  (`gitinfo::run_env`, `capture`) a na událost / otevřený šuplík.
- **Zámky:** nedrž `cfg.read()` při volání `Core::monitor` (rekurzivní read lock parking_lot se
  zasekne, když čeká `apply_config`) — `refresh_strip` bere monitor první.
- Schvalování Clauda jen člověk: `ctl_handler` záměrně nemá `decide`.
- **Paměť WebView2:** `BROWSER_ARGS` = `--disable-gpu --in-process-gpu` + `NetworkServiceInProcess2` →
  5 procesů (ledge, browser, renderer, storage, crashpad). Zavření = `set_dormant(true)`: Low cíl paměti +
  `TrySuspend` (bez nich ~330 MB WS). Argumenty zkoušej přes `scripts/argbench.ps1` (env proměnná,
  bez rekompilace); `--single-process` WebView2 neumí (šuplík se neotevře).

## Testy

`threads::emit` (`next_wake`, hodina klidu), `state` (`MonitorCache`; `monitor_cache_bench` je
`#[ignore]`), `commands::projects` (`pull_message`), `threads::notes_watch` (filtr událostí),
`edge`, `config`, `procman`, `autostart`, `migrate`, `personal`, `weather`, `update`.

Nečti: `claude/` (most, viz [sessions.md](sessions.md)), `notes*` ([notes.md](notes.md)),
`icons.rs`/`cloning.rs` ([projects-icons-perf.md](projects-icons-perf.md)).
