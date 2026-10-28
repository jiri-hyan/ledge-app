# Codemapa: Nastavení — panely, pole, model, záloha (skořápka = Ledge Studio → [studio.md](studio.md))

## Kostra (kolo 21)

Nastavení zůstává v **rozšířeném šuplíku** (ne samostatné okno): žádná nová oprávnění ani další WebView (paměť),
sdílený rozpracovaný config i skoky z karet beze změny; větší (`widen.ts` `SETTINGS_WIDE` 1280, výška pevná
`min(860px, strop − hlavička)` v `settings.css` `.st`). Navigace = **sekce** (`settings/structure.ts` `SECTIONS`:
Doporučené · Vzhled · Šuplík a karty · Claude · Projekty a akce · Poznámky a lístky · Na ploše · Aplikace),
sekce s víc stránkami je rozbalí pod sebou (Šuplík a karty = Záložky a karty · Otevírání a klávesy · Obsah karet;
Aplikace = Jazyk a průvodce · Spouštění a hry · Záloha a verze). Stránka = panel z registru (níž). **Navštívené stránky žijí dál
skryté** (`Settings.tsx` `visited` + `hidden`, čistá `structure.ts` `withVisited`) — přepnutí nic nezničí ani
znovu nenačte, posuv každé stránky se pamatuje. Přepínač **Základní / Pokročilé** (`settings/mode.ts`,
localStorage `ledge.settings.mode`): v Základním jsou oddíly „Pokročilé“ (`fields/Advanced.tsx`, `panels/apUi.tsx`
`Advanced`, atribut `data-adv`) skryté úplně, kromě `basic` oddílů a oddílu, na který vedl skok; pod stránkou
pak „Ukázat pokročilé“. **Skupiny** (`fields/Group.tsx`, kolo 21 — panely): dlouhé stránky (Projekty a akce,
Obsah karet, Poznámky a lístky) mají sbalovací skupiny s lidským nadpisem a větou; NEjsou to Pokročilé (vidět
i v Základním), nejčastější otevřené, paměť `advModel.ts` `GROUP_KEY`. Malé sekce vedle sebe ve 2 sloupcích
(`.st-cols`, jen když je stránka široká). **Náhled dole** (`settings/PreviewDock.tsx`, jen u Vzhledu): Zavřený ·
Po najetí · Otevřený z `peek() ?? rozpracovaný vzhled` (+ `peekLook` / `peekHover` z dlaždic galerií) — najetí
na dlaždici mění JEN náhled, živý šuplík až klik a Uložit; „Co se zrovna děje“ je jen tady.
Kde se edituje který klíč configu: `settings/placement.ts` `PLACEMENT` (stránka + úroveň; i `panelForField`),
guard `placement.test.mjs` (každý klíč z Rust struktur má místo, nebo `NO_EDITOR` s důvodem). Chytré hledání
`settings/searchModel.ts` `smartSearch` (synonyma cs/en, klíč configu, překlep) + `searchModel.test.mjs`.

Čti tohle místo celých souborů. Cituj symbolem, ne číslem řádku. Sekce jiných oblastí (Vzhled, akce,
otázky Clauda, rozložení, schránka, přehrávač, kalendář) jen skládají panely — jejich mapy jsou u nich.

## Stránky (panely; id v `registry.ts` `PANEL_META`, komponenta v `catalog.ts` `COMPONENTS`, sekce v `structure.ts`)

| id | stránka | skládá (soubory) |
|---|---|---|
| `naplose` | Widgety na ploše | `panels/Desk.tsx` (nadpis „Na ploše“ jen tady) → `desk/DeskSettings.tsx` (sekce Widgety na ploše + Jak se widgety chovají; [desk.md](desk.md)); dřív sekce Šuplíku a karet |
| `zaciname` | Jazyk a průvodce (dřív Začínáme) | `panels/Start.tsx` — jazyk (`ui.lang`, první řádek), „Spustit průvodce znovu“ (`onboarding.wizard` → `startWizard`), „Upravit šuplík“, klávesy a gesta; tipy a novinky jsou jen v Doporučených |
| `spousteni` | Spouštění a hry (kolo 21; dřív konec Chování) | `panels/Startup.tsx` — `autostart` + `pkgctx/BubbleNotice.tsx`, „Šetřit paměť“ (`drawer.keepWarmS`), hry přes celou obrazovku, herní režim (zkratka herního režimu zůstává v Otevírání a klávesy) |
| `suplik` | Záložky a karty | `panels/Drawer.tsx` → `layout/LayoutEditor.tsx` (záložky, karty, části karet, „Upravit přímo v šuplíku“), přihrádka |
| `vzhled` | Vzhled | `panels/Appearance.tsx` (4 kroky) → `AppearanceClosed.tsx` (+ `StripSettings.tsx` „Co proužek ukazuje“ v Základním; v Pokročilé tvar, linka, krytí), `AppearanceHover.tsx` (v Pokročilé `drawer.hoverExpandMs`), `AppearanceOpen.tsx` (+ `AppearanceStyles`, `AppearanceLayout`), `AppearanceMotion.tsx`; náhledy `settings/previews/*` — mapa [appearance.md](appearance.md) |
| `akce` | Projekty, akce a odkazy | `panels/ProjectsActions.tsx` — skupiny (`Group`): Projekty (`Projects.tsx`, hlavička s přidáním; +`ProjectEditor` = `panels/ProjectEditor.tsx`; tlačítko Z Launcheru… = `settings/launcherImport.ts`) a Odkazy a akce (`Actions.tsx`; jeden seznam, odkaz = akce, `SortList`) otevřené; sbalené Upozornění na neuloženou práci (`GitAlerts`: `gitAlerts.enabled`, ztišená repa s Odztišit) a Složky s repy (`Roots`); `LegacyLinks.tsx` (jen nepřevedené `personal.links`); v **Pokročilé**: `actions/ScriptsSettings.tsx` + `widgets/WidgetsSettings.tsx` |
| `claude` | Claude | `panels/Claude.tsx` (`HooksSetup`; port `claude.port` v **Pokročilé**) + `../mcp/McpSettings.tsx` (Claude (MCP), přepínač hned; Claude Desktop, záloha přes http a výměna klíče v inline **Pokročilé** — [mcp.md](mcp.md)) + `panels/Questions.tsx` (otázky Clauda, `claude.questionHoldS`) |
| `osobni` | Obsah karet (dřív Osobní) | `panels/Personal.tsx`: PO KARTÁCH — skupina (`Group`) na kartu, nahoře karty z rozložení (otevřené), pak ostatní karty a funkce bez karty (sbalené; `personalModel.ts` `splitPersonal`); Počasí s polem Město (`panels/CityPicker.tsx` → `geoApi.ts` → Rust `geo_search`, logika `cityModel.ts`; souřadnice v **Pokročilé**) + `calendar/CalendarSettings` (ICS první), pod ním **Pokročilé: připojit účet Microsoft / Google** = `accounts/AccountsSettings` ([accounts.md](accounts.md)), `media/MediaSettings`, časovač, `cliphist/ClipSettings` (jen řádky, nadpis dává skupina), `integrations/IntegrationsSettings` (GitHub, Systém — [integrations.md](integrations.md)) |
| `poznamky` | Poznámky a lístky | `panels/Notes.tsx` — skupiny Poznámky (složka, kategorie), Lístky na ploše (`PinsRows`) a **Propojení** (synchronizace přes git — repo a interval v inline **Pokročilé**; propojení se složkou `notes/FolderLinkSection.tsx` — přenášení mazání v inline **Pokročilé**, mapa [notes.md](notes.md)) |
| `chovani` | Otevírání a klávesy (dřív Chování) | `panels/Behavior.tsx` (otevírání, šuplík nad prohlížečem, paleta + klávesnice/myš ve 2 sloupcích) + `panels/Hotkeys.tsx` (`HotkeysSection`) |
| `aplikace` | Záloha a verze | `panels/AppPanel.tsx` → `Backup.tsx`, `About.tsx`; v **Pokročilé** `BackupAdvanced` (config.json) a `AboutAdvanced` (repo s vydáními, soubory a log) |

Staré id (`vitejte`, `projekty`, `otazky`, `zkratky`, `zaloha`, `oaplikaci`, `ucty`, `plocha`, `widgety`) vedou přes `PANEL_ALIASES` na
nové (panel Účty od kola 15 není; účty jsou v Osobní pod Kalendáři). Lidské názvy: `drawer.keepWarmS` = „Šetřit paměť“, `drawer.barWindows` = „Šuplík nad prohlížečem“;
staré názvy jsou v `kw` (hledání je najde).

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/settings/placement.ts` | klíč configu → stránka + úroveň Základní / Pokročilé (nejdelší prefix); zdroj `panelForField` a štítku Pokročilé — guard `placement.test.mjs` (`NO_EDITOR` s důvody je v testu) | `PLACEMENT` `placeOf` `ADVANCED_ANCHORS` `isAdvancedField` |
| `src/settings/searchModel.ts` | chytré hledání: synonyma cs/en, klíč configu (i camelCase část), překlep ≤ 1 od 5 znaků, položky jen pro klíč — test `searchModel.test.mjs` | `smartSearch` `SYNONYMS` `synonymsOf` `keyWords` `nearlyEqual` |
| `src/settings/ctx.ts` | rozpracovaný config (store na úrovni modulu), `base`, chyba, hit, Vrátit, úkoly po uložení, kontext pro pole | `cfg` `loadBase` `adoptBase` (změna zvenku sloučená s draftem, `mcp/merge.ts`) `discard` `setCfg` `makeCtx` `useSettings` `offerUndo` `afterSave` `runAfterSave` `dirty` `active` |
| `src/settings/registry.ts` · `catalog.ts` · `labels.ts` | stránky bez komponent (názvy, kw — čte i paleta) a staré id · stránky s komponentami + položky hledání (volby s klíčem a štítkem Pokročilé, prefixy klíčů) · popisky voleb (JEDINÝ zdroj pro `Row` i hledání; i volby panelu Vzhled), texty datových párů | `PANEL_META` `PANEL_ALIASES` · `PANELS` `searchItems` · `FIELDS` `fieldLabel` `fieldHint` `fieldKw` `fieldCs` `fieldAnchor` `tr` |
| `src/settings/en.ts` · `panels/en.ts` | angličtina (`registerDict`, líně načítá `src/i18n/dicts.ts`): skořápka, nav, volby `settings.f.<id>`/`.hint`/`.kw`, sekce configu · texty panelů (vč. `appearance.*`, `actions.panel.*`, `questions.panel.*`); Průvodce má `settings.welcome.*` v `onboarding/en.ts` | |
| `src/settings/model.ts` | čistá logika: hledání bez diakritiky (`search` = prosté, skořápka používá `searchModel.smartSearch`), cesty, chyba→stránka (`placement.ts`: `drawer` → Otevírání, `drawer.glass`/`drawer.width`/`strip` → Vzhled, `personal.links` → akce, `desk` → Na ploše), skok odjinud (`resolveTarget`: id panelu / staré id / kotva `f-<pole>`), přesuny, zkratky, porovnání | `search` `highlight` `panelForField` `resolveTarget` `moveItem` `dropIndex` `normalizeHotkey` `hotkeyFromEvent` `hotkeyCollisions` `canon` `cleanDraft` `inRoots` `relativeTo` |
| `src/settings/fields/*` (`fields/{Hotkey,Row,SortList}.tsx`; společný tvar panelů `settings/types.ts`, ikony panelů `settings/icons.tsx`) | `Row`/`Section`/`PanelHead`/`FieldErr` (`id` = kotva `f-<id>` i u sekce a hlavičky; `Row live` + `LiveBadge` = štítek „Platí hned“), `Advanced.tsx` (sdílené sbalitelné „Pokročilé“ — viz Invarianty), ovládání (`controls.tsx`; `Seg` = 2–4 volby), `SortList` (pointer drag + Alt+šipky), `HotkeyInput`, `Confirm` | |
| `src/settings/fields/Group.tsx` | sbalovací skupina voleb (kolo 21): nadpis + věta, výchozí `open`, `badge`, `aside` (tlačítko v hlavičce nesbalí), kotva `anchor`; skok (`hit`) rozbalí; obsah připojený i sbalený (`hidden`); paměť `advModel.ts` `GROUP_KEY` (try/catch) | `Group` |
| `src/settings/panels/Startup.tsx` | stránka Spouštění a hry (Aplikace): autostart s bublinou MSIX, Šetřit paměť, hry a herní režim | `StartupPanel` |
| `src/settings/appearanceModel.ts` | panel Vzhled jako čistá data: kroky a kde leží která kotva (i „Pokročilé“), volby přepínačů (≤ 4), monitor ve skutečném poměru (šířka = zrcadlo Rust `preset_width`, pozice = `anchor_c2`, sloupce rozložení) | `STEPS` `ANCHOR_STEP` `ADVANCED` `stepOf` `isAdvanced` `WIDTH_STEPS` `DENSITY_STEPS` `PREVIEW_STATES` `drawerWidth` `drawerLeft` `monitorModel` `sampleSpans` |
| `src/settings/panels/*` | panely a jejich sekce — tabulka **Panely** výš; každý panel jen mění draft (`PanelProps` bez `save`); proužek čistou logikou `settings/strip.ts` (+ `strip.test.mjs`); bublina MSIX `pkgctx/model.ts` (+ `model.test.mjs`); Osobní záložku nevypíná (řídí `layout`) | |
| `src/settings/advModel.ts` · `personalModel.ts` · `cityModel.ts` | čistá logika bez DOM: paměť rozbalení Pokročilé (`parseAdvOpen` `withAdv`) a skupin (`parseGroups` `groupOpen` `withGroup`), pořadí skupin Obsahu karet (`layoutCardIds` `splitPersonal`: shown · more · helpers), pole Město (`geoQuery` `placeLabel` `placeToWeather`, zrcadlo Rust `weather::clean_query`), „Co je nového“ po upgradu (`upgradedFrom` `cmpVersion`) | |
| `src/settings/widen.ts` | širší šuplík pro Nastavení a přechod podle stylu pohybu (`theme/appearance.ts` `widenMotion`; none / omezit pohyb = okamžitě) | `SETTINGS_WIDE` `wantWidth` `holdWide` `widenIn` `narrowOut` |
| `src/fit.ts` · `fit.css` | `ViewMsg.maxH` → `--drawer-max-h`; tělo stránky se posouvá uvnitř (všechny stránky) | |
| `src-tauri/src/commands/settings.rs` | příkazy Nastavení | `check_config` `config_defaults` `config_export` `config_import_preview` `pick_path` `pick_save` `project_detect` `project_candidates` `logo_preview` `app_info` `open_app_file` `launcher_catalog_read` (jen soubor, který právě vrátil `pick_path("json")` — jednorázová stopa `PICKED_JSON`; `.json` ≤ 2 MB, ne odkaz) `launcher_import_check` (příkazy zástupců přes `runguard`) |
| `src/settings/launcherImport.ts` | import zástupců z katalogu Launcheru (kolo 18): `items[]` → projekty (dev z `web.run`, app z první `actions[].exec`, vypnuté = `enabled: false`, bez příkazu přeskočit, duplicita id / složky / remote přeskočit), rodiny → složky zástupců s názvem z `families[].label`; příkazy přes pojistku v jádře, co neprojde = vypnuté (`withVerdicts`), před převzetím `ui/Confirm` se seznamem příkazů (`commandLines`); jen do návrhu (Uložit / Vrátit). Test `launcherImport.test.mjs` | `importLauncherCatalog` `projectOf` `folderId` `folderName` `commandsOf` `withVerdicts` `commandLines` |
| `src-tauri/src/configerr.rs` | chyba s cestou k poli | `FieldError` `fe` `field_from_message` |
| `src-tauri/src/configio.rs` | export, náhled importu po sekcích (rizikové), nic nezapisuje; pojistka zápisu | `section_diff` `preview` `read_import` `export` `RISKY` `keep_unreadable` |
| `src-tauri/src/projectdetect.rs` | návrh projektu ze složky, naklonovaná repa v roots | `detect` `guess_port` `make_id` `candidates` |
| `src-tauri/src/commands/geo.rs` · `weather.rs` (dole) | hledání města pro pole Město: `geo_search(query) -> Vec<Place>`; `parse_places` / `clean_query` / `geo_url` čisté (test na `fixtures/open-meteo-geocoding.json`), `search_places` async 5 s | `geo_search` `Place` `parse_places` `clean_query` `geo_url` `search_places` |
| `src-tauri/src/drawerfit.rs` | strop výšky (80 % pracovní plochy, nejvýš do − 20 px; vlna 9) a šířka Full | `limits` `fit_full` `HEIGHT_FRACTION` `BOTTOM_GAP` `WIDE_FRACTION` |

## Toky

- **Úprava**: pole → `ctx.set(path)` → `touch` → `check` (250 ms): `canon(draft) != canon(base)` = neuloženo,
  `check_config` → `FieldError` → `Row`/`FieldErr` u pole, tečka u panelu v navigaci.
- **Změna zvenku** (Claude přes MCP, přepínač MCP): jádro pošle `config-changed` → `Settings` `adoptExternal` →
  `mcp/merge.ts` `external`: bez úprav `loadBase`, úpravy v jiné sekci `adoptBase` (sloučení), stejná sekce = konflikt.
- **Uložení** (`save`): `get_config` ≠ `base` → konflikt (Načíst nové / Přesto uložit) → `check_config` →
  `save_config` (String chyba, zpětně kompatibilní) → `runAfterSave` (odložené úkoly panelů) → `loadBase(get_config)`.
- **Jedno chování ukládání ve všech panelech:** panel mění jen draft, zapisuje JEN lišta shellu (Uložit / Ctrl+S,
  Zahodit, Vrátit v liště přes `offerUndo`). `PanelProps` nemá `save` — žádné vlastní „Uložit vzhled“ ani okamžité
  ukládání (Vzhled, Rychlé akce, Otázky Clauda). Co jádro přijme až pro uloženou verzi (souhlas s příkazem akce
  `action_consent`), panel odloží `afterSave(fn)`; `loadBase` (Zahodit, Načíst nové) frontu zahodí. Rychlé akce:
  „Spustit“ jen pro akci uloženou přesně v tomhle tvaru (`isSaved` proti `base`), editor akce má „Hotovo“ (do draftu).
- **Import**: `pick_path(json)` → `config_import_preview` (ověří celý soubor) → výběr sekcí (rizikové
  nezaškrtnuté) → do draftu + Vrátit → zapíše až Uložit. **Reset**: `config_defaults()[sekce]` (ne `projects`).
- **Skok odjinud** (`palette/goto.ts` `gotoSettingsPanel(id)`, volá se PO `setPage("settings")`): `Settings` čte
  `settingsTarget` → `model.resolveTarget` — id panelu (i staré přes `PANEL_ALIASES`) přepne panel, kotva
  `f-<pole>` přepne na panel pole (`labels.ts` `FIELDS`, jinak `panelForField`) a posune + zvýrazní řádek
  (`reveal`); cíl pak vynuluje. Volající: karty (Odkazy `f-actions.add`, Rychlé akce `akce`, Kalendář,
  Počasí, Claude `f-claude.hooks` / `f-claude.statusLine`), hlavička (bublina `f-autostart`), paleta (panely
  z `PANEL_META`, schránka `f-clipboard.enabled`), Začínáme (`go: …`) — všechny hlídá test.
- **Jazyk** (`ui.lang`, přepínač úplně nahoře v panelu Začínáme — kotva `f-ui.lang`): draft → Uložit → `loadBase` → `setLang`
  (UI se přepne hned, bez restartu; `loadBase` bere jazyk vždy z ULOŽENÉ verze). Hledání (`searchItems`) jde
  v textu aktuálního jazyka (popisky přes `t()`, klíčová slova `settings.f.<id>.kw`); mimo češtinu hledá i
  v české podobě (`fieldCs`). Memo v `Settings` čte `lang()`, přepočítá se samo.
- **Šířka/výška**: `App.report` posílá `wantWidth(page)`; Rust `Drawer::logical_size` → `drawerfit::fit_full`;
  `ViewMsg.max_h` → `fit.ts`. `Drawer::hide` vrací `full_w` na 0.

## Invarianty a pasti

- `config::save` (jediný zápis configu): sériově, vlastní tmp na zápis, nečitelný cíl nejdřív zkopíruje
  (`configio::keep_unreadable` → `config.json.broken-<s>`). Rozbitý config při startu = v paměti výchozí
  hodnoty → bez té kopie by „Uložit“ v Nastavení smazalo uživatelovy projekty.
- Validace je JEN v Rustu (`config::validate_field`); `config::validate` je obálka vracející text.
  Validace jiných modulů vracející `String` dostanou pole z prefixu zprávy (`sekce.pole: …`).
- Dialogy volá jen Rust (`DialogExt`, rodič = okno šuplíku → blur ho nezavře); webview nemá žádné
  `dialog:*` oprávnění a `@tauri-apps/plugin-dialog` se nepoužívá (node_modules sdílené s hlavním repem).
- Draft přežije zavření stránky (modul); zničení WebView v úsporném režimu ho zahodí.
- **Stránky drží stav (kolo 21):** navštívená stránka se při přepnutí jen skryje (`hidden`), nikdy znovu
  nepostaví — žádné `<For each={[panel()]}>` + `<Dynamic>` (hlídá `structure.test.mjs`). Vzhled tak zůstane
  připojený a jeho rozpracovaný vzhled platí na šuplíku, dokud je Nastavení otevřené (přepnutí stránky
  už šuplík nepřebarví zpět). Záložky šuplíku (`layout/Tabs.tsx`, `Deck.tsx`) ten vzor nemají: `For` přes
  id záložek / karet, karta společná dvěma záložkám zůstává; skryté záložky naživu záměrně NEdržíme
  (karty pak nic nepočítají).
- Nastavení bere fokus (`focus_drawer`) — otevírá se jen klikem; `nav.typing` obsahuje `settings`.
- **Ovládání se vždy vejde:** segmentový přepínač má nejvýš 4 volby (víc = galerie dlaždic nebo rozbalovací;
  hlídá `segments.test.mjs` nad celým `src`, výjimky s důvodem v `EXEMPT`). Řádek `.st-row` / `.opt` je flex
  se zalomením: nevejde-li se ovládání vedle popisku, spadne pod něj (popisek drží ~16 em, nikdy se nezmáčkne);
  v úzké sekci (`@container`) jde zalomené ovládání vlevo.
- **Skok na kotvu v panelu s kroky** (Vzhled): `reveal` nastaví `hit` HNED, panel podle něj přepne krok a
  rozbalí Pokročilé (`appearanceModel.stepOf`/`isAdvanced`), řádek se hledá až po dvou snímcích.
- HTML5 drag&drop v okně Tauri nefunguje (chytá ho přetahování souborů) → `SortList` přes pointer.
- **Každá volba na jednom místě.** Vzhled šuplíku (pozadí/akryl `drawer.glass`, šířka `drawer.width`,
  animace `appearance.motion`) a CELÝ proužek (styl, pozice, krytí, položky, barvy, vlastní text, monitor)
  jsou JEN v panelu Vzhled (i `drawer.hoverExpandMs`, krok Po najetí → Pokročilé); otevírání (spouštěč,
  prodlevy, zkratky, paleta, `drawer.barWindows`) JEN v Otevírání a klávesy; autostart, `drawer.keepWarmS`,
  hry a herní režim JEN v Aplikace → Spouštění a hry. „Šetřit paměť“ = `drawer.keepWarmS` (uvolnění paměti
  WebView), ne animace. Odkazy = akce (jeden seznam, `actions/model.ts` `isLink`); `personal.links` jen
  pro nepřevedené (`linkmigrate.rs`). Hlídá `settings.test.mjs` (kotva právě v jednom panelu, ve svém;
  složení panelů z importů).
- **Pokročilé bez strachu** (kolo 15): základ každého panelu je pro člověka, který technice nerozumí; porty, repa,
  příkazy, klíče a skripty jsou ve sdíleném `fields/Advanced.tsx` (méně časté, ale ne technické volby patří do
  `fields/Group.tsx`, ne do Pokročilé — „Co proužek ukazuje“ i karty Obsahu karet jsou v Základním; jedna implementace; `<details>`, rozbalení se
  pamatuje po `id` v localStorage `ledge.settings.advanced` s try/catch; Vzhled má vlastní kroky v `apUi.tsx`).
  Skok z hledání / odjinud / chyby validace (`hit`) oddíl rozbalí, pokud v něm kotva `f-<id>` je (hledá se v DOM
  oddílu), do paměti to nezapíše. Kotvy zůstávají v souborech svého panelu, takže `settings.test.mjs` platí beze změny.
  Volby bez lišty Uložit (zapnutí MCP, hooky, stavový řádek, přihlášení účtu, povolení widgetu) mají štítek
  `LiveBadge`. Pole Město volá `geo_search` jen z časovače po psaní (≥ 2 znaky, 450 ms), nikdy v klidu.
- Barvy jen přes tokeny `styles.css` (motivy APPEARANCE); `.switch` v `settings.css` používá i NotesPage.

## Testy

`configerr::tests` (cesty všech chyb validace — tabulka), `configio::tests` (diff, import ověří před
čímkoli, export, záloha nečitelného configu, soubor jen pro čtení, souběžné zápisy), `projectdetect::tests` (port, id, web/podsložka/exe, kandidáti), `drawerfit::tests`
(monitory, DPI 100/125/150 %, panel dole/nahoře). Frontend: `npx tsc --noEmit`; `corepack pnpm test:ui` →
`src/settings/appearanceModel.test.mjs` (každá volba Vzhledu má krok, šířka = tabulka Rust `preset_width_table`, pozice, sloupce),
`src/settings/segments.test.mjs` (žádný přepínač > 4 volby; mutace ověřená), `src/settings/placement.test.mjs` (každý klíč configu má místo nebo důvod; úroveň Pokročilé sedí s panelem), `src/settings/searchModel.test.mjs` (tabulka hledání), `src/settings/structure.test.mjs` (sekce, Doporučené, stránky naživu, pevná výška), `src/settings/advanced.test.mjs` (paměť rozbalení Pokročilé i skupin, technické volby v Pokročilé, Účty dohledatelné, štítek Platí hned), `src/settings/personal.test.mjs` (skupiny Obsahu karet podle rozložení, Město, verze a „Co je nového“ jen v Doporučených), `src/settings/settings.test.mjs` (každá volba z `labels.ts` má kotvu `f-<id>` ve svém panelu a v žádném
jiném; soubory panelu z importů jeho komponenty a žádného jiného; všechny `gotoSettingsPanel`/`onSettings`/
`go:` cíle v `src` vedou na existující panel a kotvu v něm; staré id; hledání starých názvů bez diakritiky;
chyba pole → panel); Rust `linkmigrate::tests` (skutečný tvar starého configu přes `config::load`,
idempotence, stropy); vizuál naživo.

## Co nečíst

`panels/*` mimo panel, který měníš; `settings.css` celé (hledej třídu `st-…`).
