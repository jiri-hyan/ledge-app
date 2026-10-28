# Codemapa: karta Rychlé dlaždice Windows (kolo 23, id `winquick`)

Volitelná karta (jen v katalogu karet, `layout::OPT_IN`; výchozí rozložení beze změny). Člověk si v ní dlaždice přidává, odebírá a řadí
(✎ Upravit dlaždice). Config `winquick` ([config-behavior.md](../agents/config-behavior.md) → Rychlé dlaždice Windows). Každá akce jen na
výslovný klik; stav (motiv, rádia, poslední snímek) se čte jen při viditelné kartě a otevřeném šuplíku (`drawerShown`), žádný časovač ani watcher.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/winquick.rs` | config `winquick` (`tiles`, `outputSwitch` výchozí VYPNUTO, `folders`), čistá logika: výběr dlaždic, příkazy terminálu, nejnovější obrázek, `@cesta` pro Clauda, motiv přes `ThemeStore` (atrapa v testech) | `WinQuickConfig`, `normalize_tiles`, `validate`, `launch_for`, `newest_image`, `claude_ref`, `toggle_theme`, `ThemeStore` |
| `src-tauri/src/winquickwin.rs` | Windows API: Known Folders, motiv v registru + `WM_SETTINGCHANGE`, rádia (`Windows.Devices.Radios`), poslední snímek (cache podle času složky), obrázek do schránky (`CF_DIB` + `PNG`) | `known_dir`, `RegTheme`, `radios`, `set_radio`, `latest_shot`, `copy_image`, `dib_pixels` |
| `src-tauri/src/commands/winquick.rs` | příkazy `wq_*`; všechny odmítnou, když karta není na záložce (`gate`); UI neposílá cesty ani URI, jen klíče | `wq_state`, `wq_config_set`, `wq_terminal`, `wq_folder`, `wq_theme_toggle`, `wq_radio_set`, `wq_settings`, `wq_output_set`, `wq_shot` |
| `src-tauri/src/winsettings.rs` | tabulka odkazů dlaždice Nastavení (síť, zvuk, displej, Bluetooth) | `QUICK`, `quick_uri` |
| `src/winquick/*` | karta: `model.ts` (čisté: přidat / odebrat / pořadí, rádio, připnutí složky; test `model.test.mjs`), `WinQuickCard.tsx`, `winquick.css` (`wq-`), `en.ts` | `cleanTiles`, `moveTile`, `radioOf`, `pinFolder`, `WinQuickCard` |

## Rozhodnutí

- **Mikrofon** a **výstup** používají `volume.rs` (`volume::allowed` zahrnuje i tuto kartu). Přepnutí výstupu jedním klikem jde jen s
  `winquick.outputSwitch` (jen dlaždice výstupu v Rychlých dlaždicích, `wq_output_set`; menu výstupu v kartě Zvuk `media/VolumeBar.tsx` volbě nepodléhá a jde přes `volume_set_device`; obojí používá neoficiální `IPolicyConfig` v `volume::set_default_output`, selhání = zpráva); bez volby dlaždice
  otevře `ms-settings:sound`.
- **Terminál**: `wt.exe -d <složka>` (DETACHED), PowerShell (nová konzole, `current_dir`), Průzkumník; složka je z `project_dir` projektu. Terminál je viditelný záměrně.
- **Motiv**: `AppsUseLightTheme` + `SystemUsesLightTheme` (HKCU) + `WM_SETTINGCHANGE "ImmersiveColorSet"`, jen na klik.
- **Wi-Fi / Bluetooth**: bez rádia dlaždice ukáže „není k dispozici“ (zakázaná). Přístup si Windows ověřují (`RequestAccessAsync`).
- **Snímek**: Known Folder Screenshots, výsledek se drží, dokud se nezmění čas složky. Kopírovat = obrázek do schránky, Kopírovat cestu, Poslat Claudovi = zkopíruje `@cestu`
  (cesta s mezerou v uvozovkách) — nic se samo nevkládá.
- Neověřeno bez GUI: rádia, registr a schránka se v testech nikdy nemění; ověřuje je člověk (viz report kola).
