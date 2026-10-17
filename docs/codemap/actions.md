# Codemapa: rychlé akce a paleta příkazů

Config `actions` → karta „Rychlé akce“, panel Nastavení „akce“ a paleta (`Ctrl+K` / 🔍). Formát pro
agenty: `AGENTS.md` → „Rychlé akce“. Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/actions.rs` | tvar configu, validace (tvar, bez disku), čistá rozhodnutí | `Action`, `ActionKind`, `ShowOn`, `Shell`, `validate`, `validate_one`, `fingerprint`, `needs_confirm`, `Action::risky`, `command_line`, `confirm_text`, `monogram_color`, `MAX_ACTIONS` |
| `src-tauri/src/actionrun.rs` | kam akce vede, spuštění, souhlasy, běžící | `resolve`, `Launch`, `check_fs`, `work_dir`, `shell_cmd`, `run_hidden`, `run_terminal`, `run_exe`, `tail`, `Outcome`, `Consents` (`has`, `has_cached`, `grant`), `CONSENTS`, `RUNNING`, `RunGuard`, `local_dir` |
| `src-tauri/src/actionicons.rs` | odkud ikona, favicon (síť), disková cache | `spec`, `Spec`, `origin`, `join_href`, `candidates`, `cached`/`store`/`forget`, `local`, `fetch_favicon`, `cache_dir` |
| `src-tauri/src/actionstate.rs` | stav pro UI: ikony + varování, vlákno `action-icons`, snapshot | `STORE`, `refresh`, `compute`, `icon_url`, `views`, `ActionView` |
| `src-tauri/src/commands/settings.rs` | „Vybrat…“: společný `pick_path` (dialog plugin, i pro Nastavení) | `pick_path` |
| `src-tauri/src/commands/actions.rs` | Tauri příkazy | `run_action`, `action_icon`, `action_icon_refresh`, `action_consent`, `action_check`, `pick_path` |
| `src-tauri/src/iconimg.rs` | + ikona souboru/složky ze shellu (48 px) | `shell_png` |
| `src/actions/api.ts` | invoke, cache ikon podle `iconKey`, spuštění s potvrzením a výsledkem | `actionsApi`, `actionIconSrc`, `runAction`, `isBusy`, `lastResult` |
| `src/actions/model.ts` | čistá logika UI | `slugId`, `moveItem`, `guessKind`, `linksToActions`, `visibleGroups`, `cleanAction`, `draftError` |
| `src/actions/ui.tsx` | ikona/monogram, potvrzení v šuplíku, výstup příkazu | `ActionIcon`, `askConfirm`, `ConfirmAction`, `answerConfirm`, `confirmPending`, `LastOutput` |
| `src/actions/ActionEditor.tsx` · `src/settings/panels/Actions.tsx` | editor akce · panel (seznam, přetažení, Vrátit, převod odkazů) | `ActionEditor`, `kindLabel` · `ActionsPanel` |
| `src/cards/ActionsCard.tsx` | karta (Práce bez akcí se neukáže, Osobní ukáže pozvánku) | `ActionsCard` |
| `src/palette/search.ts` | čisté hledání: bez diakritiky, fuzzy, zvýraznění, nedávné | `fold`, `score`, `segments`, `rank`, `pushRecent` |
| `src-tauri/src/scripts/mod.rs` | Script Commands: config `scripts`, složka (`resolve_dir_with`), sken bez watcheru (stat + jen změněné, strop 200, 1 MB), pohled pro UI, stav v paměti | `ScriptsConfig`, `scan_dir`, `load_one`, `Scan`, `Script` (`fingerprint`, `confirm_text`), `ScriptView`, `view`, `problem`, `refresh`, `refresh_async`, `snapshot_views`, `fresh`, `Listing`, `MAX_SCRIPTS`, `MAX_FILE` |
| `src-tauri/src/scripts/meta.rs` | čistá hlavička `@ledge.*` (komentáře podle jazyka, prvních 4 KB) | `parse`, `header_text`, `split_args`, `Meta`, `Mode`, `Lang`, `HEADER_BYTES` |
| `src-tauri/src/scripts/run.rs` | interpret z PATH (bez aliasů Storu), program + argumenty, otisk souhlasu, text potvrzení | `find_on_path`, `interp`, `Interp`, `plan`, `Plan`, `fingerprint`, `confirm_text`, `sha256_hex` |
| `src-tauri/src/scripts/tests.rs` | sken v tempdir, cache, stropy, pohled, skutečný cmd / PowerShell | — |
| `src-tauri/src/winsettings.rs` | tvar cíle „Nastavení Windows“ (`ms-settings:` / `control:Microsoft.`), cesta k `control.exe` | `parse`, `WinTarget`, `control_exe` |
| `src-tauri/src/commands/scripts.rs` | Tauri příkazy skriptů a nastavení Windows | `scripts_list`, `run_script`, `scripts_open_dir`, `win_settings_open` |
| `src/actions/scripts.ts` · `ScriptsSettings.tsx` | invoke skriptů + `runAny` (akce i skript stejným tokem) · karta Skripty v panelu Rychlé akce (přepínače, složka, nalezené, formát) | `scriptsApi`, `runAny` · `ScriptsSettings` |
| `src/palette/winsettings.ts` | statická tabulka stránek Nastavení Windows (jen dokumentované, CZ+EN slova přes `tIn`), položky jen s dotazem | `WIN_PAGES`, `winItems`, `WinPage` |
| `src/palette/more.ts` | položky palety mimo snapshot: skripty (`scripts_list` při otevření) + Nastavení Windows | `scriptItems`, `windowsItems` |
| `src/palette/compact.ts` | kompaktní paleta (`palette.compact`): co ukázat pod polem podle režimu a dotazu, šipky (rozbalit ↓ / sbalit ↑) | `shown`, `visible`, `arrow`, `Shown`, `Fold` |
| `src-tauri/src/paletteconf.rs` | sekce configu `palette` → `Snapshot.palette_compact` (`paletteCompact`) | `PaletteConfig` |
| `src/palette/items.ts` · `Palette.tsx` · `goto.ts` | položky ze snapshotu · UI palety · skok na panel Nastavení | `buildItems`, `PItem`, `Nav` · `Palette`, `IconSearch` · `gotoSettingsPanel`, `settingsTarget` |
| `src/types.ts` (blok `// --- actions ---`) | typy + `Snapshot.actions`/`scripts`/`paletteWinSettings` (sloučení rozhraní) | `ActionCfg`, `ActionView`, `ScriptView`, `ScriptsListing`, `ActionOutcome` |

Lepidlo: `config.rs` (`Config.actions` + `validate`), `snapshot.rs` (`Snapshot.actions`),
`settings.rs` (`apply_config` → `actionstate::refresh`), `lib.rs` (start + `generate_handler!`),
`MainPage.tsx` (karta), `Head.tsx` (🔍), `App.tsx` (signál palety, `Ctrl+K`, `palNav`). Skripty:
`config.rs` (`Config.scripts` + `scripts::validate`), `snapshot.rs` (`scripts`, `palette_win_settings`),
`settings.rs` (změna `scripts` → `refresh_async`), `threads/ui.rs` (otevření šuplíku s `inActions` →
`refresh_async`), `i18n.ts` (`tIn`), `i18n/load.ts` (`en-winsettings`).

## Toky

- **Spuštění**: dlaždice/paleta → `runAction` (problém → hláška; `needsConfirm` → `ConfirmAction`) →
  `run_action(id, target, confirmed)` → znovu najde akci (config se mohl změnit), ověří potvrzení
  proti `confirm_text`, `resolve(check_fs)` → opener (web/složka/soubor/.exe) / `run_exe` /
  `run_terminal` / `run_hidden` (`RunGuard`). Otevření zavře šuplík, skrytý příkaz ne.
- **Ikony**: start, změna `actions`, „Obnovit“ → `refresh` → vlákno `action-icons` (stat, COM,
  disk) → `STORE` + `mark_dirty`; chybějící favicon → `fetch_favicon` na runtime Tauri → disk →
  `refresh` znovu (už z cache). UI: `iconKey` → `action_icon` jednou na klíč.
- **Skript**: paleta při otevření → `scripts_list` → `refresh` (výpis složky, změněné soubory) →
  položky „Skript“; volba → `runAny` → `runAction(…, run_script)` (potvrzení přes `ConfirmAction`
  s `confirmNote`) → `run_script(id, confirmed)` → `fresh` (soubor znovu přečíst + SHA-256) → sedí
  `confirm_text` (obsahuje otisk) → poprvé `Consents::grant_fp` → `plan` → `actionrun::capture`
  (hidden/output, `RunGuard`) / `spawn_console` (terminal). Výstup `output` ukáže karta Rychlé akce
  (`LastOutput`, i když skripty v kartě nejsou).
- **Nastavení Windows**: s dotazem `winItems` (77 řádků tabulky) → volba → `win_settings_open(target)`
  → `winsettings::parse` → opener (`ms-settings:`) / `control.exe /name …` → `UiCmd::Hide`.
- **Souhlas**: editor (zaškrtnutí, jen riziková akce bez `confirm`) → uložit → `action_consent`
  (musí sedět s configem) → `Consents::grant`. `views` čte jen `has_cached` (nenačteno = ptát se).

## Invarianty a pasti

- Argumenty podle shellu (`command_line`): `cmd` = `"…"` (koncové `\` zdvojené), PowerShell =
  `'…'` doslova (`quote_ps` — žádné `$`, `#`, `,`). Skrytý příkaz drží v paměti jen posledních 64 KB
  výstupu (`gitinfo::capture_tail`, `MAX_OUTPUT_BYTES`). Plné souhlasy (256) vyhodí jiný, nikdy právě udělený.
- `command` / `app` s `args` bez souhlasu se VŽDY potvrzuje; souhlas = SHA-256 cíle, args, cwd,
  shellu a okna — popisek/ikona ho nemění, cokoli spouštěného ano. Soubor souhlasů není v configu.
- Web jen http/https, cesty přes `personal::normalize` (UNC, ADS, koncové tečky ne), soubor jen
  z `FILE_ALLOW`, typ musí sedět s cílem (`resolve`).
- Síť jen pro favicon, jednou na host (`.miss` = už nezkoušet), 5 s termín, HTML ≤ 512 KB, ikona
  ≤ 256 KB; nikdy periodicky. Snapshot nedělá I/O (`views` = paměť).
- Skrytý příkaz čte stdout do konce → příkaz, který nechá běžet potomka (server), doběhne až
  termínem (zabije se strom) — na to je `terminal: true`.
- **Kompaktní paleta** (`palette.compact`, výchozí vypnuto; Nastavení → Šuplík a proužek): bez dotazu
  jen pole (`pal-bare`, seznam není v DOM → `ResizeObserver` v `App.tsx` šuplík přeměří sám, morf se
  nemění); psaní = výsledky, ↓ s prázdným polem = nedávné (bez nich vše), ↑ z prvního řádku sbalí,
  smazání dotazu sbalí. Rozhoduje `compact.ts`, `Palette.tsx` jen kreslí.
- **Návrat fokusu**: paleta / poznámka zavřená Esc, ✕, „Zavřít šuplík“ nebo uložením (`close_drawer` →
  `UiCmd::Close`) vrátí fokus oknu, odkud si ho Ledge vzal (`focusreturn`, viz [core.md](core.md));
  akce / projekt / odkaz z palety, které otevírají okno, zavírají přes `UiCmd::Hide` → fokus NEvrací.
- Paleta bere fokus jen po kliku na 🔍 (`took` → při zavření `releaseFocus`); `Ctrl+K` funguje jen
  když šuplík fokus už má. Esc v paletě zavře jen paletu.
- Testy nesahají na `%LOCALAPPDATA%`: `Consents` i cache mají cestu parametrem; `log` se z
  `actionrun` nevolá (píše do `%APPDATA%`).

- **Skripty**: žádný watcher ani pollování — sken jen při otevření palety, otevření šuplíku
  (s `inActions`), změně configu a „Načíst znovu“. Snapshot čte jen paměť (`snapshot_views`, jen s
  `inActions`). Souhlas skriptu = otisk cesty + SHA-256 obsahu + režim + argumenty (prefix
  `ledge-script`, s otiskem akce se nepotká); `run_script` vždy čte soubor znovu. Zbývá okno mezi
  hashem a spuštěním (ms) — skript se spouští z cesty, ne z kopie (`$PSScriptRoot`, relativní importy).
- `cmd` řádka: cesta s `%` nebo `"` se odmítne (problém skriptu); PowerShell přes `-Command "& 'cesta'"`
  (`quote_ps`), Python/Node přímo bez shellu, `terminal` u nich přes `cmd /k`. Interpret z PATH, ne
  `…\Microsoft\WindowsApps` (alias otevře Store).
- **Nastavení Windows** jen s dotazem (bez něj by 77 řádků zahltilo seznam). Nový řádek tabulky jen
  s URI z dokumentace — `winsettings.test.mjs` drží opsaný seznam a zastaralé stránky.

## Testy

`actions::tests` (validace, potvrzení, otisk, řádka, serde, kompatibilita configu, schéma),
`actionrun::tests` (`resolve`, `shell_cmd`, `tail`, souhlasy v temp, skutečný `cmd` s termínem,
`RunGuard`), `actionicons::tests` (spec, origin, kandidáti, disková cache, lokální ikony),
`iconimg::tests::shell_icon_for_folder_and_file`. Frontend:
`node --experimental-strip-types --test src/palette/search.test.mjs` (search + model, 12 testů),
`src/palette/compact.test.mjs` (kompaktní režim: `shown`/`visible`/`arrow`, tabulky),
`src/palette/winsettings.test.mjs` (tabulka proti dokumentaci, tvar cílů, hledání CZ/EN). Rust:
`paletteconf::tests` (výchozí vypnuto / `winSettings` zapnuto, serde, schéma), `scripts::meta::tests`
(styly komentářů, pole, chyby, argumenty), `scripts::run::tests` (plán příkazu, PATH bez Storu,
otisk), `scripts::tests` (sken, cache změněných, stropy, pohled, souhlas, skutečné spuštění),
`winsettings::tests`.

## Co nečíst

`personal.rs` celý (stačí `classify`, `normalize`, `FILE_ALLOW`), `icons.rs` (jen `icon_links`,
`b64`), `gitinfo.rs` (jen `capture`/`capture_tail`/`read_tail`), `Palette.tsx` když měníš jen hledání (`search.ts`).
