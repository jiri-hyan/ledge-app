# Codemapa: jádro (stav, vlákna, příkazy, okna, proužek, procesy)

`lib.rs` je jen seznam modulů a `run()` (Tauri builder, založení `Core`, start vláken, registrace příkazů `commands::…`). Všechno ostatní je v modulech níže — čti mapu, pak jen jmenovanou funkci.

Detail (toky, invarianty, testy, co nečíst, historie): [core-detail.md](core-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `lib.rs` | `mod` seznam (první `#[macro_use] mod i18n`), `mcp_main` (`ledge.exe --mcp` … | `run`, `generate_handler!` |
| `testrng.rs` | jen testy: deterministický xorshift generátor (`Rng`) pro vlastnostní a … | `Rng`, `soup`, `below` |
| `hostile.rs` | jen testy: nepřátelské vstupy parserů (výstup gitu, cesty, hlavičky … | `fuzz` |
| `i18n.rs` | jazyk textů jádra (hlášky, chyby, stavy ve snapshotu, proužek) … | `tr(cs, en)` |
| `errors.rs` | srozumitelné chyby: `io::Error` / text (`os error N`, `(0x8007NNNN)`) / … | `Kind` `kind_of_io` |
| `state.rs` | sdílený stav a jeho přístup | `Core` `pub(crate)` `CORE` |
| `snapshot.rs` | stav pro UI (`get_state`, událost `state`) — jen paměť, žádné I/O | `snapshot` `Snapshot` |
| `stripview.rs` | obsah proužku podle `strip.items` (vč. `waiting`, `custom`, `timer`, `media` = … | `refresh_strip` `look_facts` |
| `looks/*` | styly zavřeného Ledge (`strip.look`, `strip.lookHide`) … | `Look` `Facts` `Show` `Part` |
| `stripdraw.rs` | kreslení stylů zavřeného Ledge (D2D, písmo ikon zjištěné jednou při startu … | `draw`, `pick_fonts`, `Fonts` |
| `stripconf.rs` | config proužku (`StripConfig` vč. `look`/`lookHide`, `StripItem`, `StripColor` … | `StripConfig` `StripItem` |
| `timer.rs` | časovač: odpočet + stopky jako časy (žádný tik), akce `Op` → `apply`, `tick` … | `TimerState` `Countdown` |
| `attention.rs` | pozornost Clauda: počet čekajících session (segment `waiting`, `upgrade_items` … | `Event::rank` |
| `toast.rs` | oznámení Windows přes WinRT (`windows` crate, `UI_Notifications`), krátké … | `show` `show_xml` `toast_xml` |
| `tray.rs` | ikona v oznamovací oblasti (Tauri `tray-icon`, bez vlákna a pollingu) … | `init` `relabel` `action_for` |
| `gamemode.rs` | herní režim: „je Ledge mrtvý?“, rozhodnutí podnětů (zkratky, druhé spuštění) … | `is_dead` `decide` `Trigger` |
| `refresh.rs` | obnovy na pozadí + reakce na události (počasí jen s kartou na otevřené záložce … | `maybe_refresh_weather` |
| `hotkeys.rs` | globální zkratky | `sync_hotkeys` |
| `settings.rs` | config za běhu + vlákno `config-watch`; nový uživatel → … | `load_or_seed` `apply_config` |
| `ctl.rs` | most `/status` doplněk a `/ctl/*` (open, close, settings, launch, stop, reload) | `status_extra`, `ctl_handler` |
| `threads/ui.rs` | vlákno `ui`: sériově `UiCmd` → `Drawer` | `spawn_ui_thread` |
| `threads/emit.rs` | vlákno `emit`: stav do šuplíku, proužek, připomínky, git | `spawn_emit_thread` |
| `threads/hover.rs` | vlákno `hover`: myš, fullscreen, lištové okno, tlak → otevřít/zavřít … | `spawn_hover_thread` |
| `threads/notes_watch.rs`, `notes_sync.rs` | vlákna poznámek (watcher, git sync) — viz [notes.md](notes.md) | `NotesState` `reload_notes` |
| `commands/*.rs` (`commands/{config,pkgctx,timer,update}.rs`) | Tauri příkazy po oblastech: `drawer` (stav, režim, fokus, odkazy … | `git_pull` `pull_message` |
| `focusreturn.rs` | návrat fokusu do předchozí aplikace: `remember` v `Drawer::focus` (těsně před … | `remember` `on_close` |
| `drawer.rs` | okno šuplíku (WebView2), argumenty Chromia, handshake morfu, suspend, sklo … | `Drawer::{show, hide, warm, ack, set_dormant, take_focus, release_focus, destroy_if_idle, set_anchor, set_expand_ms}` |
| `hovermode.rs` | co se stane po najetí (`drawer.hover`: `full` výchozí / `compact` / `island` … | `HoverMode` `hover_target` |
| `strip.rs` | nativní proužek: vrstvené okno, D2D, click-through, vlákno `strip` … | `Strip::{start, set}` |
| `edge.rs` | čistá logika „otevřít myší?“ (tabulkové testy) | `armed` `PushTracker` |
| `rawpush.rs` | raw myš (tlak na hranu), vlákno `rawpush`, aktivní jen u hrany | `RawPush::{start, arm, take}` |
| `screen.rs` | monitory, DPI, kurzor, fullscreen (QUNS + okno v popředí) | `monitors` `pick` |
| `procman.rs` | dev servery / appky v Job Objectu, URL z logu, porty | `ProcMan::{launch_dev, launch_app, stop, stop_all, snapshot}` |
| `config/*` (`config/{claude,drawer,hotkeys,io,personal,project,sections,validate}.rs`) | `config.json` + schéma, validace (s polem … | `Config` `Sections` |
| `src-tauri/src/config/validate.rs` | `validate` / `validate_field` celého configu … | `validate` `validate_field` |
| `drawerfit.rs` | strop výšky šuplíku (80 % pracovní plochy monitoru) a širší Full pro Nastavení … | `limits`, `fit_full` |
| `fsx.rs` | sdílený atomický zápis (unikátní tmp, `sync_all`, rename s retry), JSON load/save | `write_atomic` `read_retry` `save_json` |
| `dirs.rs` | jediné čtení `%USERPROFILE%`/`%APPDATA%`/`%LOCALAPPDATA%`; v testech panika bez přepsání (`dirs::testing`) | `home` `appdata` `claude_dir` |
| `util.rs` | `now_ms`, `cut`/`clip_line` (UTF-8, `…`), `pct` (saturující) | `now_ms` `cut` `pct` |
| `autostart.rs` / `migrate.rs` | HKCU Run (míří na instalovanou kopii; v bublině MSIX nic … | `apply` `gate` `decide` |
| `pkgctx.rs` | bublina MSIX (Ledge spuštěný z Claude desktop) … | `ctx` `current` `detect` |
| `outside.rs` | spuštění MIMO bublinu: jednorázová úloha Plánovače z XML (`InteractiveToken` … | `task_xml` `run_task` |
| `update.rs` / `updatenet.rs` | nová verze: verze, rozbor vydání (veřejné API, záložně `gh` … | `evaluate` `evaluate_tag` |
| `updsign.rs` | podpis instalátoru (S1): minisign / Ed25519 ve formátu `tauri signer sign` … | `verify_bytes` `verify_file` |
| `ipcacl.rs` | kdo smí volat vlastní příkazy (N4): `build.rs` předá seznam z … | `handler_commands` |
| `updater.rs` | stav aktualizací a vlákno `update` (spí na kanálu, žádný časovač navíc) … | `Updater::{start, check_now, retry_fetch, mark_notes_read, install, rollback, view}` |
| `updcache.rs` | cache instalátorů `%LOCALAPPDATA%\Ledge\updates\<tag>` (manifest `release.json` … | `Manifest` `Entry` `scan` |
| `updseen.rs` | `updates\seen.json`: naposledy spuštěná verze, předchozí (cíl návratu) … | `Seen` `Known` `on_start` |
| `actions.rs` · `actionrun.rs` · `actionicons.rs` · `actionstate.rs` | rychlé akce → [actions.md](actions.md) | `Action` `run_action` |
| `pins/*` | lístky na plochu — okna `note-<klíč>` s UI Ledge (`pin.html`), stav … | `win::open` `win::sync_dead` |
| `desk/*` | widgety na ploše — okna `desk-<id>` s UI Ledge (`desk.html`), stav `desk.json` … | `win::pin` `win::sync_yield` |
| `webpanel.rs` | okno s webem `web-<id>` (rychlá akce `kind: "web"`) … | `open` `nav_allowed` |
| `personal.rs` / `weather.rs` | rychlé odkazy (nedůvěryhodný cíl) / Open-Meteo | `classify_fs` `LinkTarget` |
| `cliphist/*` | historie schránky (vlákno `cliphist` jen se zapnutým `clipboard.enabled`) → … | `sync`, `paste` |
| `media.rs` · `volume.rs` · `battery.rs` | Osobní: právě hraje, hlasitost, baterie → [media.md](media.md) | `ensure`, `sync`, `view` |

Další soubory (podrobnosti v detailu):
`src-tauri/src/{rawpush.rs,updcache.rs,updseen.rs,rawpush.rs,updcache.rs,updseen.rs}` · `src/ui/errortext.ts`
