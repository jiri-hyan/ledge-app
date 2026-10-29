# Codemapa: Nastavení — panely, pole, model, záloha (skořápka = Ledge Studio → [studio.md](studio.md))



Detail (toky, invarianty, testy, co nečíst, historie): [settings-detail.md](settings-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src/settings/placement.ts` | klíč configu → stránka + úroveň Základní / Pokročilé (nejdelší prefix); zdroj `panelForField` a štítku Pokročilé — guard `placement.test.mjs` (`NO_EDITOR` s důvody je v testu) | `PLACEMENT` `placeOf` `ADVANCED_ANCHORS` `isAdvancedField` |
| `src/settings/searchModel.ts` | chytré hledání: synonyma cs/en, klíč configu (i camelCase část), překlep ≤ 1 od 5 znaků, položky jen pro klíč — test `searchModel.test.mjs` | `smartSearch` `SYNONYMS` `synonymsOf` `keyWords` `nearlyEqual` |
| `src/settings/ctx.ts` | rozpracovaný config (store na úrovni modulu), `base`, chyba, hit, Vrátit, úkoly po uložení, kontext pro pole | `cfg` `loadBase` `adoptBase` `mcp/merge.ts` `discard` `setCfg` `makeCtx` `useSettings` |
| `src/settings/registry.ts` · `catalog.ts` · `labels.ts` | stránky bez komponent (názvy, kw — čte i paleta) a staré id · stránky s komponentami + položky hledání (volby s klíčem a štítkem Pokročilé, prefixy klíčů) · popisky voleb (JEDINÝ zdroj pro `Row` i … | `PANEL_META` `PANEL_ALIASES` `PANELS` `searchItems` `FIELDS` `fieldLabel` `fieldHint` |
| `src/settings/en.ts` · `panels/en.ts` | angličtina (`registerDict`, líně načítá `src/i18n/dicts.ts`): skořápka, nav, volby `settings.f.<id>`/`.hint`/`.kw`, sekce configu · texty panelů (vč … |  |
| `src/settings/model.ts` | čistá logika: hledání bez diakritiky (`search` = prosté, skořápka používá `searchModel.smartSearch`), cesty, chyba→stránka (`placement.ts` … | `search` `highlight` `panelForField` `resolveTarget` `moveItem` `dropIndex` |
| `src/settings/fields/*` (`fields/{Hotkey,Row,SortList}.tsx`; společný tvar panelů `settings/types.ts`, ikony panelů `settings/icons.tsx`) | `Row`/`Section`/`PanelHead`/`FieldErr` (`id` = kotva `f-<id>` i u sekce a hlavičky; `Row live` + `LiveBadge` = štítek „Platí hned“), `Advanced.tsx` (sdílené sbalitelné „Pokročilé“ … |  |
| `src/settings/fields/Group.tsx` | sbalovací skupina voleb (kolo 21): nadpis + věta, výchozí `open`, `badge`, `aside` (tlačítko v hlavičce nesbalí), kotva `anchor`; skok (`hit`) rozbalí; obsah připojený i sbalený (`hidden`) … | `Group` |
| `src/settings/panels/Startup.tsx` | stránka Spouštění a hry (Aplikace): autostart s bublinou MSIX, Šetřit paměť, hry a herní režim | `StartupPanel` |
| `src/settings/panels/Notify.tsx` · `src/types/notify.ts` | sekce Oznámení na stránce Claude (kolo 26): kde (Notch / toast), záložní toast, doba, styl každé události + Vyzkoušet (`notifyPreview`) a Vrátit; zrcadlo `NotifyConfig` | `NotifySection` `NOTIFY_DEFAULT` |
| `src/settings/appearanceModel.ts` | panel Vzhled jako čistá data: kroky a kde leží která kotva (i „Pokročilé“), volby přepínačů (≤ 4), monitor ve skutečném poměru (šířka = zrcadlo Rust `preset_width`, pozice = `anchor_c2`, sloupce … | `STEPS` `ANCHOR_STEP` `ADVANCED` `stepOf` `isAdvanced` `WIDTH_STEPS` `DENSITY_STEPS` |
| `src/settings/panels/*` | panely a jejich sekce — tabulka Panely výš; každý panel jen mění draft (`PanelProps` bez `save`); proužek čistou logikou `settings/strip.ts` (+ `strip.test.mjs`) … |  |
| `src/settings/advModel.ts` · `personalModel.ts` · `cityModel.ts` | čistá logika bez DOM: paměť rozbalení Pokročilé (`parseAdvOpen` `withAdv`) a skupin (`parseGroups` `groupOpen` `withGroup`), pořadí skupin Obsahu karet (`layoutCardIds` `splitPersonal` … |  |
| `src/settings/widen.ts` | širší šuplík pro Nastavení a přechod podle stylu pohybu (`theme/appearance.ts` `widenMotion`; none / omezit pohyb = okamžitě) | `SETTINGS_WIDE` `wantWidth` `holdWide` `widenIn` `narrowOut` |
| `src/fit.ts` · `fit.css` | `ViewMsg.maxH` → `--drawer-max-h`; tělo stránky se posouvá uvnitř (všechny stránky) |  |
| `src-tauri/src/commands/settings.rs` | příkazy Nastavení | `check_config` `config_defaults` `config_export` `config_import_preview` `pick_path` |
| `src/settings/launcherImport.ts` | import zástupců z katalogu Launcheru (kolo 18): `items[]` → projekty (dev z `web.run`, app z první `actions[].exec`, vypnuté = … | `importLauncherCatalog` `projectOf` `folderId` `folderName` `commandsOf` `withVerdicts` |
| `src/settings/projectAdd.ts` | přidání projektu: duplicita podle složky i remote, id z návrhu, kontrola odkazů projektu, složka × akce (kolo 24) | `findDuplicate` `remoteKey` `draftIds` `linkProblem` `idChanged` `addTarget` |
| `src-tauri/src/configerr.rs` | chyba s cestou k poli | `FieldError` `fe` `field_from_message` |
| `src-tauri/src/configio.rs` | export, náhled importu po sekcích (rizikové), nic nezapisuje; pojistka zápisu | `section_diff` `preview` `read_import` `export` `RISKY` `rotate_backups` (config.json.1..3) `keep_unreadable` |
| `src-tauri/src/projectdetect.rs` | návrh projektu ze složky, naklonovaná repa v roots | `detect` `guess_port` `make_id` `candidates` |
| `src-tauri/src/commands/geo.rs` · `weather.rs` (dole) | hledání města pro pole Město: `geo_search(query) -> Vec<Place>`; `parse_places` / `clean_query` / `geo_url` čisté (test na `fixtures/open-meteo-geocoding.json`), `search_places` async 5 s | `geo_search` `Place` `parse_places` `clean_query` `geo_url` `search_places` |
| `src-tauri/src/drawerfit.rs` | strop výšky (80 % pracovní plochy, nejvýš do − 20 px; vlna 9) a šířka Full | `limits` `fit_full` `HEIGHT_FRACTION` `BOTTOM_GAP` `WIDE_FRACTION` |

Další soubory (podrobnosti v detailu):
`src/pkgctx/BubbleNotice.tsx` · `src/settings/fields/controls.tsx` · `src/settings/geoApi.ts` · `src/settings/{notesModel.ts,backupModel.ts}` (zámek Id kategorie; názvy sekcí Zálohy + test parity s Config) · `src/settings/panels/AboutInstall.tsx` (Nainstalovat aktualizaci v O aplikaci) · `src/settings/panels/{AppPanel.tsx,Backup.tsx,Behavior.tsx,CityPicker.tsx,Hotkeys.tsx,LegacyLinks.tsx,Notes.tsx,ProjectEditor.tsx}`
