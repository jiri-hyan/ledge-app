# Codemapa: jádro (stav, vlákna, příkazy, okna, proužek, procesy)

`lib.rs` je jen seznam modulů a `run()` (Tauri builder, založení `Core`, start vláken, registrace
příkazů `commands::…`). Všechno ostatní je v modulech níže — čti mapu, pak jen jmenovanou funkci.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `lib.rs` | `mod` seznam (první `#[macro_use] mod i18n`), `mcp_main` (`ledge.exe --mcp`, [mcp.md](mcp.md)), `run()` (`i18n::set_lang` po `load_or_seed`), re-export `crate::log` a `crate::screen_glass_capable`; `RunEvent::ExitRequested` bez kódu (zavřené poslední okno) → `prevent_exit` | `run`, `generate_handler!` |
| `testrng.rs` | jen testy: deterministický xorshift generátor (`Rng`) pro vlastnostní a „nepřátelské“ testy bez nové závislosti; stejné semínko = stejná posloupnost | `Rng`, `soup`, `below` |
| `hostile.rs` | jen testy: nepřátelské vstupy parserů (výstup gitu, cesty, hlavičky skriptů/widgetů, text poznámek a schránky, odkazy schůzek) — žádná panika; správnost hlídají testy v modulech | `fuzz` |
| `i18n.rs` | jazyk textů jádra (hlášky, chyby, stavy ve snapshotu, proužek): globální atomic z `ui.lang` (`set_lang` při startu a v `apply_config`), výchozí čeština; text v obou jazycích na místě. Testy: jazyk per vlákno (`with_lang`) | `tr(cs, en)`, `trf!(cs, en, args)`, `lang`, `set_lang`, `with_lang` |
| `errors.rs` | srozumitelné chyby: `io::Error` / text (`os error N`, `(0x8007NNNN)`) / `serde_json` / git / `reqwest` → druh `Kind` → věta „co se stalo + co s tím“ (cs/en); původní text jde do logu (`detail`), nikdy do UI. Na hranici příkazu: `.map_err(\|e\| errors::io(tr("Poznámku nejde uložit", "Can't save the note"), &e))`; neznámé → „Kontext: něco se nepovedlo — podrobnosti jsou v logu.“; už lidské věty z jiných modulů jen `prefixed` | `Kind`, `kind_of_io`, `kind_of_text`, `kind_of_git`, `kind_of_net`, `render`, `io`, `spawn`, `spawn_text`, `any`, `json`, `git`, `net`, `prefixed`, `looks_technical`; FE pojistka `ui/errortext.ts` (`humanError`, `looksTechnical`) v `errorText` (`ui/toast.tsx`) |
| `state.rs` | sdílený stav a jeho přístup | `Core` (pole `pub(crate)`), `CORE`, `core()`, `mark_dirty`, `UiCmd`, `WeatherState`, `Core::{dead, monitor, monitor_fresh, project_dir, rebuild_index, send_ui, strip_logical}`, `MonitorCache`, `MONITOR_TTL`, `FLASH`, `now_ms`, `find_project`, `log` |
| `snapshot.rs` | stav pro UI (`get_state`, událost `state`) — jen paměť, žádné I/O | `snapshot`, `Snapshot`, `ProjectView`, `PersonalView`, `WeatherView`, `LinkView`, `session_rows`, `personal_view`, `monogram`, `github_url` |
| `stripview.rs` | obsah proužku podle `strip.items` (vč. `waiting`, `custom`, `timer`, `media` = `media::strip_now`; časovač i automaticky), segmenty po položkách → `stripconf::compose` (klid, barvy), ambient, puls, viditelnost (`Core::dead`); u stylů zavřeného Ledge (`strip.look` ≠ `classic`) sbírá `looks::Facts` z paměti (`look_facts`) → `looks::show` do `StripModel.show` | `refresh_strip`, `look_facts`, `seg`, `project_for_cwd`, `local_clock` |
| `looks/*` | styly zavřeného Ledge (`strip.look`, `strip.lookHide`): čisté „stav → co styl ukáže“ (`show`, priorita `events`), převody faktů (`facts`), tvar, místo a hover zóna (`frame`) → [appearance.md](appearance.md) | `Look`, `Facts`, `Show`, `Part`, `Glyph`, `show`, `events`, `without`, `frame::{spec, place, zone, WidthAnim}` |
| `stripdraw.rs` | kreslení stylů zavřeného Ledge (D2D, písmo ikon zjištěné jednou při startu vlákna `strip`) | `draw`, `pick_fonts`, `Fonts` |
| `stripconf.rs` | config proužku (`StripConfig` vč. `look`/`lookHide`, `StripItem`, `StripColor`; re-export z `config/*`), validace `strip.custom`, čisté skládání: vlastní text, „v klidu skryté“ (`is_event`), barva na položku, automatická položka časovače | `StripConfig`, `StripItem`, `StripColor::base`, `compose`, `is_event`, `custom_segment`, `effective_items`, `validate`, `CUSTOM_MAX` |
| `timer.rs` | časovač: odpočet + stopky jako časy (žádný tik), akce `Op` → `apply`, `tick` (doběhnutí → `attention::signal` + toast `timer.toast`), `next_deadline` (konec, změny minuty v proužku), segment proužku, stav v `timer.json` (zápis jen při akci), config `TimerConfig` | `TimerState`, `Countdown`, `Stopwatch`, `Op`, `apply`, `tick`, `next_deadline`, `strip_segments`, `minutes_text`, `clock_text`, `act`, `tick_live`, `next_deadline_live`, `init`, `view`, `TimerView` |
| `attention.rs` | pozornost Clauda: počet čekajících session (segment `waiting`, `upgrade_items` pro starý config), přechody „čeká na tebe“ / „hotovo“ ze snímků mostu, dedup 30 s, rozhodnutí bliknutí / puls / toast, tvar pulsu; `on_change` volá `refresh::on_claude_change`; `signal` = bliknutí + puls podle `react` i pro jiné zdroje (konec časovače, schůzka za chvíli — `Event::Meeting`, GitHub — `Event::GithubCi` / `GithubReview` z `suggest::pulse` bez toastu, priorita `Event::rank`: čeká > schůzka > spadlé CI > PR k revizi > hotovo) | `Event::rank`, `waiting_count_all`, `waiting_text`, `upgrade_items`, `ITEMS_REV`, `Seen`, `transitions`, `Dedup`, `Gate`, `react`, `signal`, `pulse_level`, `PULSE_MS`, `pulse`, `on_change` |
| `toast.rs` | oznámení Windows přes WinRT (`windows` crate, `UI_Notifications`), krátké vlákno; QUNS „přijímá oznámení?“; tlačítka s aktivací protokolem (Připojit ke schůzce) | `show`, `show_xml`, `toast_xml`, `toast_xml_actions`, `xml_escape`, `accepts_notifications` |
| `tray.rs` | ikona v oznamovací oblasti (Tauri `tray-icon`, bez vlákna a pollingu): levý klik = otevřít šuplík (`gamemode::handle(Relaunch)`, fokus smí — klik je výslovný), nabídka Otevřít · Nastavení · Ukončit Ledge; ukončení = `app.exit(0)` → `RunEvent::Exit` (`stop_all`, `pins::win::flush`), tutéž cestou jako aktualizace; popisky z `ui.lang`, po změně jazyka `relabel` z `settings::apply_config` | `init`, `relabel`, `action_for`, `texts`, `Action` |
| `gamemode.rs` | herní režim: „je Ledge mrtvý?“, rozhodnutí podnětů (zkratky, druhé spuštění), stav v `game-mode.json` vedle configu | `is_dead`, `decide`, `Trigger`, `Outcome`, `handle`, `set`, `load`, `save`, `FILE` |
| `refresh.rs` | obnovy na pozadí + reakce na události (počasí jen s kartou na otevřené záložce, `layout::page_has`) | `maybe_refresh_weather`, `reclassify_links`, `refresh_logos`, `refresh_git_async`, `on_claude_change`, `on_proc_event` |
| `hotkeys.rs` | globální zkratky | `sync_hotkeys`, `sync_approve_keys`, `on_shortcut` |
| `settings.rs` | config za běhu + vlákno `config-watch`; nový uživatel → `onboarding::fresh_config` | `load_or_seed`, `apply_config`, `watch_config` |
| `ctl.rs` | most `/status` doplněk a `/ctl/*` (open, close, settings, launch, stop, reload) | `status_extra`, `ctl_handler` |
| `threads/ui.rs` | vlákno `ui`: sériově `UiCmd` → `Drawer` | `spawn_ui_thread` |
| `threads/emit.rs` | vlákno `emit`: stav do šuplíku, proužek, připomínky, git | `spawn_emit_thread`, `next_wake`, `GIT_EVERY` |
| `threads/hover.rs` | vlákno `hover`: myš, fullscreen, lištové okno, tlak → otevřít/zavřít; zóna u hrany = `looks::frame::zone` (tvar proužku, ořez na monitor, skrytý proužek = pruh na ose); tah souborů na proužek → `shelf::Arm` / `shelf::drop::set_armed` ([shelf.md](shelf.md)) | `spawn_hover_thread` |
| `threads/notes_watch.rs`, `notes_sync.rs` | vlákna poznámek (watcher, git sync) — viz [notes.md](notes.md) | `NotesState`, `reload_notes`, `check_reminders`, `SyncMsg`, `spawn_notes_sync` |
| `commands/*.rs` (`commands/{config,pkgctx,timer,update}.rs`) | Tauri příkazy po oblastech: `drawer` (stav, režim, fokus, odkazy, `expand_drawer` z náhledu po najetí), `projects` (launch, open_in, `git_pull`, klon), `notes`, `sessions` (decide, hooky), `config`, `update`, `pkgctx` (`pkg_context`, `relaunch_outside`) | `git_pull` + `pull_message`, `focus_in_own_popup`, `open_link`, `update_install` |
| `focusreturn.rs` | návrat fokusu do předchozí aplikace: `remember` v `Drawer::focus` (těsně před převzetím), `give_back` v `Drawer::hide` / po Full → peek; jen po `UiCmd::Close`, jinak `on_close(false)` zapomene. Čisté `decide`/`keep_or_take` (tabulky) | `remember`, `on_close`, `give_back`, `decide`, `keep_or_take`, `Remembered`, `Probe` |
| `drawer.rs` | okno šuplíku (WebView2), argumenty Chromia, handshake morfu, suspend, sklo, pozice u hrany (`appearance`); pohledy `View` = Hidden / Peek / Full / Compact / Island (náhledy po najetí), `View::covers_strip` = panel, který zavírá odjetí myši; velikosti malých pohledů `hovermode::SmallSizes`, `ViewMsg.expand_ms` | `Drawer::{show, hide, warm, ack, set_dormant, take_focus, release_focus, destroy_if_idle, set_anchor, set_expand_ms}`, `BROWSER_ARGS`, `View`, `Opener`, `LABEL` |
| `hovermode.rs` | co se stane po najetí (`drawer.hover`: `full` výchozí / `compact` / `island`, `drawer.hoverExpandMs`), čisté funkce + tabulky: cíl najetí (`hover_target`: Claude čeká → peek ve všech režimech, podržení nad připomínkou → Full, jinak pohled režimu), zavření (`after_close`: s čekajícím Claudem Full/Compact/Island → peek), ořez prodlevy a velikostí, pohled podle jména pro `/ctl/open` | `HoverMode`, `hover_target`, `after_close`, `expand_ms`, `EXPAND_MS`, `SmallSizes`, `fit_small`, `view_by_name` |
| `strip.rs` | nativní proužek: vrstvené okno, D2D, click-through, vlákno `strip`; vzhled z `appearance::geom` ([appearance.md](appearance.md)); puls (`StripModel.pulse`) animuje `WM_TIMER` jen po dobu pulsu (týž časovač ~240 ms při změně šířky ostrůvku, `ROTATE_TIMER` 5 s jen u `ticker` s víc událostmi); `StripModel.show` → `stripdraw::draw`, jinak klasický `draw`; překryv přihrádky `StripModel.shelf` → `shelf::overlay` ([shelf.md](shelf.md)) | `Strip::{start, set}`, `StripModel`, `Segment`, `Rgba`, `pulse_now`, `PULSE_TIMER`, `ROTATE_TIMER` |
| `edge.rs` | čistá logika „otevřít myší?“ (tabulkové testy) | `armed`, `PushTracker`, `BarLatch`, `is_bar_window`, `edge_passes_through`, `Delays` |
| `rawpush.rs` | raw myš (tlak na hranu), vlákno `rawpush`, aktivní jen u hrany | `RawPush::{start, arm, take}` |
| `screen.rs` | monitory, DPI, kurzor, fullscreen (QUNS + okno v popředí) | `monitors`, `pick`, `fullscreen_state`, `FsState`, `ForegroundProbe` |
| `procman.rs` | dev servery / appky v Job Objectu, URL z logu, porty | `ProcMan::{launch_dev, launch_app, stop, stop_all, snapshot}`, `Phase`, `ProcEvent` |
| `config/*` (`config/{claude,drawer,hotkeys,io,personal,project,sections,validate}.rs`) | `config.json` + schéma, validace (s polem: `validate_field` → `configerr::FieldError`), zkratky; `Sections` (`drawer.sections`) generuje makro `sections!` — plochý seznam boolů ODVOZENÝ z `layout` (`layout.rs` `derive`, → [layout.md](layout.md)) | `Config`, `Sections`, `sections!`, `load`, `save`, `validate`, `validate_field`, `active_hotkeys`, `rebind_plan`, `config_dir` |
| `src-tauri/src/config/validate.rs` | `validate` / `validate_field` celého configu: první chyba s cestou k poli (`FieldError`), volá validace sekcí (`smartconf::validate`, poznámky, zkratky, projekty) | `validate` `validate_field` |
| `drawerfit.rs` | strop výšky šuplíku (80 % pracovní plochy monitoru) a širší Full pro Nastavení → [settings.md](settings.md) | `limits`, `fit_full` |
| `autostart.rs` / `migrate.rs` | HKCU Run (míří na instalovanou kopii; v bublině MSIX nic — `gate`) / Notch → Ledge při startu | `apply`, `gate`, `decide`, `install_target` / `at_startup`, `plan_notes` |
| `pkgctx.rs` | bublina MSIX (Ledge spuštěný z Claude desktop): identita `GetCurrentPackageFullName`, jinak sonda virtualizace `%LOCALAPPDATA%` → `Packages\*\LocalCache`; jednou za proces, log při startu; fyzická kopie virtualizované cesty; `--after-pid` (nová instance čeká na starou, `run()` před builderem) | `ctx`, `current`, `detect`, `classify`, `combine`, `probe_virtualization`, `bubble_twin`, `twin_of`, `relaunch_target`, `after_pid`, `wait_for_predecessor`, `view` |
| `outside.rs` | spuštění MIMO bublinu: jednorázová úloha Plánovače z XML (`InteractiveToken`, `PT0S`, priorita 5), `After::Delete` po naběhnutí / `Keep` (instalátor), záloha `explorer.exe`; úklid `Ledge-outside-*` po startu | `task_xml`, `run_task`, `run_explorer`, `cleanup_stale`, `cleanup_later`, `TASK_RELAUNCH`, `TASK_UPDATE` |
| `update.rs` / `updatenet.rs` | nová verze: verze, rozbor vydání (veřejné API, záložně `gh`), ověření SHA256, stažení do cache (`.part` → ověřit → manifest), opětovné ověření instalátoru z cache, spuštění instalátoru (v bublině MSIX `/S /R` přes `outside::run_task`, cesta z cache přes `pkgctx::twin_of`); test hlídá `allowDowngrades` v `tauri.conf.json` / HTTP dotaz (`latest`, `by_tag`), stažení s průběhem ze skutečných bajtů | `evaluate`, `evaluate_tag`, `release_of`, `verify`, `download_and_verify`, `verified_cached`, `launch_installer`, `fetch_release`, `fetch_release_tag` / `latest`, `by_tag`, `download`, `download_ok`, `Progress` |
| `updater.rs` | stav aktualizací a vlákno `update` (spí na kanálu, žádný časovač navíc): plán kontrol, po kontrole tiché stažení (`update.prefetch`, jen se zapnutou kontrolou, jednou na verzi, selhání čeká na „Zkusit znovu“), instalace na klik (z cache), návrat k předchozí verzi, při startu `housekeeping` (seen.json, úklid cache a starého `%TEMP%\ledge-update`); pohled `Snapshot.update` (`fetch`, `whatsNew`, `rollback`) | `Updater::{start, check_now, retry_fetch, mark_notes_read, install, rollback, view}`, `prefetch`, `fetch_now`, `refresh_cache`, `housekeeping`, `auto_fetch`, `UpdateView`, `WhatsNew`, `RollbackView` |
| `updcache.rs` | cache instalátorů `%LOCALAPPDATA%\Ledge\updates\<tag>` (manifest `release.json` se SHA256 a poznámkami): čisté funkce — co uklidit, kam se vrátit (z cache / stažení podle tagu / nikam), kdy obnovit průběh, co udělat po kontrole | `Manifest`, `Entry`, `scan`, `cached`, `of_version`, `plan_rollback`, `Rollback`, `plan_cleanup`, `remove`, `progress_due`, `Fetch`, `next_step`, `Step` |
| `updseen.rs` | `updates\seen.json`: naposledy spuštěná verze, předchozí (cíl návratu), přečtené „Co je nového“ a SHA256 dřív ověřených instalátorů podle verze (návrat odmítne přepsané vydání); zápis jen při změně verze, po ověřeném stažení a po „Rozumím“ | `Seen`, `Known`, `on_start`, `unread`, `mark_read`, `remember`, `check_known`, `load`, `save` |
| `actions.rs` · `actionrun.rs` · `actionicons.rs` · `actionstate.rs` | rychlé akce → [actions.md](actions.md) | `Action`, `run_action`, `actionstate::refresh`, `views` |
| `pins/*` | lístky na plochu — okna `note-<klíč>` s UI Ledge (`pin.html`), stav `note-pins.json` → [notes.md](notes.md) | `win::open`, `win::sync_dead`, `win::restore_all` |
| `desk/*` | widgety na ploše — okna `desk-<id>` s UI Ledge (`desk.html`), stav `desk.json`, data `desk-data` z vlákna `emit` jen viditelným → [desk.md](desk.md) | `win::pin`, `win::sync_yield`, `live::tick` |
| `webpanel.rs` | okno s webem `web-<id>` (rychlá akce `kind: "web"`): normální okno s rámem, sdílí WebView2 se šuplíkem, zavření = zničení → [actions.md](actions.md) | `open`, `nav_allowed`, `frame_allowed`, `place` |
| `personal.rs` / `weather.rs` | rychlé odkazy (nedůvěryhodný cíl) / Open-Meteo | `classify_fs`, `LinkTarget` / `fetch`, `needs_fetch`, `loading_live`, `status` |
| `cliphist/*` | historie schránky (vlákno `cliphist` jen se zapnutým `clipboard.enabled`) → [clipboard.md](clipboard.md) | `sync`, `paste` |
| `media.rs` · `volume.rs` · `battery.rs` | Osobní: právě hraje, hlasitost, baterie → [media.md](media.md) | `ensure`, `sync`, `view` |

## Vlákna a buzení

- `emit` spí na `Core.wake` (`mark_dirty`) s timeoutem `next_wake`: začátek další minuty (hodiny),
  `NotesState.next_due`, konec `FLASH`, git za `GIT_EVERY`, watchdog počasí, reset limitu Claude na 100 %
  (`limitslive::next_deadline`, [limits.md](limits.md)), časovač (`timer::next_deadline_live` — jen když běží:
  konec odpočtu a změna minuty v proužku, ~60/h navíc; pozastavený / žádný nic). V klidu ~80 probuzení/h.
  `hover` ho budí i po změně fullscreenu (peek připomínky).
- `hover`: 16 ms u hrany / 100 ms daleko / 500 ms ve fullscreenu; co 3 s `monitor_fresh` + výčet monitorů.
- `ui`, `notes-watch`, `notes-sync`, `config-watch`, `strip`, `rawpush`, `update`, `media` a `volume` (vzniknou až při prvním zobrazení záložky Osobní; `media` i dřív, je-li v proužku položka `media`, [media.md](media.md)), `shelf-drop` (vznikne až
  při prvním tahu souborů na proužek, [shelf.md](shelf.md)), `cliphist` (jen se zapnutou historií schránky, [clipboard.md](clipboard.md)): blokují na kanálu / událostech. `strip` má časovač (~30 fps) jen během pulsu `attention::PULSE_MS` (1,2 s) a ~240 ms po změně šířky ostrůvku, střídání `ticker` po 5 s jen při víc událostech; v klidu žádný;
  `toast` je jednorázové vlákno na jedno oznámení. `pins-save` (zpožděný zápis stavu lístků na plochu) vznikne s
  prvním lístkem a spí na kanálu; `desk-save` totéž pro widgety na ploše ([desk.md](desk.md)). `logos`, `links`, git (`refresh_git_async`) jsou krátká jednorázová vlákna.

## Invarianty a pasti

- **Bublina MSIX** (`pkgctx`): procesy z Claude Code session nemají identitu balíčku, a přesto jsou
  virtualizované (změřeno) → API samo nestačí, rozhoduje i sonda. V bublině `autostart::apply` registr
  nechá být; ven jen přes Plánovač úloh (`outside`), ne přes `Command::spawn` (dítě zdědí kontejner).
  `relaunch_outside` i aktualizace končí `app.exit(0)` (s kódem → `ExitRequested` neblokuje).

- **Fokus:** jen `UiCmd::Focus` z výslovného kliku (`focus_drawer`); hover/peek/připomínka/náhled po najetí nikdy.
- **Náhled po najetí** (`hovermode`): vlákno `hover` rozhodne pohled (`hover_target`, jediné místo v
  `threads/hover.rs`), Compact/Island zavírá odjetí myši stejně jako Full otevřený najetím; do celého šuplíku
  `expand_drawer` (klik / setrvání z frontendu) se stejným openerem → bez fokusu. Show Compact/Island nic
  drahého nespouští (git, logy, sessionlive, poznámky, počasí a kalendář jen pro Full). `/ctl/open` bere
  `view: "compact" | "island"` (živé ověření bez myši).
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
  JS v rendereru: hlavní chunk šuplíku ~430 kB (Nastavení, průvodce, paleta, Poznámky, angličtina jsou líné chunky,
  [ui.md](ui.md) `lazyViews.tsx`). Úsporný režim je volitelný a už zapojený: `drawer.keepWarmS` (Chování → Spouštění a paměť
  → „Šetřit paměť“, výchozí 0 = vypnuto) → vlákno `hover` volá `Drawer::destroy_if_idle`.

## Testy

`hovermode` (cíl najetí, zavření do peeku, `covers_strip`, prodleva, velikosti, starý i nový config přes `config::load`), `attention` (počet a text čekajících, `upgrade_items` i přes `config::load`, přechody, dedup, `react`,
tvar pulsu), `stripconf` (serde starých/nových configů, migrace přes `config::load`, vlastní text, `is_event`,
`compose` klid/barvy, automatický časovač, čitelnost předvoleb ve všech motivech), `timer` (akce odpočtu a
stopek, `tick`, termíny, texty, segmenty, tvar `Op`, config, soubor v tempdiru), `gamemode` (`is_dead`, `decide`, stav v tempdiru), `toast` (XML), `threads::emit` (`next_wake`, hodina klidu), `state` (`MonitorCache`; `monitor_cache_bench` je
`#[ignore]`), `commands::projects` (`pull_message`), `threads::notes_watch` (filtr událostí),
`looks` (tabulka 11 stylů × 6 stavů, priorita, Pokročilé, geometrie a hover zóny, config, zrcadlo TS), `edge`, `config`, `procman`, `autostart`, `migrate`, `personal`, `weather`, `update`, `focusreturn`
(`decide`, `keep_or_take`), `paletteconf`.

Nečti: `claude/` (most, viz [sessions.md](sessions.md)), `notes*` ([notes.md](notes.md)),
`icons.rs`/`cloning.rs` ([projects-icons-perf.md](projects-icons-perf.md)).
