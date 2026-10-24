# Codemapa: chytřejší šuplík (kolo 20) — klávesy, přetažení, kolečko, zvuk, incidenty, export

Šest malých voleb kolem otevřeného šuplíku. Config `smart` ([config-behavior.md](../agents/config-behavior.md) → Chytřejší šuplík),
Nastavení → Šuplík a karty → Otevírání a klávesy (klávesy, přetažení, kolečko) a Claude (zvuk, incidenty). Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/smartconf.rs` | sekce `smart`: tři přepínače UI (výchozí VYPNUTO od kola 21, zapíná je člověk), zvuk a incidenty (výchozí VYPNUTO), meze, validace, pohled pro snapshot | `SmartConfig` `SoundConfig` `IncidentsConfig` `SmartView` `view` `validate` `GAP_MIN_S` `EVERY_MIN` |
| `src-tauri/src/approvalsound.rs` | tiché pípnutí (`MessageBeep`) u nového schválení / otázky Clauda; čisté rozhodnutí `decide` (vypnuto, mrtvý Ledge, QUNS, šuplík otevřený, terminál / Claude v popředí, odstup); volá `attention::on_change` | `decide` `Gate` `quiet_app` `on_needs_input` `QUIET_APPS` |
| `src-tauri/src/incidents.rs` | stav incidentů z `status.anthropic.com` (Statuspage JSON): `parse` nejzávažnějšího probíhajícího, `clean` cizího textu (řídicí i formátovací znaky Cf pryč), `redirect_ok`, `due` (interval i po chybě), `retry_floor_ok` (spodní mez 60 s mezi pokusy i po změně volby), `maybe_refresh` z `emit` / otevření šuplíku / změny záložky, `on_config`; test nad `fixtures/statuspage-unresolved.json` | `parse` `Incident` `clean` `is_format_char` `redirect_ok` `due` `retry_floor_ok` `maybe_refresh` `on_config` `view` `URL` |
| `src-tauri/src/dataexport.rs` | export útrat a limitů do CSV / Markdownu z pohledů držených v paměti (`costslive::view`, `limitslive::view`); `csv_cell` (uvozovky, ochrana před vzorci i po úvodních mezerách), prázdné = neznámé | `render` `Format` `csv_cell` `has_data` |
| `src-tauri/src/commands/smart.rs` | Tauri příkaz `export_usage(format)`: dialog Uložit jako (rodič = šuplík) + zápis | `export_usage` |
| `src-tauri/src/shelf/shortcut.rs` | přetažení na OTEVŘENÝ šuplík: `target_for` (zavřený = přihrádka, otevřený naplno = nový zástupce — zóny se nekryjí), `judge` / `classify` puštěných dat (soubor / složka / `.exe` / web; příkaz nikdy; `.lnk` `.url` `.bat` `.cmd` a vše mimo `personal::FILE_ALLOW` = `Verdict::Refuse` s větou, událost `drop-refused`), `Payload` | `target_for` `Kind` `judge` `Verdict` `classify` `Payload` `is_web_url` |
| `src-tauri/src/shelf/drop.rs` (rozšíření) | tentýž neviditelný cíl OLE s `Kind`: `Shortcut` bere i odkaz z prohlížeče a text (`UniformResourceLocatorW`, `CF_UNICODETEXT`), puštěné jen pošle jako událost `drop-shortcut` | `set_armed` `offers` `dropped_text` `propose` |
| `src-tauri/src/threads/hover.rs` (rozšíření) | druhý `Arm` nad `drawer.rect()` při `View::Full`; tah musí začít mimo Ledge (popředí ≠ vlastní pid) | `drawer_arm` `target_for` |
| `src/quick/keys.ts` | čistá logika kláves: `keyPlan` (Alt+1…9, šipky, Home / End), `neighbor` (soused podle rozmístění), `badges` | `keyPlan` `neighbor` `badges` `MAX_NUMBERED` |
| `src/quick/quick.ts` · `quick.css` | DOM: posluchač `keydown` / `keyup` na okně šuplíku, zástupci `QUICK_SELECTOR`, čísla při držení Alt jako atribut `data-qn` (žádná třída, žádný časovač) | `installQuickKeys` `QUICK_SELECTOR` |
| `src/layout/tabwheel.ts` | čistá logika kolečka nad záložkami (součet, odstup, bez zalamování); zapojení v `layout/Tabs.tsx` `onWheel` | `wheelStep` `nextTab` `wheelAllowed` `pxOf` |
| `src/actions/dropshortcut.ts` · `dropstate.ts` · `DropNew.tsx` | nový zástupce z přetaženého: čistý návrh akce (`draftFrom`, `readPayload`, `visibility` = uvidí se na záložce?), signál návrhu, panel s `ActionEditor` (líný chunk); zapíše až „Přidat“ | `draftFrom` `readPayload` `visibility` `hostCards` · `offerDrop` `dropped` · `DropNew` |
| `src/exports/model.ts` · `usage.ts` | `canExport`, položky „Exportovat…“ v ⋯ karty Claude (`MainPage` → `extra`) | `canExport` · `exportMenuItems` `exportUsage` |
| `src/limits/incident.ts` | popisky incidentu (stav, závažnost), řádek v `limits/LimitsRow.tsx` `IncidentLine` | `statusText` `severity` `shown` `STATUS_URL` |
| `src/i18n/en-smart.ts` | angličtina `actions.drop.*`, `sessions.export.*` (incidenty v `en-limits.ts`, volby v `settings/en.ts`) | |

## Toky

- **Klávesy**: `keydown` na oknu šuplíku (přijde jen s fokusem od uživatele) → `keyPlan` → `click()` na N-tém zástupci
  (jeho obvyklý `onClick`: tytéž kontroly a potvrzení) nebo `focus()` souseda. Alt drží čísla (`data-qn`), puštění / blur je odebere.
- **Přetažení**: `hover` (16 ms u hrany) → `target_for(view == Full, shelf, shortcut)` → `Arm` nad `drawer.rect` (jen vjezd s drženým
  tlačítkem, OLE kurzor, popředí ≠ Ledge) → `shelf::drop::set_armed((rect, Kind::Shortcut))` → okno nad šuplíkem → `Drop` →
  `classify` → událost `drop-shortcut` → `App` `offerDrop` → `MainPage` `DropNew` → `ActionEditor` → Přidat: `get_config` →
  `save_config` (validace `actions::validate_one`) → případně `addCard` (když na záložce není karta, která akci ukáže).
- **Zvuk**: `attention::on_change` (nové schválení / otázka) → `approvalsound::on_needs_input` → brány → `MessageBeep`.
- **Incidenty**: `emit` (probuzení ≥ 1×/min), otevření šuplíku, změna záložky → `incidents::maybe_refresh` → `due` → `reqwest` GET →
  `parse` → `STATE` → `Snapshot.incident` → `IncidentLine`. Vypnuto / karta nevidět / šuplík zavřený = žádná síť.

## Invarianty a pasti

- Zvuk a síť jsou VYPNUTÉ; klávesy, přetažení a kolečko jen přidávají chování, které dřív nic nedělalo.
- Přihrádka (zavřený proužek) a nový zástupce (otevřený šuplík) se vybírají STAVEM šuplíku, ne polohou — nikdy nejsou aktivní zároveň.
  Vlastní tah karet v šuplíku (pointer události) cíl nezapne: vyžaduje vjezd s drženým tlačítkem a popředí jiné aplikace.
- Z puštěného nikdy nevznikne `command`; `.exe` je `app` s předvyplněným „ptát se před spuštěním“; nic se nespustí ani neuloží bez Přidat.
- Incident: text je cizí (ořez, bez řídicích znaků, jen jako text); chyba sítě nic nehlásí v UI (starý údaj zůstane, důvod do logu).
- V `quick/quick.css` nesmí být CSS třídy (jen `data-qn`), jinak kolize s vlastníky tříd (`cssClasses.test.mjs`).

## Testy

Rust: `smartconf::tests` (starý config, round-trip, meze), `approvalsound::tests` (tabulka bran, aplikace), `incidents::tests`
(fixture, tabulka parse, ořez, přesměrování, interval), `dataexport::tests` (CSV / Markdown, vzorce, prázdné), `shelf::shortcut::tests`
(zóny se nekryjí, `classify`, web). UI: `quick/keys.test.mjs`, `layout/tabwheel.test.mjs`, `actions/dropshortcut.test.mjs`,
`exports/model.test.mjs`, `limits/incident.test.mjs`. Win32 (OLE nad šuplíkem, `MessageBeep`, dialog) jen živě.
