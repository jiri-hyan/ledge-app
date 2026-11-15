# Codemapa: šablonový widget (`widgetkit`)

Soubor `*.kit.json` ve složce widgetů (`widgets.dir`) popisuje řádky a tlačítka; **nic se nespouští** (žádný proces, souhlas ani SHA). Rust ho načte a ořeže (`spec::parse`), data ze snapshotu dosadí UI (`src/widgetkit`, balík kit-render). Karta `widget:<soubor>` (stejné id jako skript, jede přes `widgets/`: sken `Kind::Kit`, `WidgetView.spec`).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/widgetkit/mod.rs` | modul | — |
| `src-tauri/src/widgetkit/spec.rs` | čistý parser a validace: stropy (≤ 16 KB, 12 řádků, 6 tlačítek, texty 120 znaků, název 60), BOM, neznámé klíče ignorovány, špatný řádek = zahozen + poznámka, rozbitý JSON = `Err`; syntaxe `{zdroj|formát}` (zdroje neověřuje — to je `SOURCES` v UI); akce jen z bílé listiny | `parse`, `Spec`, `Row`, `Button`, `Acts`, `KitTone`, `template_ok`, `Note` |
| `src-tauri/src/widgetkit/needs.rs` | které skupiny dat kit potřebuje (limity / čekání / procesy / kalendář / útrata) ze zdrojů v řádcích, `{…}` a `copy`; `desk/live.rs` `window_needs` podle toho plní `DeskData` okna kitu (kit bez zdroje nespustí nic navíc, ani přepočet útraty); test hlídá shodu se `SOURCES` | `needs`, `Needs` |
| `src-tauri/src/widgetkit/spec/tests.rs` | tabulkové testy (limity, akce, syntaxe šablony, tolerance) | — |
| `src/types/widgets.ts` | kontrakt `KitSpec` / `KitRow` / `KitButton` / `KitTone`, `WidgetView.spec` + `notes` | `KitSpec` |

Bílá listina akcí (jedna na řádek / tlačítko, pořadí `open` › `url` › `copy` › `action`): `open` = cesta (klik přes `personal::classify_fs`) nebo `drawer:<karta>`; `url` = http(s) přes `widgets::output::url_ok`; `copy` ≤ 4 KB; `action` = id rychlé akce (`actions::valid_id`; existenci řeší až klik, `confirm` a souhlas příkazů drží `actionrun`). `run` skriptu se z kitu nepřenáší.

Invarianty: kit se nikdy nespouští (`widgets::live::ready` = false, `run::command` a `attempt` ho odmítnou, `widgets::problem` vrací jen chybu parseru); soubor se čte při skenu (stat bez watcheru jako u skriptů), `MAX_BYTES`+1 bajtů; `WidgetView.data` je prázdné, aby se otisk plochy nemění zbytečně. Formát souboru: `docs/agents/actions.md` (Widgety → Šablonový widget).
