# Codemapa: Nastavení (shell, panely, záloha, strop výšky šuplíku)

Čti tohle místo celých souborů. Cituj symbolem, ne číslem řádku. Sekce jiných oblastí (Vzhled, akce,
otázky Clauda, rozložení, schránka, přehrávač, kalendář) jen skládají panely — jejich mapy jsou u nich.

## Panely (od vlny 6; id v `registry.ts` `PANEL_META`, komponenta v `catalog.ts` `COMPONENTS`)

| id | panel | skládá (soubory) |
|---|---|---|
| `zaciname` | Začínáme | `panels/Start.tsx` — průvodce (`onboarding.wizard`), „Upravit šuplík“, tipy, co je nového (odkazy `gotoSettingsPanel`) |
| `suplik` | Šuplík a karty | `panels/Drawer.tsx` → `layout/LayoutEditor.tsx` (záložky, karty, části karet, „Upravit přímo v šuplíku“), přihrádka |
| `vzhled` | Vzhled | `panels/Appearance*.tsx` + `panels/StripSettings.tsx` (celý proužek: co ukazuje, barvy, klid, vlastní text, časovač, monitor) |
| `akce` | Projekty, akce a odkazy | `panels/ProjectsActions.tsx` → `Projects.tsx` (+`ProjectEditor`), `Actions.tsx` (jeden seznam; odkaz = akce, `SortList`), `actions/ScriptsSettings.tsx`, `LegacyLinks.tsx` (jen nepřevedené `personal.links`) |
| `claude` | Claude | `panels/Claude.tsx` (`HooksSetup`) + `panels/Questions.tsx` (otázky Clauda, `claude.questionHoldS`) |
| `osobni` | Osobní | `panels/Personal.tsx` + `calendar/CalendarSettings`, `media/MediaSettings`, časovač, `cliphist/ClipSettings` |
| `poznamky` | Poznámky | `panels/Notes.tsx` (kategorie a sync jsou rozsáhlé → vlastní panel) |
| `chovani` | Chování | `panels/Behavior.tsx` + `panels/Hotkeys.tsx` (`HotkeysSection`) + `pkgctx/BubbleNotice.tsx` |
| `aplikace` | Záloha a aplikace | `panels/AppPanel.tsx` → `Backup.tsx`, `About.tsx` |

Staré id (`vitejte`, `projekty`, `otazky`, `zkratky`, `zaloha`, `oaplikaci`) vedou přes `PANEL_ALIASES` na
nové. Lidské názvy: `drawer.keepWarmS` = „Šetřit paměť“, `drawer.barWindows` = „Šuplík nad prohlížečem“;
staré názvy jsou v `kw` (hledání je najde).

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/Settings.tsx` | skořápka: navigace, hledání, panel, lišta Uložit/Zahodit, konflikt, ověření | `Settings` `save` `check` `jump` `reveal` `go` `navKey` |
| `src/settings/ctx.ts` | rozpracovaný config (store na úrovni modulu), `base`, chyba, hit, Vrátit, úkoly po uložení, kontext pro pole | `cfg` `loadBase` `discard` `setCfg` `makeCtx` `useSettings` `offerUndo` `afterSave` `runAfterSave` `dirty` `active` |
| `src/settings/registry.ts` · `catalog.ts` · `labels.ts` | panely bez komponent (pořadí, názvy, kw — čte i paleta) a staré id · panely s komponentami + hledání · popisky voleb (JEDINÝ zdroj pro `Row` i hledání; i volby panelu Vzhled) | `PANEL_META` `PANEL_ALIASES` · `PANELS` `searchItems` · `FIELDS` `fieldLabel` `fieldHint` `fieldKw` `fieldCs` `fieldAnchor` |
| `src/settings/en.ts` · `panels/en.ts` | angličtina (`registerDict`, načítá `src/i18n/load.ts`): skořápka, nav, volby `settings.f.<id>`/`.hint`/`.kw`, sekce configu · texty panelů (vč. `appearance.*`, `actions.panel.*`, `questions.panel.*`); Průvodce má `settings.welcome.*` v `onboarding/en.ts` | |
| `src/settings/model.ts` | čistá logika: hledání bez diakritiky, cesty, chyba→panel (nejdelší prefix: `drawer` → chování, `drawer.glass`/`drawer.width`/`strip` → vzhled, `personal.links` → akce), skok odjinud (`resolveTarget`: id panelu / staré id / kotva `f-<pole>`), přesuny, zkratky, porovnání | `search` `highlight` `panelForField` `resolveTarget` `moveItem` `dropIndex` `normalizeHotkey` `hotkeyFromEvent` `hotkeyCollisions` `canon` `cleanDraft` `inRoots` `relativeTo` |
| `src/settings/fields/*` | `Row`/`Section`/`PanelHead`/`FieldErr` (`id` = kotva `f-<id>` i u sekce a hlavičky), ovládání (`controls.tsx`), `SortList` (pointer drag + Alt+šipky), `HotkeyInput`, `Confirm` | |
| `src/settings/panels/*` | panely a jejich sekce — tabulka **Panely** výš; každý panel jen mění draft (`PanelProps` bez `save`); proužek čistou logikou `settings/strip.ts` (+ `strip.test.mjs`); bublina MSIX `pkgctx/model.ts` (+ `model.test.mjs`); Osobní záložku nevypíná (řídí `layout`) | |
| `src/settings/widen.ts` | širší šuplík pro Nastavení a přechod podle stylu pohybu (`theme/appearance.ts` `widenMotion`; none / omezit pohyb = okamžitě) | `SETTINGS_WIDE` `wantWidth` `holdWide` `widenIn` `narrowOut` |
| `src/fit.ts` · `fit.css` | `ViewMsg.maxH` → `--drawer-max-h`; tělo stránky se posouvá uvnitř (všechny stránky) | |
| `src-tauri/src/commands/settings.rs` | příkazy Nastavení | `check_config` `config_defaults` `config_export` `config_import_preview` `pick_path` `pick_save` `project_detect` `project_candidates` `logo_preview` `app_info` `open_app_file` |
| `src-tauri/src/configerr.rs` | chyba s cestou k poli | `FieldError` `fe` `field_from_message` |
| `src-tauri/src/configio.rs` | export, náhled importu po sekcích (rizikové), nic nezapisuje; pojistka zápisu | `section_diff` `preview` `read_import` `export` `RISKY` `keep_unreadable` |
| `src-tauri/src/projectdetect.rs` | návrh projektu ze složky, naklonovaná repa v roots | `detect` `guess_port` `make_id` `candidates` |
| `src-tauri/src/drawerfit.rs` | strop výšky (pracovní plocha − 20 px) a šířka Full | `limits` `fit_full` `BOTTOM_GAP` `WIDE_FRACTION` |

## Toky

- **Úprava**: pole → `ctx.set(path)` → `touch` → `check` (250 ms): `canon(draft) != canon(base)` = neuloženo,
  `check_config` → `FieldError` → `Row`/`FieldErr` u pole, tečka u panelu v navigaci.
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
- **Jazyk** (`ui.lang`, přepínač v panelu Vzhled — kotva `f-ui.lang`): draft → Uložit → `loadBase` → `setLang`
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
- Nastavení bere fokus (`focus_drawer`) — otevírá se jen klikem; `nav.typing` obsahuje `settings`.
- HTML5 drag&drop v okně Tauri nefunguje (chytá ho přetahování souborů) → `SortList` přes pointer.
- **Každá volba na jednom místě.** Vzhled šuplíku (pozadí/akryl `drawer.glass`, šířka `drawer.width`,
  animace `appearance.motion`) a CELÝ proužek (styl, pozice, krytí, položky, barvy, vlastní text, monitor)
  jsou JEN v panelu Vzhled; chování (otevírání, zkratky, paleta, herní režim, `drawer.barWindows`,
  autostart, `drawer.keepWarmS`) JEN v Chování. „Šetřit paměť“ = `drawer.keepWarmS` (uvolnění paměti
  WebView), ne animace. Odkazy = akce (jeden seznam, `actions/model.ts` `isLink`); `personal.links` jen
  pro nepřevedené (`linkmigrate.rs`). Hlídá `settings.test.mjs` (kotva právě v jednom panelu, ve svém;
  složení panelů z importů).
- Barvy jen přes tokeny `styles.css` (motivy APPEARANCE); `.switch` v `settings.css` používá i NotesPage.

## Testy

`configerr::tests` (cesty všech chyb validace — tabulka), `configio::tests` (diff, import ověří před
čímkoli, export, záloha nečitelného configu, soubor jen pro čtení, souběžné zápisy), `projectdetect::tests` (port, id, web/podsložka/exe, kandidáti), `drawerfit::tests`
(monitory, DPI 100/125/150 %, panel dole/nahoře). Frontend: `npx tsc --noEmit`; `corepack pnpm test:ui` →
`src/settings/settings.test.mjs` (každá volba z `labels.ts` má kotvu `f-<id>` ve svém panelu a v žádném
jiném; soubory panelu z importů jeho komponenty a žádného jiného; všechny `gotoSettingsPanel`/`onSettings`/
`go:` cíle v `src` vedou na existující panel a kotvu v něm; staré id; hledání starých názvů bez diakritiky;
chyba pole → panel); Rust `linkmigrate::tests` (skutečný tvar starého configu přes `config::load`,
idempotence, stropy); vizuál naživo.

## Co nečíst

`panels/*` mimo panel, který měníš; `settings.css` celé (hledej třídu `st-…`).
