# Codemapa: jádro (stav, vlákna, příkazy, okna, proužek, procesy)

`lib.rs` je jen seznam modulů a `run()` (Tauri builder, založení `Core`, start vláken, registrace
příkazů `commands::…`). Všechno ostatní je v modulech níže — čti mapu, pak jen jmenovanou funkci.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `lib.rs` | `mod` seznam (první `#[macro_use] mod i18n`), `run()` (`i18n::set_lang` po `load_or_seed`), re-export `crate::log` a `crate::screen_glass_capable`; `RunEvent::ExitRequested` bez kódu (zavřené poslední okno) → `prevent_exit` | `run`, `generate_handler!` |
| `i18n.rs` | jazyk textů jádra (hlášky, chyby, stavy ve snapshotu, proužek): globální atomic z `ui.lang` (`set_lang` při startu a v `apply_config`), výchozí čeština; text v obou jazycích na místě. Testy: jazyk per vlákno (`with_lang`) | `tr(cs, en)`, `trf!(cs, en, args)`, `lang`, `set_lang`, `with_lang` |
| `state.rs` | sdílený stav a jeho přístup | `Core` (pole `pub(crate)`), `CORE`, `core()`, `mark_dirty`, `UiCmd`, `WeatherState`, `Core::{dead, monitor, monitor_fresh, project_dir, rebuild_index, send_ui, strip_logical}`, `MonitorCache`, `MONITOR_TTL`, `FLASH`, `now_ms`, `find_project`, `log` |
| `snapshot.rs` | stav pro UI (`get_state`, událost `state`) — jen paměť, žádné I/O | `snapshot`, `Snapshot`, `ProjectView`, `PersonalView`, `WeatherView`, `LinkView`, `session_rows`, `personal_view`, `monogram`, `github_url` |
| `stripview.rs` | obsah proužku podle `strip.items` (vč. `waiting`, `custom`, `timer`, `media` = `media::strip_now`; časovač i automaticky), segmenty po položkách → `stripconf::compose` (klid, barvy), ambient, puls, viditelnost (`Core::dead`) | `refresh_strip`, `seg`, `project_for_cwd`, `local_clock` |
| `stripconf.rs` | config proužku (`StripConfig`, `StripItem`, `StripColor`; re-export z `config.rs`), validace `strip.custom`, čisté skládání: vlastní text, „v klidu skryté“ (`is_event`), barva na položku, automatická položka časovače | `StripConfig`, `StripItem`, `StripColor::base`, `compose`, `is_event`, `custom_segment`, `effective_items`, `validate`, `CUSTOM_MAX` |
| `timer.rs` | časovač: odpočet + stopky jako časy (žádný tik), akce `Op` → `apply`, `tick` (doběhnutí → `attention::signal` + toast `timer.toast`), `next_deadline` (konec, změny minuty v proužku), segment proužku, stav v `timer.json` (zápis jen při akci), config `TimerConfig` | `TimerState`, `Countdown`, `Stopwatch`, `Op`, `apply`, `tick`, `next_deadline`, `strip_segments`, `minutes_text`, `clock_text`, `act`, `tick_live`, `next_deadline_live`, `init`, `view`, `TimerView` |
| `attention.rs` | pozornost Clauda: počet čekajících session (segment `waiting`, `upgrade_items` pro starý config), přechody „čeká na tebe“ / „hotovo“ ze snímků mostu, dedup 30 s, rozhodnutí bliknutí / puls / toast, tvar pulsu; `on_change` volá `refresh::on_claude_change`; `signal` = bliknutí + puls podle `react` i pro jiné zdroje (konec časovače) | `waiting_count`, `waiting_text`, `upgrade_items`, `ITEMS_REV`, `Seen`, `transitions`, `Dedup`, `Gate`, `react`, `signal`, `pulse_level`, `PULSE_MS`, `pulse`, `on_change` |
| `toast.rs` | oznámení Windows přes WinRT (`windows` crate, `UI_Notifications`), krátké vlákno; QUNS „přijímá oznámení?“ | `show`, `toast_xml`, `xml_escape`, `accepts_notifications` |
| `gamemode.rs` | herní režim: „je Ledge mrtvý?“, rozhodnutí podnětů (zkratky, druhé spuštění), stav v `game-mode.json` vedle configu | `is_dead`, `decide`, `Trigger`, `Outcome`, `handle`, `set`, `load`, `save`, `FILE` |
| `refresh.rs` | obnovy na pozadí + reakce na události (počasí jen s nezakrytou kartou `drawer.sections.weather`) | `maybe_refresh_weather`, `reclassify_links`, `refresh_logos`, `refresh_git_async`, `on_claude_change`, `on_proc_event` |
| `hotkeys.rs` | globální zkratky | `sync_hotkeys`, `sync_approve_keys`, `on_shortcut` |
| `settings.rs` | config za běhu + vlákno `config-watch`; nový uživatel → `onboarding::fresh_config` | `load_or_seed`, `apply_config`, `watch_config` |
| `ctl.rs` | most `/status` doplněk a `/ctl/*` (open, close, settings, launch, stop, reload) | `status_extra`, `ctl_handler` |
| `threads/ui.rs` | vlákno `ui`: sériově `UiCmd` → `Drawer` | `spawn_ui_thread` |
| `threads/emit.rs` | vlákno `emit`: stav do šuplíku, proužek, připomínky, git | `spawn_emit_thread`, `next_wake`, `GIT_EVERY` |
| `threads/hover.rs` | vlákno `hover`: myš, fullscreen, lištové okno, tlak → otevřít/zavřít; tah souborů na proužek → `shelf::Arm` / `shelf::drop::set_armed` ([shelf.md](shelf.md)) | `spawn_hover_thread` |
| `threads/notes_watch.rs`, `notes_sync.rs` | vlákna poznámek (watcher, git sync) — viz [notes.md](notes.md) | `NotesState`, `reload_notes`, `check_reminders`, `SyncMsg`, `spawn_notes_sync` |
| `commands/*.rs` | Tauri příkazy po oblastech: `drawer` (stav, režim, fokus, odkazy), `projects` (launch, open_in, `git_pull`, klon), `notes`, `sessions` (decide, hooky), `config`, `update` | `git_pull` + `pull_message`, `focus_in_own_popup`, `open_link`, `update_install` |
| `focusreturn.rs` | návrat fokusu do předchozí aplikace: `remember` v `Drawer::focus` (těsně před převzetím), `give_back` v `Drawer::hide` / po Full → peek; jen po `UiCmd::Close`, jinak `on_close(false)` zapomene. Čisté `decide`/`keep_or_take` (tabulky) | `remember`, `on_close`, `give_back`, `decide`, `keep_or_take`, `Remembered`, `Probe` |
| `drawer.rs` | okno šuplíku (WebView2), argumenty Chromia, handshake morfu, suspend, sklo, pozice u hrany (`appearance`) | `Drawer::{show, hide, warm, ack, set_dormant, take_focus, release_focus, destroy_if_idle, set_anchor}`, `BROWSER_ARGS`, `View`, `Opener`, `LABEL` |
| `strip.rs` | nativní proužek: vrstvené okno, D2D, click-through, vlákno `strip`; vzhled z `appearance::geom` ([appearance.md](appearance.md)); puls (`StripModel.pulse`) animuje `WM_TIMER` jen po dobu pulsu; překryv přihrádky `StripModel.shelf` → `shelf::overlay` ([shelf.md](shelf.md)) | `Strip::{start, set}`, `StripModel`, `Segment`, `Rgba`, `pulse_now`, `PULSE_TIMER` |
| `edge.rs` | čistá logika „otevřít myší?“ (tabulkové testy) | `armed`, `PushTracker`, `BarLatch`, `is_bar_window`, `edge_passes_through`, `Delays` |
| `rawpush.rs` | raw myš (tlak na hranu), vlákno `rawpush`, aktivní jen u hrany | `RawPush::{start, arm, take}` |
| `screen.rs` | monitory, DPI, kurzor, fullscreen (QUNS + okno v popředí) | `monitors`, `pick`, `fullscreen_on`, `ForegroundProbe` |
| `procman.rs` | dev servery / appky v Job Objectu, URL z logu, porty | `ProcMan::{launch_dev, launch_app, stop, stop_all, snapshot}`, `Phase`, `ProcEvent` |
| `config.rs` | `config.json` + schéma, validace (s polem: `validate_field` → `configerr::FieldError`), zkratky; `Sections` (`drawer.sections`) generuje makro `sections!` — plochý seznam boolů, výchozí `true`, nové pole = jeden řádek | `Config`, `Sections`, `sections!`, `load`, `save`, `validate`, `validate_field`, `active_hotkeys`, `rebind_plan`, `config_dir` |
| `drawerfit.rs` | strop výšky šuplíku (pracovní plocha monitoru) a širší Full pro Nastavení → [settings.md](settings.md) | `limits`, `fit_full` |
| `autostart.rs` / `migrate.rs` | HKCU Run (míří na instalovanou kopii) / Notch → Ledge při startu | `apply`, `decide`, `install_target` / `at_startup`, `plan_notes` |
| `update.rs` / `updatenet.rs` | nová verze (veřejné API, záložně `gh`), vlákno `update`, instalace na klik / HTTP dotaz a stažení | `Updater::{start, check_now, install, view}`, `evaluate`, `fetch_release` / `latest`, `download`, `download_ok` |
| `actions.rs` · `actionrun.rs` · `actionicons.rs` · `actionstate.rs` | rychlé akce → [actions.md](actions.md) | `Action`, `run_action`, `actionstate::refresh`, `views` |
| `webpanel.rs` | okno s webem `web-<id>` (rychlá akce `kind: "web"`): normální okno s rámem, sdílí WebView2 se šuplíkem, zavření = zničení → [actions.md](actions.md) | `open`, `nav_allowed`, `frame_allowed`, `place` |
| `personal.rs` / `weather.rs` | rychlé odkazy (nedůvěryhodný cíl) / Open-Meteo | `classify_fs`, `LinkTarget` / `fetch`, `needs_fetch`, `loading_live`, `status` |
| `media.rs` · `volume.rs` · `battery.rs` | Osobní: právě hraje, hlasitost, baterie → [media.md](media.md) | `ensure`, `sync`, `view` |

## Vlákna a buzení

- `emit` spí na `Core.wake` (`mark_dirty`) s timeoutem `next_wake`: začátek další minuty (hodiny),
  `NotesState.next_due`, konec `FLASH`, git za `GIT_EVERY`, watchdog počasí, reset limitu Claude na 100 %
  (`limitslive::next_deadline`, [limits.md](limits.md)), časovač (`timer::next_deadline_live` — jen když běží:
  konec odpočtu a změna minuty v proužku, ~60/h navíc; pozastavený / žádný nic). V klidu ~80 probuzení/h.
  `hover` ho budí i po změně fullscreenu (peek připomínky).
- `hover`: 16 ms u hrany / 100 ms daleko / 500 ms ve fullscreenu; co 3 s `monitor_fresh` + výčet monitorů.
- `ui`, `notes-watch`, `notes-sync`, `config-watch`, `strip`, `rawpush`, `update`, `media` a `volume` (vzniknou až při prvním zobrazení záložky Osobní; `media` i dřív, je-li v proužku položka `media`, [media.md](media.md)), `shelf-drop` (vznikne až
  při prvním tahu souborů na proužek, [shelf.md](shelf.md)): blokují na kanálu / událostech. `strip` má časovač (~30 fps) jen během pulsu `attention::PULSE_MS` (1,2 s), v klidu žádný;
  `toast` je jednorázové vlákno na jedno oznámení. `logos`, `links`, git (`refresh_git_async`) jsou krátká jednorázová vlákna.

## Invarianty a pasti

- **Fokus:** jen `UiCmd::Focus` z výslovného kliku (`focus_drawer`); hover/peek/připomínka nikdy.
- **Návrat fokusu** (`focusreturn`): `UiCmd::Close` = výslovné zavření (`close_drawer`: Esc, ✕, uložení
  poznámky, paleta „Zavřít“; zkratka `toggle` znovu) → `SetForegroundWindow` zpět, jen když je Ledge
  pořád v popředí a zapamatované okno žije, je vidět, není minimalizované a patří stejnému pid.
  `UiCmd::Hide` (spuštění akce/projektu/odkazu/session, blur, myš, peek, `/ctl/close`) zapamatované okno
  zahodí — nové okno má fokus dostat. `give_back` volá `Drawer::hide` AŽ po `view = Hidden` (blur pak
  nezařadí další `Hide`, který by zavřel peek).
- **Mrtvý Ledge:** `Core::dead()` = `Core.fullscreen` (hover) || `Core.game_mode` (`gamemode::is_dead`) →
  šuplík se schová, proužek zmizí, myš, zkratky, peek, připomínky (nemarkují se jako vyjeté), puls i toast
  nic. Herní režim navíc blokuje `UiCmd::Show`/`Note` přímo ve vlákně `ui` (i `/ctl/open`); vypíná se jen
  `gamemode::set(false)` — zkratka `hotkeys.gameMode`, hlavní zkratka, druhé spuštění (`gamemode::decide`).
- **Upozornění Clauda:** přechody se počítají ze snímků (`attention::Seen`) při každém `on_claude_change`,
  ne v mostu (`claude/` jen čtení). „Hotovo“ = Working → Idle bez `ended`; session, kterou předchozí snímek
  neznal, nic nehlásí. Bliknutí je `Core.flash` (termín `FLASH` v `emit`), puls jen při stylu pohybu ≠ none
  a zapnutých animacích Windows (`SPI_GETCLIENTAREAANIMATION`).
- **Rozpočet:** žádné I/O v `snapshot`/`refresh_strip` (jen cache); git/spawn jen s termínem
  (`gitinfo::run_env`, `capture`) a na událost / otevřený šuplík.
- **Zámky:** nedrž `cfg.read()` při volání `Core::monitor` (rekurzivní read lock parking_lot se
  zasekne, když čeká `apply_config`) — `refresh_strip` bere monitor první.
- Schvalování Clauda jen člověk: `ctl_handler` záměrně nemá `decide`.
- **Paměť WebView2:** `BROWSER_ARGS` = `--disable-gpu --in-process-gpu` + `NetworkServiceInProcess2` →
  5 procesů (ledge, browser, renderer, storage, crashpad). Zavření = `set_dormant(true)`: Low cíl paměti +
  `TrySuspend` (bez nich ~330 MB WS). Argumenty zkoušej přes `scripts/argbench.ps1` (env proměnná,
  bez rekompilace); `--single-process` WebView2 neumí (šuplík se neotevře). Okno s webem (`webpanel.rs`)
  bere stejné `BROWSER_ARGS` a výchozí data dir → stejný proces prohlížeče; očekávaně přidá vlastní
  renderer (izolace webů) a podle stránky utility proces (zvuk/média) — neměřeno, měř `perf.ps1`. Zavřením
  se zničí, renderer skončí.

## Testy

`attention` (počet a text čekajících, `upgrade_items` i přes `config::load`, přechody, dedup, `react`,
tvar pulsu), `stripconf` (serde starých/nových configů, migrace přes `config::load`, vlastní text, `is_event`,
`compose` klid/barvy, automatický časovač, čitelnost předvoleb ve všech motivech), `timer` (akce odpočtu a
stopek, `tick`, termíny, texty, segmenty, tvar `Op`, config, soubor v tempdiru), `gamemode` (`is_dead`, `decide`, stav v tempdiru), `toast` (XML), `threads::emit` (`next_wake`, hodina klidu), `state` (`MonitorCache`; `monitor_cache_bench` je
`#[ignore]`), `commands::projects` (`pull_message`), `threads::notes_watch` (filtr událostí),
`edge`, `config`, `procman`, `autostart`, `migrate`, `personal`, `weather`, `update`, `focusreturn`
(`decide`, `keep_or_take`), `paletteconf`.

Nečti: `claude/` (most, viz [sessions.md](sessions.md)), `notes*` ([notes.md](notes.md)),
`icons.rs`/`cloning.rs` ([projects-icons-perf.md](projects-icons-perf.md)).
