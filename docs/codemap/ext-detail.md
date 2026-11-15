# Rozšíření — detail kontraktu (kolo 33, E0)

Jádro mapy: [ext.md](ext.md).

## Jak se rozšíření registruje (pro E1 / E2 / E5)

1. Řádek `Manifest` v `ext::REGISTRY` (`id` malými písmeny / číslicemi / `-`, `Start::Always|OnDemand`, `idle_quit_min`,
   `pause_quit_min`, `wants` = témata dat). Test `registry_ids_are_unique_and_safe`.
2. Tělo v `ext/child.rs` `body_for(id)`: `impl Body { wants(); run(Io) -> i32 }`. `run` běží na hlavním vlákně procesu
   (okno potřebuje vlastní smyčku zpráv); `Io.rx` = zprávy Ledge→ext (při EOF stdin přijde syntetické `Down::Quit`),
   `Io.out` = zápis ext→Ledge (`ready()` až když okno vykresluje, `active()`, `err()`, `send(Up::Req…)`).
3. Zapnutí: `extensions: ["<id>"]` v configu. `Always` se spustí 3 s po startu / hned po změně configu; `OnDemand`
   jen `ext::wake(id, reason)`.
4. Data: `ext::notify("media")` z místa, kde se zdroj mění (E2 zapojí `media.rs`); supervisor si téma natáhne přes
   `Env::topic_data` (nová témata = větev v `LiveEnv::topic_data`).

## Drát (`wire.rs`)

NDJSON, jeden JSON na řádek, `v:1`. ext→Ledge: `hello{id,pid,wants}` · `ready` · `active` · `req{what,do?,key?}` ·
`err{msg}` · `bye`. Ledge→ext: `init{cfg,lang,dead,rev}` · `state{pause,why?}` · `data{topic,rev,d}` · `cmd{name,args}` ·
`quit`. Neznámý typ zprávy = zahodit; jiná `v` = `Version` (Ledge pošle `quit`, kill po lhůtě, fáze `Incompatible`);
řádek > 256 KB = `TooLong` (kill); nesmysl na stdout (log knihovny) se zahodí. stdout patří jen protokolu, logy na stderr
(sbírá se posledních 40 řádků → `ExtStatus.tail` po pádu).

## Životní cyklus (`policy::Life`, `sup::run_once`)

`Spawned` → `hello` do 5 s → `init` poslán (`Greeted`) → `ready` do 15 s → `Ready`. Jedna smyčka na slot:
`recv_timeout(nejbližší termín)` — termíny jsou handshake, `idle_deadline`, `pause_deadline`, po `quit` lhůta 3 s
(pak `Job::kill`), odložená data. Žádný ping ani tik. Konec: čekač na proces (EOF stdout nestačí, WebView2 dítě
může držet roury) → `exit_kind` → `after_exit`: čistý konec = Idle; pád `OnDemand` = Idle s chybou; pád `Always` →
`restart_delay` (2 s / 10 s / 60 s, 4. pád v 10 min = `Failed`, dál se nezkouší, dokud ho uživatel nevypne/nezapne).
`ready` + 60 s běhu počítadlo pádů nuluje (`record_crash`).

## Hra → pauza

`threads/hover.rs` (změna fs / kind) a `gamemode::set` volají `ext::on_dead_changed()` → `Cmd::Pause(Some("game"))`
→ `state{pause:true,why:"game"}`, fáze `Paused`, data nejdou; po hře `pause:false` + jeden čerstvý snapshot všech
témat a `sync` (znovu spustí `Always` rozšíření, která mezitím skončila po `pause_quit_min`). `wake` ve hře odmítne.

## Data dolů (`Coalescer`)

Posílá se jen při změně otisku (`desk::live::fnv` nad JSON hodnoty), nejvýš 1× za 1 s na téma, poslední hodnota vyhrává;
v pauze nic. `rev` je čítač na téma a běh.

## Požadavky `req` (whitelist `policy::parse_req`)

`cover{key}` (`media::cover` → PNG ≤ 128 px jako data URL; odpověď jde dolů jako `cmd{cover,{key,url}}`, vlákno mimo smyčku supervisoru, jiný klíč než aktuální skladba = nic) · `media{do: toggle|next|prev}` (`media::control`) ·
`open_settings`. Cokoli jiného se zahodí a zaloguje; žádný `req` nevede k `/ctl/decide` (povolit akci Clauda smí jen
člověk) — tabulkový test `req_whitelist_table`.

## Měření (E1)

`ext/measure.rs`: `ext::status_measured()` (příkaz `ext_status`) změří jen při volání z UI — Nastavení → Spouštění a hry volá po 5 s, dokud je blok vidět a okno není skryté. RAM = součet `PrivateUsage` přes PID z Jobu (+ potomci hlavního PID mimo Job; kolik jich je, hlásí `Cost.outsideJob`), CPU = Δ `TotalUserTime+TotalKernelTime` Jobu / Δ čas / jádra (% celého stroje jako `perf.ps1`); první vzorek a odstup < 1 s = `null` → „—“. GPU se neukazuje. Neběží = `cost: null`.

Podprocesy WebView2 v Jobu: ověřit přes `Cost.outsideJob` (0 = ano) nebo `perf-wall.ps1` (job vs. strom PID; sekce Výkon níže).


Při `RunEvent::Exit` (`lib.rs`) volá `ext::shutdown()` → `Sup::shutdown(1,5 s)`: aktivním pošle `quit` a počká, až odejdou (řádné zavření WebView2 profilu); co nestihne, zabije až Job.
`ext::job(id)` vrací `Arc<procman::Job>` běžícího rozšíření; `Job::raw()` je handle pro `QueryInformationJobObject`.
`ext::status()` vrací `ExtStatus` všech manifestů (`phase`, `pid`, `error`, `tail`, `pauseWhy`).

## Testy

- čisté: `ext/policy.rs`, `ext/wire.rs`, `ext/conf.rs` (starý config bez klíče → `[]`);
- `ext/child.rs` — handshake nad Cursorem a pamětí (bez procesu);
- `ext/tests.rs` — skutečný proces: testovací binár se spustí znovu s `--exact ext::tests::child_entry` + `LEDGE_EXT_TEST_ID`
  (bez proměnné je `child_entry` prázdný test); supervisor běží s maketou `Env` v dočasné složce.

## Tapeta `wall` (E2)

`ledge.exe --ext wall`: WebView2 jako dítě `WorkerW` přes **primární monitor** (spike 26200: dítě `WorkerW` i
Progmanu se vykreslí 96 %, GDI/D2D děti ne). `run_wall` (hlavní vlákno): `create_env` (data dir
`%LOCALAPPDATA%\Ledge\ext\wall\wv2`, `--disable-features=CalculateNativeWinOcclusion`) → skryté top-level okno
`LedgeWallWatcher` (ne message-only, dostává `TaskbarCreated`, `WM_DISPLAYCHANGE`, časovač opakování) → most
`Io.rx` → fronta + `PostThreadMessage` → `State::sync`: `ensure_workerw` (jednou `0x052C`, když `WorkerW` chybí)
→ strom jako fakta `attach::Tree` → `attach_plan`: `Reuse` (jen přepočet polohy) · `Reparent` (`SetParent`) ·
`Recreate` (Progman zahynul s oknem; nové okno + WebView) · `Fallback` (Průzkumník neběží; opakování po 2 s
jen dokud chybí). Poloha = `geom::rel_rect(primární monitor, počátek klientské oblasti rodiče)`. `ready` po
prvním načtení scény. Pauza (`state{pause}`): `SW_HIDE` + `SetIsVisible(false)` + nízký cíl paměti +
`TrySuspend`; po resume zpět a čerstvá data od Ledge. Okno je `WS_EX_NOACTIVATE`, `WM_MOUSEACTIVATE` →
`MA_NOACTIVATE`; WorkerW-dítě stejně myš nedostává. Záloha `IDesktopWallpaper` = TODO fáze 1b (mění tapetu).

**Scéna (E3):** zdroj `src/ext/wall/scene.html` (CSS + značky + lepidlo) a `model.ts` (čisté funkce); `node scripts/wall-build.mjs` je složí do `ext/wall/wall.html` (neupravovat ručně; `model.test.mjs` hlídá aktuálnost, `--check`). Vzhled F8 a/b: pozadí statický přechod `BG_STOPS` (bez animace mlhy) nebo vlastní fotka (viz níže), `SUNDAY` Light s trackingem, datum + čas (jediný `setTimeout` na další minutu, nic při `hidden`), skleněný přehrávač (`backdrop-filter`) jen s hudbou (`playerView`), obal = skutečný obal z `req cover` (bez něj neutrální plocha, nic vymyšleného), průběh = jedna CSS animace `scaleX` do konce skladby (`barPlan`), barva textu z kontrastu vůči pozadí (`pickInk`). Tlačítka posílají `req media`; myš do WorkerW-dítěte nejspíš nedojde (neověřeno), ovládání později přes desk okno. Náhled bez Rustu: otevřít `wall.html` v prohlížeči a `window.__wallMsg({t:'init',lang:'en'})` + `{t:'data',topic:'media',rev:1,d:{...}}` (hook jen bez WebView2). Formát: cs `NEDĚLE` / `15. PROSINCE 2024`, en `SUNDAY` / `15 DECEMBER, 2024`, čas 24 h.

**Pozadí z vlastní fotky (kolo 34):** Nastavení → blok Rozšíření → „Pozadí tapety“ (jen při zapnuté tapetě) volá `ext_wall_bg` (`pick`): dialog Otevřít běží v jádře (UI cestu neposílá), `ext/wall/bg.rs` `check_src` pustí jen absolutní lokální cestu (UNC `\\server` ne), příponu jpg/jpeg/png/webp (podmnožina `personal::FILE_ALLOW`) a ≤ 40 MB; `iconimg::to_jpeg` (WIC) zdroj ověří dekódováním a zmenší na ≤ 1920 px (vstup > 100 Mpx se odmítne) → kopie `%LOCALAPPDATA%\Ledge\ext\wall\bg.jpg` + `bg.txt` (název). Rozšíření čte jen kopii (pevné jméno, kontrola magic bytes JPEG a ≤ 6 MB) a posílá ji stránce jako `{t:'bg',url:'data:image/jpeg…'}` (po `init` a po `cmd bg` z jádra `ext::wall_bg_changed`); žádné `file://`. Stránka vzorkuje 32×18 px náhled (`rgbSamples` → `repColor` 85. percentil jasu) a `photoPlan` spočítá nejmenší černou vrstvu (0,2–0,85), při které má i nejslabší text (krytí .7) kontrast ≥ 4,5; fotka `cover`, vrstva statická (žádná animace). Bez fotky / po „Odebrat“ zpět přechod.

**Obal alba:** stránka při změně `MediaView.coverKey` (`coverPlan`) pošle jednou `req cover{key}`; Ledge odpoví `cmd{cover,{key,url}}`, stránka přijme jen shodný klíč a jen `data:image/png` (`isImageUrl`).

**Pauza a úspora (kolo 34):** fullscreen okno / hra / herní režim = existující `Core::dead` → `state{pause,why:"game"}` (jádro, beze změny). Zamčená obrazovka a baterie nejsou v jádře — `ext/wall/env.rs` je přijímá jako události skrytého okna watcheru (`WTSRegisterSessionNotification`, `RegisterPowerSettingNotification` `GUID_ACDC_POWER_SOURCE`), žádný polling ani časovač. Zamčeno = totéž co hra (`SW_HIDE` + `SetIsVisible(false)` + `TrySuspend`, stránka `hidden` = žádný časovač ani animace). Baterie = plocha je vidět, takže se neskrývá; stránka dostane `eco` (bez `backdrop-filter`, bez animace průběhu a bez lišty). `MemoryUsageTargetLevel LOW` platí od připojení stránky trvale (statická scéna), ne jen v pauze. Cíl RAM < 120 MB v klidu je ODHAD (WebView2 1280×720 měřeno 170–180 MB pod zátěží, L112); změří šéf `perf-wall.ps1`.

**Vstup pro scénu:** `ext/wall/wall.html` (`include_str!`, `NavigateToString` ⇒ limit 2 MB; vývoj bez
překladu `LEDGE_WALL_HTML=<soubor>`). Zprávy: Rust→stránka `chrome.webview` událost `message` (`e.data`):
`init{cfg,lang,paused}` (po `{t:'ready'}` ze stránky) · `data{topic,rev,d}` (téma `media` = `MediaView`,
camelCase; po `init` poslední známé hodnoty) · `cmd{name,args}`. Stránka→Rust `chrome.webview.postMessage`:
`ready` · `req{what:'media',do:toggle|next|prev}` / `req{what:'cover',key}` (jde přes whitelist
`policy::parse_req`) · `active` · `log{msg}`. Pauza = `visibilitychange` (žádné časovače při `hidden`).

**`--diag`:** `ledge.exe --ext wall --diag` (bez Ledge, 60 s, mutex `wall` platí) zapisuje do
`%LOCALAPPDATA%\Ledge\ext\wall\diag.log` události FOREGROUND / CREATE / DESTROY / SHOW / HIDE / REORDER /
LOCATIONCHANGE / CLOAKED oken wall, Progman, WorkerW, DefView (rodič, viditelnost, pořadí sourozenců). Šéf
stiskne Win+D / restartuje Průzkumník sám a log přečte.

**Testy:** `attach.rs` (`choose_parent_table`, `attach_plan_table`), `geom.rs` (`rel_rect_table` + 4 testy sondy),
`mod.rs` (`wants_reattach_table`, simulace `TaskbarCreated` vlastním skrytým oknem v testu, kontrakt `wall.html`);
`live.rs` ručně: `LEDGE_WALL_LIVE=1 cargo test ext::wall::live -- --ignored --nocapture` (guard, sonda volné
plochy, snímek jen sondy, RAM/CPU stromu, kontrola osiřelého procesu).

## Výkon: `perf.ps1`, `perf-wall.ps1` (E4)

- `scripts/perf.ps1` měří rozpočet Ledge jen nad **jádrem (`ledge.exe` bez `--ext`) a jeho WebView2**; každé
  `ledge.exe --ext <id>` s celým stromem vypíše zvlášť řádkem „rozšíření <id>“ (mimo rozpočet). Klasifikace:
  `scripts/perf-lib.ps1` `Get-LedgeGroups` (pid → `core` / `ext:<id>`, dítě musí být mladší než rodič) a `Get-ExtId`;
  test `scripts/perf-lib.test.mjs` (node:test spouští PowerShell nad zachycenou tabulkou procesů, běží v `test:ui`).
- `scripts/perf-wall.ps1` spustí `ledge.exe --ext wall` sám (vlastní Job s KILL_ON_JOB_CLOSE, NDJSON `init`/`state{pause}`/`quit`
  po stdin/stdout) a změří CPU Jobu + private MB ve scénářích klid / pauza; porovná PID Jobu se stromem PID a vypíše
  „WebView2 podprocesy v jobu: ANO/NE“. **Jen s guardem** (`Test-MeasureGuard`: `SHQueryUserNotificationState` ∉ 2/3/4, nečinnost ≥ 20 s,
  kontrola před každým scénářem), odmítne starší build bez `--ext`; jinak vypíše „neměřeno: …“. Nepouštět z bran.
  Spuštění: `powershell -ExecutionPolicy Bypass -File scripts/perf-wall.ps1 [-Exe …] [-Seconds 30]`.

## Checklist živého ověření tapety (pro šéfa)

Dělá jen člověk; automat na ploše nesahá (Win+D, restart Průzkumníka). Log: `%LOCALAPPDATA%\Ledge\ext\wall\diag.log`.

| # | krok | co zkontrolovat | kam se podívat |
|---|---|---|---|
| 1 | Zavři Ledge a ostatní `--ext wall`. `ledge.exe --ext wall --diag` (60 s) | okno tapety je vidět pod ikonami (1 pohled): ikony a hlavní panel nad ní | oči; `diag.log` řádky CREATE/SHOW |
| 2 | Během 60 s stiskni **Win+D** a pak ho vrať | tapeta zůstane vidět, neblikne, ikony jsou pořád nad ní | `diag.log`: SHOW/HIDE/REORDER wall, rodič = WorkerW |
| 3 | Jednou restartuj Průzkumník (Správce úloh → Restartovat) | tapeta se po pár sekundách objeví znovu (`TaskbarCreated`), proces `ledge.exe --ext wall` nespadl | `diag.log`: DESTROY Progman/WorkerW → CREATE; `Get-Process ledge` |
| 4 | Druhý monitor (je-li): zapoj/odpoj, případně přesuň primární | tapeta je jen na primárním monitoru, po změně přepočítá polohu (i záporné souřadnice) | oči; `diag.log` LOCATIONCHANGE |
| 5 | Po 60 s | proces sám skončil, žádný osiřelý `msedgewebview2` s `Ledge\ext\wall` | `Get-CimInstance Win32_Process` podle příkazové řádky |
| 6 | Zapni tapetu, v Nastavení → Rozšíření → „Pozadí tapety“ → Vybrat fotku (světlou i tmavou) | fotka je na tapetě přes celou plochu (cover), text „NEDĚLE“, datum a čas jsou čitelné i na světlé fotce; „Odebrat“ vrátí přechod | oči |
| 7 | Pusť hudbu (Spotify / prohlížeč) | v přehrávači scény je skutečný obal alba (u skladby bez obalu neutrální plocha), po přeskočení skladby se obal změní | oči |
| 8 | Zamkni obrazovku (Win+L), odemkni; pak zapni režim spánku baterie / vytáhni nabíječku (notebook) | po odemčení je tapeta zpět a hodiny sedí; na baterii zmizí rozostření přehrávače a lišta průběhu (plocha se neskrývá); `perf-wall.ps1` při zamčení / pauze CPU ≈ 0 | oči; výpis skriptu |
| 9 | (volitelně, nikdo nehraje a ty se nehýbeš 20 s) `scripts/perf-wall.ps1` | „WebView2 podprocesy v jobu: ANO“, pauza CPU ≈ 0 | výpis skriptu |

## Co zatím není

- Záloha `IDesktopWallpaper` (statický snímek při selhání připnutí; mění tapetu uživatele → jen s jeho souhlasem).
- Živé ověření šéfem (checklist výše): „pod ikonami“, Win+D, restart Průzkumníka, nová pozadí / obal / pauza; ověřená RAM tapety (< 120 MB je odhad, L112) a „WebView2 podprocesy v jobu“ (`perf-wall.ps1`, L141).
- Klikání na tlačítka přehrávače: WorkerW-dítě nejspíš myš nedostává (neověřeno) → později přes desk okno.
- `POST /ctl/ext` a `ext` v `GET /status` (API/MCP).
- Tapeta jen na primárním monitoru; více monitorů a více stylů scény (F5/F8 c) jsou další vlny.
