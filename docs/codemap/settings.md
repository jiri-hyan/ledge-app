# Codemapa: Nastavení (shell, panely, záloha, strop výšky šuplíku)

Čti tohle místo celých souborů. Cituj symbolem, ne číslem řádku. Panely jiných oblastí (Průvodce,
Vzhled, Rychlé akce, Otázky) jen vykresluje shell — jejich mapy jsou u nich.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/Settings.tsx` | skořápka: navigace, hledání, panel, lišta Uložit/Zahodit, konflikt, ověření | `Settings` `save` `check` `jump` `reveal` `go` `navKey` |
| `src/settings/ctx.ts` | rozpracovaný config (store na úrovni modulu), `base`, chyba, hit, Vrátit, kontext pro pole | `cfg` `loadBase` `discard` `setCfg` `makeCtx` `useSettings` `offerUndo` `dirty` `active` |
| `src/settings/catalog.ts` · `labels.ts` | pořadí panelů (vlastní + `EXTERNAL_PANELS`) · popisky voleb (JEDINÝ zdroj pro `Row` i hledání; i volby panelu Vzhled) | `PANELS` `searchItems` · `FIELDS` `fieldLabel` `fieldHint` `fieldKw` `fieldCs` `fieldAnchor` |
| `src/settings/en.ts` · `panels/en.ts` | angličtina (`registerDict`, načítá `src/i18n/load.ts`): skořápka, nav, volby `settings.f.<id>`/`.hint`/`.kw`, sekce configu · texty panelů (vč. `appearance.*`, `actions.panel.*`, `questions.panel.*`); Průvodce má `settings.welcome.*` v `onboarding/en.ts` | |
| `src/settings/model.ts` | čistá logika: hledání bez diakritiky, cesty, chyba→panel (nejdelší prefix: `drawer.glass`/`drawer.width`/`strip.opacity` → vzhled), přesuny, zkratky, porovnání | `search` `highlight` `panelForField` `moveItem` `dropIndex` `normalizeHotkey` `hotkeyFromEvent` `hotkeyCollisions` `canon` `cleanDraft` `inRoots` `relativeTo` |
| `src/settings/fields/*` | `Row`/`Section`/`PanelHead`/`FieldErr` (`id` = kotva `f-<id>` i u sekce a hlavičky), ovládání (`controls.tsx`), `SortList` (pointer drag + Alt+šipky), `HotkeyInput`, `Confirm` | |
| `src/settings/panels/*` | Projects (+`ProjectEditor`, `ProjectLogo`), Drawer (jen CHOVÁNÍ: otevírání, karty, proužek, okna s taby, autostart, uvolnění paměti), Claude (`HooksSetup`), Notes, Personal, Hotkeys, Backup, About | |
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
  `save_config` (String chyba, zpětně kompatibilní) → `loadBase(get_config)`.
- **Import**: `pick_path(json)` → `config_import_preview` (ověří celý soubor) → výběr sekcí (rizikové
  nezaškrtnuté) → do draftu + Vrátit → zapíše až Uložit. **Reset**: `config_defaults()[sekce]` (ne `projects`).
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
  animace `appearance.motion`, styl a pozice proužku, krytí `strip.opacity`) je JEN v panelu Vzhled;
  „Úsporný režim“ = `drawer.keepWarmS` (uvolnění paměti WebView), ne animace — v UI „Uvolnit paměť
  při nečinnosti“. Hlídá `settings.test.mjs` (kotva právě v jednom panelu, ve svém).
- Barvy jen přes tokeny `styles.css` (motivy APPEARANCE); `.switch` v `settings.css` používá i NotesPage.

## Testy

`configerr::tests` (cesty všech chyb validace — tabulka), `configio::tests` (diff, import ověří před
čímkoli, export, záloha nečitelného configu, soubor jen pro čtení, souběžné zápisy), `projectdetect::tests` (port, id, web/podsložka/exe, kandidáti), `drawerfit::tests`
(monitory, DPI 100/125/150 %, panel dole/nahoře). Frontend: `npx tsc --noEmit`; `corepack pnpm test:ui` →
`src/settings/settings.test.mjs` (každá volba z `labels.ts` má kotvu `f-<id>` ve svém panelu a v žádném
jiném, chyba pole → panel); vizuál naživo.

## Co nečíst

`panels/*` mimo panel, který měníš; `settings.css` celé (hledej třídu `st-…`).
