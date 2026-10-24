# Codemapa: widgety na ploše (`desk`)

Karta Ledge jako malé okno bez rámu na ploše / druhém monitoru („Na plochu“). Žije i se zavřeným šuplíkem.
Config `desk` = jen výchozí chování nových widgetů ([agents/config-behavior.md](../agents/config-behavior.md) →
Widgety na ploše); které widgety a kde = `%LOCALAPPDATA%\Ledge\desk.json` (jen tento PC). Návod pro lidi
`docs/widgety-na-plose.md`. Vzor oken: lístky ([notes.md](notes.md) → `pins/*`). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/desk/mod.rs` | čisté: druhy (`Kind`: limity, čekající, dev servery, agenda, `widget:<soubor>`), widget a soubor `desk.json` (tolerantní načtení, strop, duplicity), `Patch`, profily (`save_profile` / `apply_profile` → `Diff`), viditelnost ve hře (`shown`, `on_top`, `movable`), umístění (`plan_place` volá `pins::restore`, `home_on_screen` volá `pins::monitor_of`), přichycení `snap`, config `DeskConfig` | `Kind` `Layer` `DeskConfig` `DeskWidget` `DeskFile` `Profile` `Patch` `Diff` `diff` `AddErr` `load` `save` `label_for` `id_of` `valid_id` `new_id` `clean_name` `same_name` `shown` `on_top` `movable` `plan_place` `home_on_screen` `snap` `MAX_WIDGETS` `MAX_PROFILES` `LABEL_PREFIX` |
| `src-tauri/src/desk/tests.rs` | tabulkové testy `mod.rs` (druhy, id, config starý / částečný, soubor chybějící / rozbitý / starý tvar / strop, přidání, volby, profily, viditelnost ve hře, odpojený monitor, přichycení) | — |
| `src-tauri/src/desk/win.rs` | okna `desk-<id>`: vznik bez fokusu, vrstvy, zámek / průchozí, schovat ve hře (`SW_HIDE` + uspaný WebView `dormant`, návrat `SW_SHOWNOACTIVATE`), geometrie z událostí, mimo domov při odpojeném monitoru (`DISPLACED`) a návrat (`on_monitors`), volby ⋯ (dočasně větší okno `menu`), profily, schovat vše, pohled pro snapshot | `pin` `open` `close` `set` `menu` `hide_all` `sync_yield` `set_fs_kind` `restore_all` `on_monitors` `profile_save` `profile_apply` `profile_delete` `view` `DeskView` `shown_labels` `shown_kinds` `script_ids` `on_config` |
| `src-tauri/src/desk/save.rs` | ukládání `desk.json` vyčleněné z `win.rs` (kolo 21): zpožděný zápis ve vlákně `desk-save` (série pohybů = jeden zápis), přichycení po puštění myši (`snap_moved`), okamžitý `flush` při konci Ledge | `persist` `mark_moved` `flush` `snap_moved` |
| `src-tauri/src/desk/live.rs` | data pro okna (`desk-data`): jen viditelné druhy, jen při změně (otisk JSON), čas po minutách; „čeká na tebe“ jen řádky ke čtení | `tick` `invalidate` `data_for` `DeskData` `WaitRow` `wait_rows` `clip` `folder_name` `WAIT_MAX` |
| `src-tauri/src/commands/desk.rs` | příkazy: šuplík (`desk_pin`, `desk_unpin`, `desk_set` podle id, profily, `desk_hide_all`), okno widgetu podle labelu (`desk_init`, `desk_set`, `desk_drag`, `desk_menu`, `desk_unpin`, `desk_open_drawer` → náhled se schválením / Nastavení / šuplík) | `desk_pin` `desk_unpin` `desk_init` `desk_set` `desk_drag` `desk_menu` `desk_open_drawer` `desk_profile_save` `desk_profile_apply` `desk_profile_delete` `desk_hide_all` `DeskInit` |
| `desk.html` · `src/desk/main.tsx` · `desk.css` | vlastní vstup Vite okna widgetu (`styles.css` + karty, které znovupoužívá; angličtina líně přes `i18n/dicts.ts`) · styly okna (prefix `dk-`, průhlednost `--dk-o`) | |
| `src/desk/DeskApp.tsx` | okno widgetu: karta s nadpisem + průhledná lišta přes nadpis (tah = `desk_drag`, ⋯ = volby v `PopMenu fill`: vrstva, kompaktní, zámek, průchozí, schovat ve hře, průhlednost, Otevřít Ledge, Odepnout); poslech `desk-data`, `desk-prefs`, `desk-lang`, `appearance` | `DeskApp` |
| `src/desk/views.tsx` | obsah podle druhu: `LimitsRow`, `Running`, `CalendarCard`, `WidgetCard` nad `deskSnapshot`; vlastní „Čeká na tebe“ (klik = šuplík, nic neschvaluje) a skript bez souhlasu = odkaz do šuplíku | `DeskBody` |
| `src/desk/model.ts` | čistá logika (test `model.test.mjs`): druhy a názvy (páry), `kindsOfCard`, stav připnutí, vrstvy, průhlednost, jméno profilu, data → tvar snapshotu | `BUILTIN_KINDS` `KIND_NAME` `KIND_HINT` `LAYERS` `kindsOfCard` `kindName` `isKind` `widgetOf` `canPin` `clampOpacity` `cleanProfileName` `sameName` `waitMinutes` `deskSnapshot` `liveCount` |
| `src/desk/api.ts` · `pin.tsx` · `palette.ts` | invoke · ⋯ karty „Připnout na plochu“ (`DeskPinItems` v `cards/CardMenu.tsx`), hlášky, názvy · Ctrl+K (`deskItems` v `palette/items.ts`) | `deskApi` `DeskPatch` · `DeskPinItems` `pinKind` `unpinWidget` `deskKindName` · `deskItems` |
| `src/desk/DeskSettings.tsx` · `settings.css` | Nastavení → Na ploše: seznam widgetů s volbami (hned), + připnout, schovat vše, profily, výchozí chování (`desk.layer`, `desk.hideInGame`, `desk.opacity`, `desk.snap` — lišta Uložit) | `DeskSettings` |
| `src/types/desk.ts` · `src/desk/en.ts` | typy (`Snapshot.desk`, `desk-data`) · angličtina `cards.desk.*`, `settings.desk.*` | `DeskWidget` `DeskView` `DeskData` `DeskInit` `DeskWaitRow` |

Lepidlo (cizí soubory, jeden až pár řádků): `lib.rs` (`mod desk`, příkazy, `restore_all` po lístcích, `flush`),
`commands/mod.rs`, `config/mod.rs` (`Config.desk`), `threads/hover.rs` (`set_fs_kind`, `sync_yield` vedle
`pins::win::sync_dead`, `on_monitors` při změně monitorů), `gamemode.rs` (`sync_yield`), `threads/emit.rs`
(`live::tick`), `settings.rs` (`on_config`), `snapshot.rs` (`desk`), `widgets/plan.rs` (`Ctx.desk`) +
`widgets/live.rs` (`placed`, `ctx`), `webpanel.rs` (test capabilities: `desk-*`), `capabilities/desk.json`,
`vite.config.ts` (vstup `desk`). UI: `cards/CardMenu.tsx`, `palette/items.ts`, `settings/panels/Drawer.tsx`,
`settings/labels.ts` + `en.ts` + `model.ts` (`desk` → `suplik`) + `registry.ts` (kw) + `settings.test.mjs`
(PANEL_FILES), `types/index.ts` + `types/snapshot.ts`, `i18n/dicts.ts`.

## Okna: proč jedno okno na widget (rozhodnutí kola 20)

- **Zjištěno z dokumentace WebView2** ([Process model](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model)):
  všechny WebView se stejnou složkou dat a stejnými volbami prostředí sdílí JEDEN proces prohlížeče; renderer
  „není svázaný s jedním `CoreWebView2`… může obsluhovat rámce více instancí“ podle izolace webů — nový renderer
  vzniká hlavně pro jiný web (doménu). Ledge dává všem oknům stejné `drawer::BROWSER_ARGS` (vč.
  `--renderer-process-limit=1`, `--in-process-gpu`, síť v procesu prohlížeče) a výchozí data dir → widget
  NEPŘIDÁ proces prohlížeče ani GPU; očekávaně ani renderer (stejný původ jako šuplík) — **neměřeno**.
  Každé okno má ale vlastní stránku (paměť JS, DOM, kompozici okna), takže paměť roste s počtem widgetů.
- **Jedno průhledné okno přes celý monitor nejde:** WebView2 kreslí přes DirectComposition, klik do průhledného
  místa nepropadne na plochu ani okna pod ním (okno by spolklo myš na celém monitoru); `set_ignore_cursor_events`
  je jen pro celé okno. Navíc by nešla vrstva, zámek a průchozí zvlášť pro každý widget.
- **Proto:** okno na widget + strop `MAX_WIDGETS` = 6 („Na ploše může být nejvýš 6 widgetů — každý je samostatné
  okno a bere paměť.“). Schovaný widget (hra, „schovat vše“) má WebView uspaný (`TrySuspend` + nízký cíl paměti) a
  nedostává data. Úspora navíc ke změření: `--process-per-site` v `BROWSER_ARGS` (sdílí renderer jistě) — jen přes
  `scripts/argbench.ps1`, mění to všechna okna.

## Toky

- **Připnout**: ⋯ karty / Ctrl+K / Nastavení → `deskApi.pin(kind)` → `desk_pin` → `win::pin` (`DeskFile::add`:
  stejný druh = tentýž widget, plno = věta) → `open` (bez fokusu, `plan_place`) → okno `desk.html` → `desk_init`
  (widget, jazyk, data) → `DeskApp`.
- **Data**: vlákno `emit` po každém probuzení → `live::tick` → nic, když žádný widget není vidět → `build` jen pro
  viditelné druhy (limity `limitslive::view`, čekající `claude.pending/questions` → `wait_rows`, procesy
  `procs.snapshot`, kalendář `calendar::live::view` z cache, skripty `widgets::live::snapshot_views`) → otisk ≠
  minule → `desk-data` viditelným oknům. Čas `now` po minutách = nejvýš 1 zpráva/min v klidu (emit se budí
  na minutu tak jako tak). Kalendář se kvůli widgetu NEstahuje (bez nové sítě) — obnoví ho otevřený šuplík.
- **Skripty**: `widgets::live::placed` zahrnuje widgety na ploše (snapshot je ukáže), `plan::Ctx.desk` = viditelné
  widgety na ploše → plánovač je spouští i se zavřeným šuplíkem, ve hře ne.
- **Hra**: `hover` / `gamemode::set` → `sync_yield` → `pins::pins_yield` (stejné pravidlo jako lístky; prohlížeč
  ve F11 ne) → `shown` = schovat ty s `hideInGame`, navrchu nikdo; po hře zpět bez aktivace.
- **Monitor**: start / profil → `plan_place`: domov na monitoru = tam (velikost × DPI), jinak monitor proužku a
  `DISPLACED` (domov v souboru zůstane). `Moved` s domovem mimo všechny monitory (Windows okno přesunul po
  odpojení) = mimo domov, ne nový domov. `hover` při změně sestavy monitorů → `on_monitors` → domů.
- **Profily**: `profile_save` (kopie widgetů pod jménem) · `profile_apply` → `Diff`: zavřít / přesunout / otevřít.

## Invarianty a pasti

- Widget **nic neschvaluje** (test `model.test.mjs`): „Čeká na tebe“ jen otevře šuplík (`View::Peek`), souhlas se
  skriptem jen v šuplíku. Dev servery: Otevřít / Stop / Log jsou tytéž příkazy jako karta Běží (`stop` zastaví jen
  procesy, které spustil Ledge).
- Fokus: okno vzniká `focused(false)` + viditelné; ukázání po hře `SW_SHOWNOACTIVATE` (Tauri `show()` by aktivovalo).
- Výchozí stav = žádný widget, žádné okno, žádné vlákno (`desk-save` vznikne s prvním zápisem); `desk.json` se čte
  až při startu ve vlákně obnovy (snapshot na disk nesahá).
- `desk.json` jen na tomto PC (`actionrun::local_dir`), config se může synchronizovat — poloha do něj nepatří.
- Capability `desk.json` dává oknům `desk-*` poslech událostí a jen příkazy widgetu (tabulka `DESK_WINDOW`
  v `ipcacl.rs`, AppManifest v `build.rs`): `desk_*` svého okna, kliky / výstup / obnovení widgetu, karty Běží a Agenda,
  čtení configu. Zápis configu ani souhlas se skriptem z widgetu neprojde (Agenda proto na ploše `readOnly`). Nové volání
  z okna widgetu = řádek v `capabilities/desk.json` + v tabulce. Rust bere id z labelu volajícího okna. Test
  `webpanel::tests::capabilities_never_reach_web_windows` zná `desk-*`, `ipcacl::tests::allowed_table` hlídá hranice.
- Třídy CSS jen `dk-` (`desk.css` okno, `settings.css` Nastavení — `desk.css` nastavuje `html, body, #root`, do
  šuplíku se nesmí importovat).
- Návrh sjednocení s `pins`: `win.rs` má kopii `pins::win::fallback`, `monitors`, převod `KINDS` a výpočet
  „ustoupit hře“ (`yielding`) — `pins::win` je má soukromé. Až to LÍSTKY dovolí: `pins::win::{fallback, monitors,
  yielding}` jako `pub(crate)` a desk je jen volá.

## Testy

Rust `cargo test desk` (`desk::tests::*` tabulky, `desk::live::tests` řádky čekajících, ořez, otisk),
`widgets::plan::tests::active_table` (plocha), `config::tests` (starý config), `webpanel::tests`. UI
`src/desk/model.test.mjs` (karty → druhy, zrcadlo Rustu, strop, průhlednost, profily, tvar snapshotu, nic
neschvaluje), `settings.test.mjs` (kotvy `desk.*` na stránce Na ploše, `panels/Desk.tsx`), `segments.test.mjs`, i18n, třídy CSS.
Okna samotná jen naživo (GUI) — postup v reportu kola 20.
