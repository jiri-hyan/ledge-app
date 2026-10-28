# Codemapa: widgety na ploše (`desk`)

Karta Ledge jako malé okno bez rámu na ploše / druhém monitoru („Na plochu“). Žije i se zavřeným šuplíkem. Config `desk` = jen výchozí chování nových widgetů ([agents/config-behavior.md](../agents/config-behavior.md) → Widgety na ploše); které widgety a kde = `%LOCALAPPDATA%\Ledge\desk.json` (jen tento PC) …

Detail (toky, invarianty, testy, co nečíst, historie): [desk-detail.md](desk-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/desk/mod.rs` | čisté: druhy (`Kind`: limity, čekající, dev servery, agenda, kolo 22 týden / hodiny / Claude dnes — [board.md](board.md), `widget:<soubor>`), widget a soubor `desk.json` (tolerantní načtení, strop … | `Kind` `Layer` `DeskConfig` `DeskWidget` `DeskFile` `Profile` `Patch` `Diff` `diff` |
| `src-tauri/src/desk/tests.rs` | tabulkové testy `mod.rs` (druhy, id, config starý / částečný, soubor chybějící / rozbitý / starý tvar / strop, přidání, volby, profily, viditelnost ve hře, odpojený monitor, přichycení) | — |
| `src-tauri/src/desk/win.rs` | okna `desk-<id>`: vznik bez fokusu, vrstvy, zámek / průchozí, schovat ve hře (`SW_HIDE` + uspaný WebView `dormant`, návrat `SW_SHOWNOACTIVATE`), geometrie z událostí, mimo domov při odpojeném … | `pin` `open` `close` `set` `menu` `hide_all` `sync_yield` `set_fs_kind` `restore_all` |
| `src-tauri/src/desk/save.rs` | ukládání `desk.json` vyčleněné z `win.rs` (kolo 21): zpožděný zápis ve vlákně `desk-save` (série pohybů = jeden zápis), přichycení po puštění myši (`snap_moved`), okamžitý `flush` při konci Ledge | `persist` `mark_moved` `flush` `snap_moved` |
| `src-tauri/src/desk/live.rs` | data pro okna (`desk-data`): každé viditelné okno dostane jen data svého druhu (`per_window`; Hodiny ani Týden nedostanou útratu), jen při změně (otisk JSON), čas po minutách … | `tick` `per_window` `invalidate` `data_for` `DeskData` `WaitRow` `wait_rows` `clip` |
| `src-tauri/src/commands/desk.rs` | příkazy: šuplík (`desk_pin`, `desk_unpin`, `desk_set` podle id, profily, `desk_hide_all`), okno widgetu podle labelu (`desk_init`, `desk_set`, `desk_drag`, `desk_menu`, `desk_unpin` … | `desk_pin` `desk_unpin` `desk_init` `desk_set` `desk_drag` `desk_menu` `desk_open_drawer` |
| `desk.html` · `src/desk/main.tsx` · `desk.css` | vlastní vstup Vite okna widgetu (`styles.css` + karty, které znovupoužívá; angličtina líně přes `i18n/dicts.ts`) · styly okna (prefix `dk-`, průhlednost `--dk-o`) |  |
| `src/desk/DeskApp.tsx` | okno widgetu: karta s nadpisem + průhledná lišta přes nadpis (tah = `desk_drag`, ⋯ = volby v `PopMenu fill`: vrstva, kompaktní, zámek, průchozí, schovat ve hře, průhlednost, Otevřít Ledge, Odepnout) … | `DeskApp` |
| `src/desk/views.tsx` | obsah podle druhu: `LimitsRow`, `Running`, `CalendarCard`, `WidgetCard`, kolo 22 `board/WeekCard`, `BigClockCard` (bez sekund), `UsageCard` (nic nevolá) nad `deskSnapshot` … | `DeskBody` |
| `src/desk/model.ts` | čistá logika (test `model.test.mjs`): druhy a názvy (páry), `kindsOfCard`, stav připnutí, vrstvy, průhlednost, jméno profilu, data → tvar snapshotu | `BUILTIN_KINDS` `KIND_NAME` `KIND_HINT` `LAYERS` `kindsOfCard` `kindName` `isKind` |
| `src/desk/api.ts` · `pin.tsx` · `palette.ts` | invoke · ⋯ karty „Připnout na plochu“ (`DeskPinItems` v `cards/CardMenu.tsx`), hlášky, názvy · Ctrl+K (`deskItems` v `palette/items.ts`) | `deskApi` `DeskPatch` `DeskPinItems` `pinKind` `unpinWidget` `deskKindName` `deskItems` |
| `src/desk/DeskSettings.tsx` · `settings.css` | Nastavení → Na ploše: seznam widgetů s volbami (hned), + připnout, schovat vše, profily, výchozí chování (`desk.layer`, `desk.hideInGame`, `desk.opacity`, `desk.snap` — lišta Uložit) | `DeskSettings` |
| `src/types/desk.ts` · `src/desk/en.ts` | typy (`Snapshot.desk`, `desk-data`) · angličtina `cards.desk.*`, `settings.desk.*` | `DeskWidget` `DeskView` `DeskData` `DeskInit` `DeskWaitRow` |

Další soubory (podrobnosti v detailu):
`src/settings/panels/{Desk.tsx,Drawer.tsx}` · `src/types/snapshot.ts`
