# Codemapa: rozšíření `ledge.exe --ext <id>` (kolo 33, E0 kostra)

Rozšíření = samostatný proces, **dítě hlavního Ledge** (tapeta, prohlížeč…). Ledge ho spouští a hlídá, ono si drží
vlastní okno a RAM. Bez `extensions` v configu (výchozí `[]`) neběží **žádné vlákno ani časovač** a `ext::on_dead_changed`
jen přečte jedno `OnceLock`. Hotovo: E1 měření ceny a blok Rozšíření v Nastavení, E2 okno tapety (`ext/host`, `ext/wall`), E3 scéna (`src/ext/wall/**` → `wall.html`), E4 perf skripty, kolo 34: pozadí z fotky, obal alba, pauza při zamčení a úspora na baterii.

| soubor | účel | klíčové symboly |
|---|---|---|
| `ext/mod.rs` | registr `REGISTRY` (manifesty), napojení na aplikaci `LiveEnv`, líný singleton supervisoru, háčky pro zbytek kódu | `REGISTRY` `LiveEnv` `wire_up` `on_config` `on_dead_changed` `notify` `wake` `status` |
| `ext/conf.rs` | config `extensions: string[]` (transparentní newtype, výchozí `[]`, neznámé id se ignoruje) | `ExtConfig` |
| `ext/wire.rs` | NDJSON `v:1`: zprávy `Up` (ext→Ledge) a `Down` (Ledge→ext), kódování, dekódování, limit řádku 256 KB | `Up` `Down` `decode_up` `decode_down` `encode_up` `encode_down` `read_line` `WireError` `MAX_LINE` |
| `ext/policy.rs` | čistá rozhodovací logika (tabulkové testy): restart, nečinnost, pauza, životní fáze, plán synchronizace, koalescence, whitelist `req` | `Manifest` `restart_delay` `record_crash` `exit_kind` `after_exit` `idle_deadline` `pause_deadline` `pause_reason` `Life` `plan_sync` `Coalescer` `parse_req` `Timing` |
| `ext/sup.rs` | supervisor: spuštění v Jobu, handshake, vlákna běhu, pauza, data, termíny, restart; rozhraní `Env` (maketa v testech) | `Sup` `Env` `ExtStatus` `Launcher` |
| `ext/child.rs` | strana rozšíření: `main` (`--ext`), `run_with` (hello → init → `Body::run` → bye), mutex `Local\ledge-ext-<id>`, registr těl `body_for` | `Body` `Io` `Out` `InitData` `run_with` `body_for` |
| `src/types/ext.ts` | zrcadlo configu `extensions` (`ExtensionsConfig`, výchozí `[]`); sekce Zálohy `settings/backupModel.ts` | `ExtensionsConfig` `EXTENSIONS_DEFAULT` |
| `ext/measure.rs` | poctivá cena: RAM (private bytes) a CPU % celého Jobu přes `QueryInformationJobObject` + `GetProcessMemoryInfo`, potomci mimo Job (záloha); čisté `cpu_pct` `next_sample` `job_sum` `descendants` `merge_pids`; měří jen při volání, bez časovače | `Cost` `measure` `forget` `cpu_pct` `next_sample` |
| `commands/ext.rs` | příkazy Studia `ext_status` (stav + `Cost`) a `ext_wall_bg` (`get`/`pick`/`clear` pozadí tapety; dialog běží v jádře, UI cestu neposílá) | `ext_status` `ext_wall_bg` |
| `src/ext/{api,model}.ts` `ExtBlock.tsx` | blok Rozšíření na stránce Spouštění a hry: stav, RAM, CPU (poll 5 s jen s viditelným blokem), Zapnout/Vypnout zapisuje `extensions`, řádek Pozadí tapety (`WallBg`); čistá logika stavu a formátu v `model.ts` (`fmtRam`, `fmtCpu` — bez měření „—“) | `extApi` `extState` `fmtRam` `fmtCpu` `withExt` `ExtBlock` |
| `ext/echo.rs` | testovací rozšíření `echo` (jen `cfg(test)`): handshake a smyčka bez okna, zapisuje přijaté zprávy do souboru | `Echo` |
| `ext/tests.rs` | integrační testy supervisoru nad skutečným procesem (testovací binár znovu spuštěný jako dítě) | `child_entry` |
| `ext/host/win.rs` | sdílené Win32: třída okna s obsluhou v closure, smyčka zpráv s vlákovými zprávami, `pump_until` (čekání na WebView2), DPI | `create_window` `message_loop` `pump_until` `post_thread` `init_dpi` `class_name` |
| `ext/host/webview.rs` | WebView2 bez Tauri: prostředí s vlastní datovou složkou, `Page` (načtení HTML, zprávy stránka↔Rust, `TrySuspend`) | `create_env` `Page` `BROWSER_ARGS` |
| `ext/wall/mod.rs` | tělo `wall`: strom Progman/WorkerW, okno-dítě, fronta událostí z Ledge a stránky, `TaskbarCreated` watcher, pauza | `Wall` `State` `create_watcher` `wants_reattach` `run_wall` |
| `ext/wall/attach.rs` | čisté: kam okno připnout a co s ním (strom oken jako fakta) | `choose_parent` `attach_plan` `Tree` `Action` `ParentKind` |
| `ext/wall/geom.rs` | čisté: obdélníky, `rel_rect` (poloha dítěte, záporný levý monitor), volné místo pro sondu | `R` `rel_rect` `uncovered_rects` `pick_probe` |
| `ext/wall/bg.rs` | pozadí z vlastní fotky: kontrola zdroje (`check_src`: lokální absolutní cesta, jpg/jpeg/png/webp z `personal::FILE_ALLOW`, ≤ 40 MB), WIC zmenšení na ≤ 1920 px → kopie `ext/wall/bg.jpg`; rozšíření čte jen kopii a posílá ji stránce jako `data:` URL | `check_src` `set_from` `clear` `name` `load_url` |
| `ext/wall/env.rs` | prostředí tapety bez pollingu: zamčení stanice (`WTS`) a zdroj napájení jako události skrytého okna watcheru → `hidden` (pauza) / `eco` (baterie); čisté `lock_change` `on_battery` `battery_from_source` `hidden` | `Flags` `subscribe` `handle` |
| `ext/wall/diag.rs` | `--ext wall --diag`: 60 s event-driven log viditelnosti / rodiče / z-order do `diag.log` | `start` `set_target` `note` |
| `ext/wall/wall.html` | GENEROVANÝ jeden soubor scény (SUNDAY, hodiny, skleněný přehrávač); neupravovat ručně, `node scripts/wall-build.mjs` | — |
| `src/ext/wall/model.ts` | čistá logika scény: dny/data cs+en, minutový časovač, `barPlan` z MediaView, kontrast textu, `photoPlan` (ztmavení fotky pro kontrast ≥ 4,5), `coverPlan` (obal alba) | `formatDay` `formatDate` `msToNextMinute` `barPlan` `pickInk` `playerView` `photoPlan` `textRatio` `coverPlan` `isImageUrl` |
| `src/ext/wall/scene.html` | zdroj scény: CSS, značky, lepidlo zpráv; za `/*@MODEL@*/` se vloží `model.ts` | — |
| `scripts/wall-build.mjs` | složí `scene.html` + `model.ts` → `wall.html` (`--check` ověří aktuálnost) | `buildWall` `modelJs` |
| `ext/wall/live.rs` | živá zkouška (`#[ignore]`, `LEDGE_WALL_LIVE=1`): guard, sonda volné plochy, pixely, RAM/CPU stromu | `live_attach_renders_in_free_probe_rect` |
| `scripts/perf.ps1` `perf-lib.ps1` `perf-wall.ps1` | rozpočet Ledge bez rozšíření (`Get-LedgeGroups`), zvlášť rozšíření; `perf-wall` měří tapetu klid/pauza s guardem a Job vs. strom PID; test `perf-lib.test.mjs`; checklist pro šéfa v ext-detail.md | `Get-LedgeGroups` `Get-ExtId` `Test-MeasureGuard` |
| `procman/appstop.rs` | Stop aplikace bez jobu: otisk (PID + čas startu + exe) → `WM_CLOSE` → po 3 s kill, cizí PID nikdy | `close_then_kill`, `AppIdent` |
| `procman/job.rs` | `Job` (Job Object s KILL_ON_JOB_CLOSE, `spawn` v pozastaveném stavu, `kill`, `raw` pro měření) vyčleněný z `procman.rs` | `Job` `CREATE_NO_WINDOW` |

Detail (toky, invarianty, jak přidat rozšíření): [ext-detail.md](ext-detail.md).
