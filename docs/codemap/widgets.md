# Codemapa: widgety ze skriptů (`widgets`)

Skript ve složce widgetů (`widgets.dir`, výchozí `%USERPROFILE%\Ledge\widgets`) s hlavičkou `@ledge.widget` vypíše JSON nebo text à la xbar → karta `widget:<soubor>` (dynamická karta v `layout`) a/nebo položka proužku `widget:<soubor>` (`StripItem::Widget`). Formát pro agenty …

Detail (toky, invarianty, testy, co nečíst, historie): [widgets-detail.md](widgets-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/widgets/mod.rs` | config `widgets`, složka, sken bez watcheru (stat + jen změněné, strop 50), načtení hlavičky + SHA-256 (skript ≤ 1 MB, `.exe` ≤ 64 MB + `.exe.ledge`), čerstvé čtení před během, problém (interpret) | `WidgetsConfig` `resolve_dir_with` `scan_dir` `Scan` `Widget` `fingerprint` `confirm_text` |
| `src-tauri/src/widgets/meta.rs` | čistá hlavička (`scripts::meta::fields`): název, ikona, `refresh` (min 30 s), `show`, `timeout`, `args` | `parse`, `parse_interval`, `WidgetMeta`, `Kind`, `Show` |
| `src-tauri/src/widgets/output.rs` | čistý tolerantní parser stdout: JSON / text à la xbar, stropy, barvy (předvolby + hex podle odstínu), akce položek | `parse`, `WidgetData` (`strip_text`), `Item`, `Act`, `Tone`, `tone`, `url_ok`, `MAX_STRIP` |
| `src-tauri/src/widgets/plan.rs` | čistý plánovač: co je aktivní (karta na otevřené záložce / proužek, ne mrtvý), kdy běžet (interval + jitter, nikdy souběžně, `force`) | `Ctx`, `active`, `Slot`, `decide`, `Decision`, `jitter`, `file_of`, `PREFIX` |
| `src-tauri/src/widgets/consent.rs` | souhlas: otisk (prefix `ledge-widget`), politika nový / změněný / povolený, úložiště jeden otisk na soubor | `fingerprint`, `confirm_text`, `policy`, `Consent`, `Store`, `reason_text` |
| `src-tauri/src/widgets/run.rs` | příkaz (skript přes `scripts::run::plan`, `.exe` přímo; cwd = složka widgetu), běh přes `gitinfo::capture_tail` (Job Object, `CREATE_NO_WINDOW`), strop 256 KB, hlášky | `command`, `execute`, `Fail`, `Ran`, `fail_text`, `keep`, `MAX_OUTPUT` |
| `src-tauri/src/widgets/live.rs` | stav v paměti, vlákno `widgets` (líně, spí na condvaru), běhy (`widget-run`, nejvýš 3), pohled pro UI, segment proužku | `wake` `wake_if_used` `on_config` `placed` `rescan` `snapshot_views` `listing` `grant` |
| `src-tauri/src/widgets/templates.rs` · `templates/*` | galerie (zakompilovaná `include_str!`), kopie do složky (`.ps1` s BOM, CRLF, nikdy nepřepíše) | `ALL`, `Template`, `views`, `install`, `TemplateView` |
| `src-tauri/src/widgets/tests.rs` | sken v tempdiru, cache, `.exe` + hlavička, skutečný cmd / PowerShell (cwd, UTF-8, chyba, termín, strop), offline příklady z galerie opravdu běží, souhlas po změně | — |
| `src-tauri/src/commands/widgets.rs` | Tauri příkazy; klik na položku (web / složka-dokument přes `personal::classify_fs`, schránka `shelf::clip`, povolený skript s argumentem), editor slovesem `edit` | `widgets_list` `widget_consent` `widget_refresh` `widget_output` `widget_edit` |
| `src/widgets/model.ts` | čistá logika UI (test `model.test.mjs`): id `widget:<soubor>`, barvy, interval, klik, průběh, katalog, položky proužku | `widgetFile` `isWidgetId` `TONE_VAR` `intervalParts` `itemKind` `progressPct` `isStale` |
| `src/widgets/api.ts` · `registry.ts` | invoke · názvy widgetů pro katalog / `layout/text.ts` + poslední seznam ze složky | `widgetsApi` · `rememberWidgets`, `widgetInfo`, `loadWidgets`, `widgetsListing` |
| `src/widgets/WidgetCard.tsx` · `WidgetsSettings.tsx` · `widgets.css` | karta (souhlas, chyba + výstup, položky, ⋯) · sekce Widgety + galerie v panelu Projekty, akce a odkazy | `WidgetCard`, `WidgetConsent`, `toggleStrip`, `intervalText` · `WidgetsSettings` |
| `src/types/widgets.ts`, `src/i18n/en-widgets.ts` | typy + `Snapshot.widgets` · angličtina (`cards.widget.*`, `settings.widgets.*`) | `WidgetView`, `WidgetData`, `WidgetItem`, `WidgetsListing`, `WidgetTemplate` |

Další soubory (podrobnosti v detailu):
`src/settings/panels/ProjectsActions.tsx`
