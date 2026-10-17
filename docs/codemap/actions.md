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
| `src/palette/items.ts` · `Palette.tsx` · `goto.ts` | položky ze snapshotu · UI palety · skok na panel Nastavení | `buildItems`, `PItem`, `Nav` · `Palette`, `IconSearch` · `gotoSettingsPanel`, `settingsTarget` |
| `src/types.ts` (blok `// --- actions ---`) | typy + `Snapshot.actions` (sloučení rozhraní) | `ActionCfg`, `ActionView`, `ActionOutcome` |

Lepidlo: `config.rs` (`Config.actions` + `validate`), `snapshot.rs` (`Snapshot.actions`),
`settings.rs` (`apply_config` → `actionstate::refresh`), `lib.rs` (start + `generate_handler!`),
`MainPage.tsx` (karta), `Head.tsx` (🔍), `App.tsx` (signál palety, `Ctrl+K`, `palNav`).

## Toky

- **Spuštění**: dlaždice/paleta → `runAction` (problém → hláška; `needsConfirm` → `ConfirmAction`) →
  `run_action(id, target, confirmed)` → znovu najde akci (config se mohl změnit), ověří potvrzení
  proti `confirm_text`, `resolve(check_fs)` → opener (web/složka/soubor/.exe) / `run_exe` /
  `run_terminal` / `run_hidden` (`RunGuard`). Otevření zavře šuplík, skrytý příkaz ne.
- **Ikony**: start, změna `actions`, „Obnovit“ → `refresh` → vlákno `action-icons` (stat, COM,
  disk) → `STORE` + `mark_dirty`; chybějící favicon → `fetch_favicon` na runtime Tauri → disk →
  `refresh` znovu (už z cache). UI: `iconKey` → `action_icon` jednou na klíč.
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
- Paleta bere fokus jen po kliku na 🔍 (`took` → při zavření `releaseFocus`); `Ctrl+K` funguje jen
  když šuplík fokus už má. Esc v paletě zavře jen paletu.
- Testy nesahají na `%LOCALAPPDATA%`: `Consents` i cache mají cestu parametrem; `log` se z
  `actionrun` nevolá (píše do `%APPDATA%`).

## Testy

`actions::tests` (validace, potvrzení, otisk, řádka, serde, kompatibilita configu, schéma),
`actionrun::tests` (`resolve`, `shell_cmd`, `tail`, souhlasy v temp, skutečný `cmd` s termínem,
`RunGuard`), `actionicons::tests` (spec, origin, kandidáti, disková cache, lokální ikony),
`iconimg::tests::shell_icon_for_folder_and_file`. Frontend:
`node --experimental-strip-types --test src/palette/search.test.mjs` (search + model, 12 testů).

## Co nečíst

`personal.rs` celý (stačí `classify`, `normalize`, `FILE_ALLOW`), `icons.rs` (jen `icon_links`,
`b64`), `gitinfo.rs` (jen `capture`/`capture_tail`/`read_tail`), `Palette.tsx` když měníš jen hledání (`search.ts`).
