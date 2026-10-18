# Codemapa: widgety ze skriptů (`widgets`)

Skript ve složce widgetů (`widgets.dir`, výchozí `%USERPROFILE%\Ledge\widgets`) s hlavičkou `@ledge.widget`
vypíše JSON nebo text à la xbar → karta `widget:<soubor>` (dynamická karta v `layout`) a/nebo položka proužku
`widget:<soubor>` (`StripItem::Widget`). Formát pro agenty: `AGENTS.md` → „Widgety ze skriptů“, pro lidi
`docs/widgets.md`. Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/widgets/mod.rs` | config `widgets`, složka, sken bez watcheru (stat + jen změněné, strop 50), načtení hlavičky + SHA-256 (skript ≤ 1 MB, `.exe` ≤ 64 MB + `.exe.ledge`), čerstvé čtení před během, problém (interpret) | `WidgetsConfig`, `resolve_dir_with`, `scan_dir`, `Scan`, `Widget` (`fingerprint`, `confirm_text`), `load_one`, `fresh`, `problem`, `sidecar_of`, `MAX_WIDGETS` |
| `src-tauri/src/widgets/meta.rs` | čistá hlavička (`scripts::meta::fields`): název, ikona, `refresh` (min 30 s), `show`, `timeout`, `args` | `parse`, `parse_interval`, `WidgetMeta`, `Kind`, `Show` |
| `src-tauri/src/widgets/output.rs` | čistý tolerantní parser stdout: JSON / text à la xbar, stropy, barvy (předvolby + hex podle odstínu), akce položek | `parse`, `WidgetData` (`strip_text`), `Item`, `Act`, `Tone`, `tone`, `url_ok`, `MAX_STRIP` |
| `src-tauri/src/widgets/plan.rs` | čistý plánovač: co je aktivní (karta na otevřené záložce / proužek, ne mrtvý), kdy běžet (interval + jitter, nikdy souběžně, `force`) | `Ctx`, `active`, `Slot`, `decide`, `Decision`, `jitter`, `file_of`, `PREFIX` |
| `src-tauri/src/widgets/consent.rs` | souhlas: otisk (prefix `ledge-widget`), politika nový / změněný / povolený, úložiště jeden otisk na soubor | `fingerprint`, `confirm_text`, `policy`, `Consent`, `Store`, `reason_text` |
| `src-tauri/src/widgets/run.rs` | příkaz (skript přes `scripts::run::plan`, `.exe` přímo; cwd = složka widgetu), běh přes `gitinfo::capture_tail` (Job Object, `CREATE_NO_WINDOW`), strop 256 KB, hlášky | `command`, `execute`, `Fail`, `Ran`, `fail_text`, `keep`, `MAX_OUTPUT` |
| `src-tauri/src/widgets/live.rs` | stav v paměti, vlákno `widgets` (líně, spí na condvaru), běhy (`widget-run`, nejvýš 3), pohled pro UI, segment proužku | `wake`, `wake_if_used`, `on_config`, `placed`, `rescan`, `snapshot_views`, `listing`, `grant`, `refresh_now`, `output_of`, `item`, `edit_path`, `strip_segments`, `WidgetView`, `ConsentAsk`, `Listing` |
| `src-tauri/src/widgets/templates.rs` · `templates/*` | galerie (zakompilovaná `include_str!`), kopie do složky (`.ps1` s BOM, CRLF, nikdy nepřepíše) | `ALL`, `Template`, `views`, `install`, `TemplateView` |
| `src-tauri/src/widgets/tests.rs` | sken v tempdiru, cache, `.exe` + hlavička, skutečný cmd / PowerShell (cwd, UTF-8, chyba, termín, strop), offline příklady z galerie opravdu běží, souhlas po změně | — |
| `src-tauri/src/commands/widgets.rs` | Tauri příkazy; klik na položku (web / složka-dokument přes `personal::classify_fs`, schránka `shelf::clip`, povolený skript s argumentem), editor slovesem `edit` | `widgets_list`, `widget_consent`, `widget_refresh`, `widget_output`, `widget_edit`, `widgets_open_dir`, `widget_templates`, `widget_install`, `widget_click`, `run_script_with`, `open_in_editor` |
| `src/widgets/model.ts` | čistá logika UI (test `model.test.mjs`): id `widget:<soubor>`, barvy, interval, klik, průběh, katalog, položky proužku | `widgetFile`, `isWidgetId`, `TONE_VAR`, `intervalParts`, `itemKind`, `progressPct`, `isStale`, `catalogWidgets`, `stripWidgetIds`, `withStripItem` |
| `src/widgets/api.ts` · `registry.ts` | invoke · názvy widgetů pro katalog / `layout/text.ts` + poslední seznam ze složky | `widgetsApi` · `rememberWidgets`, `widgetInfo`, `loadWidgets`, `widgetsListing` |
| `src/widgets/WidgetCard.tsx` · `WidgetsSettings.tsx` · `widgets.css` | karta (souhlas, chyba + výstup, položky, ⋯) · sekce Widgety + galerie v panelu Projekty, akce a odkazy | `WidgetCard`, `WidgetConsent`, `toggleStrip`, `intervalText` · `WidgetsSettings` |
| `src/types.ts` (blok `// --- widgets ---`), `src/i18n/en-widgets.ts` | typy + `Snapshot.widgets` · angličtina (`cards.widget.*`, `settings.widgets.*`) | `WidgetView`, `WidgetData`, `WidgetItem`, `WidgetsListing`, `WidgetTemplate` |

Lepidlo: `config.rs` (`Config.widgets` + `widgets::validate`), `layout.rs` (`is_dynamic` ve `validate`),
`stripconf.rs` (`StripItem::Widget`, ruční serde `key` / `parse`, schéma), `stripview.rs` (větev `Widget`),
`snapshot.rs` (`widgets`), `settings.rs` (`apply_config` → `on_config`), `threads/ui.rs` (otevření šuplíku →
`wake_if_used`), `commands/drawer.rs` (`set_mode` → `wake_if_used`), `lib.rs` (start + `generate_handler!`),
`scripts/meta.rs` (`fields`, `cut`, `flag` sdílené). UI: `layout/model.ts` (`CardId` = vestavěné | `widget:…`,
`isDynamic`, `defaultSize`), `layout/text.ts` (název z registru), `layout/Catalog.tsx` + `LayoutEditor.tsx`
(skupina widgetů), `cards/CardMenu.tsx` (`extra`), `MainPage.tsx` (`default` → `WidgetCard`),
`settings/panels/StripSettings.tsx` (widgety v položkách proužku), `settings/panels/ProjectsActions.tsx`,
`settings/labels.ts` + `en.ts` (`widgets`, `widgets.enabled`, `widgets.dir`, `widgets.gallery`).

## Toky

- **Běh**: událost (otevření šuplíku, záložka, config, souhlas, „Obnovit teď“, konec běhu) → `wake` → vlákno
  `widgets`: `plan::active(ctx)` → je-li co a sken starší 5 s, `rescan` (výpis + změněné soubory, souhlasy,
  problémy) → `plan::decide` → `run_one` na krátkém vlákně: `fresh` (soubor znovu + SHA-256) → souhlas sedí? →
  `run::command` → `run::execute` → `output::parse` → `Entry` + `mark_dirty` → snapshot / proužek → `wake`.
  Nic aktivního = `COND.wait` bez termínu; proužek s widgetem ve hře = kontrola po 30 s.
- **Souhlas**: karta / Nastavení `WidgetConsent` → `widget_consent(id, confirmText)` → `live::grant`: `fresh`,
  text musí sedět (obsahuje otisk) → `Store::grant` → `force` → běh hned.
- **Klik na položku**: `widget_click(id, index, label)` → `live::item` (label musí sedět s posledními daty) →
  `url` (http/https) / `open` (`classify_fs`: složka, dokument; program ne) / `copy` (`shelf::clip::set_text`)
  / `run` (`run_script_with`: skript ze složky skriptů s udělaným souhlasem, bez `@ledge.confirm`).
- **Katalog**: ✎ → „+ Přidat kartu“ → `Catalog` `loadWidgets` (`widgets_list` = sken) → `catalogWidgets`
  (show ≠ strip, na záložce ještě není) → `addCard(widget:…)` → Hotovo uloží `layout`.

## Invarianty a pasti

- Widget **nikdy** nespustí příkaz z výstupu: akce jsou jen `Act` (web, složka/dokument, schránka, povolený
  skript). `bash=` / `shell=` z xbar se ignoruje s poznámkou.
- „Upravit skript“ otevírá slovesem `edit` / Poznámkovým blokem — výchozí „otevřít“ by `.py`, `.js`, `.cmd`
  spustilo.
- Souhlas je jeden otisk na soubor (`widgets-consent.json`) — změna souboru = `Consent::Changed`, karta se
  zastaví. Soubor souhlasů není v configu, testy používají `Store::at` v tempdiru.
- `StripItem` už není `Copy` (má `Widget(String)`); v configu je text (`key` / `parse`), takže funguje i jako
  klíč mapy `strip.colors`. `ITEMS_REV` / migrace `waiting` beze změny.
- Snapshot čte jen paměť (`snapshot_views`, jen umístěné widgety); disk jen vlákno `widgets` a příkazy.
- `.ps1` s diakritikou potřebuje UTF-8 s BOM (Windows PowerShell 5.1) — galerie ho zapisuje sama.

## Testy

Rust: `cargo test widgets` (`meta::tests` interval / hlavička / chyby, `output::tests` JSON / text / rozbitý
vstup / stropy / barvy, `plan::tests` aktivní / jitter / rozhodnutí, `consent::tests` politika / úložiště,
`templates::tests` každý příklad je platný a krátký + kopie, `widgets::tests` sken a skutečné běhy),
`stripconf::tests::widget_item_as_text`, `layout::tests::validation_table` (dynamické karty). UI:
`src/widgets/model.test.mjs`, `src/layout/model.test.mjs` (dynamické karty), `settings.test.mjs` (kotvy).
