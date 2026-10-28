# Codemapa: rychlé akce a paleta příkazů

Config `actions` → karta „Rychlé akce“, karta „Odkazy“ (akce, které jen otevírají), panel Nastavení „akce“ (Projekty, akce a odkazy) a paleta (`Ctrl+K` / 🔍). Formát pro agenty: [agents/actions.md](../agents/actions.md) → „Rychlé akce“. Cituj symbolem, ne číslem řádku.

Detail (toky, invarianty, testy, co nečíst, historie): [actions-detail.md](actions-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/actions.rs` | tvar configu, validace (tvar, bez disku), čistá rozhodnutí | `Action` `ActionKind` `ShowOn` `Shell` `validate` `validate_one` `fingerprint` |
| `src-tauri/src/actionrun.rs` | kam akce vede, spuštění, souhlasy, běžící | `resolve` `Launch` `check_fs` `work_dir` `shell_cmd` `run_hidden` `run_terminal` `run_exe` |
| `src-tauri/src/webpanel.rs` | okno s webem (`kind: "web"`): politika adres (hlavní rámec, iframe), rozměry, umístění, poloha per akce (`web-panels.json`), stavba `WebviewWindow` se sdíleným WebView2 | `open` `check_url` `nav_allowed` `frame_allowed` `check_size` `size` `place` `visible` |
| `src-tauri/src/actionicons.rs` | odkud ikona, favicon (síť), disková cache | `spec` `Spec` `origin` `join_href` `candidates` `cached` `store` `forget` `local` |
| `src-tauri/src/actionstate.rs` | stav pro UI: ikony + varování, vlákno `action-icons`, snapshot | `STORE`, `refresh`, `compute`, `icon_url`, `views`, `ActionView` |
| `src-tauri/src/commands/settings.rs` | „Vybrat…“: společný `pick_path` (dialog plugin, i pro Nastavení) | `pick_path` |
| `src-tauri/src/commands/actions.rs` | Tauri příkazy | `run_action` `action_icon` `action_icon_refresh` `action_consent` `action_check` |
| `src-tauri/src/iconimg.rs` | + ikona souboru/složky ze shellu (48 px) | `shell_png` |
| `src/actions/api.ts` | invoke, cache ikon podle `iconKey`, spuštění s potvrzením a výsledkem | `actionsApi`, `actionIconSrc`, `runAction`, `isBusy`, `lastResult` |
| `src/actions/model.ts` | čistá logika UI | `slugId` `moveItem` `guessKind` `LINK_KINDS` `isLink` `onPage` `linkActions` |
| `src/actions/ui.tsx` | ikona/monogram, potvrzení v šuplíku, výstup příkazu | `ActionIcon` `askConfirm` `ConfirmAction` `answerConfirm` `confirmPending` `LastOutput` |
| `src/ui/iconTone.ts` | tón ikony (čistá `toneOf` z pixelů, `measureTone` 1× na URL přes 16×16 canvas, `useTone`): tmavá/světlá kresba na průhledném → třída `tone-dark`/`tone-light` … | `toneOf`, `measureTone`, `useTone`, `Tone` |
| `src/actions/ActionEditor.tsx` · `src/settings/panels/Actions.tsx` | editor akce · sekce „Akce a odkazy“ (jeden seznam, `SortList` tah + Alt+šipky, Vrátit) | `ActionEditor`, `kindLabel` · `ActionsPanel` |
| `src/cards/ActionsCard.tsx` | karta (Práce bez akcí se neukáže, Osobní ukáže pozvánku; `skipLinks` = na záložce s kartou Odkazy bez odkazů) | `ActionsCard` |
| `src/cards/LinksCard.tsx` | karta Odkazy: `linkActions` záložky (favicon `ActionIcon`, klik `runAction`) + nepřevedené `personal.links` postaru (`openLink`) | `LinksCard` |
| `src-tauri/src/linkmigrate.rs` | `personal.links` → akce při načtení configu a při `save_config` (idempotentní, co neprojde validací, zůstane odkazem) | `migrate`, `guess_kind` |
| `src/palette/search.ts` | čisté hledání: bez diakritiky, fuzzy, zvýraznění, nedávné + frecency (četnost × čerstvost, poločas 7 dní, `Usage` v `localStorage` `ledge.palette.usage`, strop bonusu 30 … | `fold` `score` `segments` `rank` `pushRecent` `frecency` `bumpUsage` `cleanUsage` |
| `src/palette/notehits.ts` | poznámky nalezené podle OBSAHU (`note_search`, po 180 ms ticha od 2 znaků): úryvek jako druhý řádek poznámky v paletě, starší poznámky se přidají … | `mergeNoteHits`, `NoteHit` |
| `src-tauri/src/scripts/mod.rs` | Script Commands: config `scripts`, složka (`resolve_dir_with`), sken bez watcheru (stat + jen změněné, strop 200, 1 MB), pohled pro UI, stav v paměti | `ScriptsConfig` `scan_dir` `load_one` `Scan` `Script` `fingerprint` `confirm_text` |
| `src-tauri/src/scripts/meta.rs` | čistá hlavička `@ledge.*` (komentáře podle jazyka, prvních 4 KB) | `parse`, `header_text`, `split_args`, `Meta`, `Mode`, `Lang`, `HEADER_BYTES` |
| `src-tauri/src/scripts/run.rs` | interpret z PATH (bez aliasů Storu), program + argumenty, otisk souhlasu, text potvrzení | `find_on_path` `interp` `Interp` `plan` `Plan` `fingerprint` `confirm_text` `sha256_hex` |
| `src-tauri/src/scripts/tests.rs` | sken v tempdir, cache, stropy, pohled, skutečný cmd / PowerShell | — |
| `src-tauri/src/winsettings.rs` | tvar cíle „Nastavení Windows“ (`ms-settings:` / `control:Microsoft.`), cesta k `control.exe` | `parse`, `WinTarget`, `control_exe` |
| `src-tauri/src/commands/scripts.rs` | Tauri příkazy skriptů a nastavení Windows | `scripts_list`, `run_script`, `scripts_open_dir`, `win_settings_open` |
| `src/actions/scripts.ts` · `ScriptsSettings.tsx` | invoke skriptů + `runAny` (akce i skript stejným tokem) · sekce Skripty v panelu Projekty, akce a odkazy (přepínače, složka, nalezené, formát) | `scriptsApi`, `runAny` · `ScriptsSettings` |
| `src/palette/winsettings.ts` | statická tabulka stránek Nastavení Windows (jen dokumentované, CZ+EN slova přes `tIn`), položky jen s dotazem | `WIN_PAGES`, `winItems`, `WinPage` |
| `src/palette/more.ts` | položky palety mimo snapshot: skripty (`scripts_list` při otevření) + Nastavení Windows + rychlé odpovědi z dotazu (řádky s `copy`) + příkaz „Emoji…“ + nápověda | `scriptItems`, `windowsItems`, `quickRows`, `emojiCommand`, `paletteTips` |
| `src/palette/quick.ts` | co z dotazu vznikne bez snapshotu: `:` → emoji (místo seznamu), převod, výpočet (jeden řádek nahoře), jinak nic | `quick`, `Quick` |
| `src/palette/calc.ts` | kalkulačka: lexer + rekurzivní sestup (žádné `eval`/`Function`), procenta (`200 + 10%`, `15 % z 340`), funkce, konstanty; číslo podle jazyka bez float šumu | `evaluate`, `lex`, `formatNum` |
| `src/palette/units.ts` | převody „číslo jednotka na/v/do/to/in jednotka“: délka, hmotnost, teplota (posun), objem, plocha, data (1000 i 1024, bity), čas, rychlost; měny ne | `UNITS`, `convert`, `conversionText`, `unitIndex`, `unitKey`, `unitSym` |
| `src/palette/emoji.ts` · `emojidata.ts` | emoji: prefix `:`, hledání cs+en bez diakritiky (`search.ts`), shoda celého jména vyhrává remízu, líné načtení tabulky (dynamický `import()` = samostatný chunk ~51 kB při prvním `:` dotazu … | `emojiQuery`, `searchEmoji`, `emojiItems`, `loadEmoji`, `emojiLoaded` · `EMOJI` |
| `src/cliphist/model.ts` | historie schránky v paletě (`v:`) → [clipboard.md](clipboard.md) | `clipQuery`, `clipRows` |
| `src/palette/compact.ts` | kompaktní paleta (`palette.compact`): co ukázat pod polem podle režimu a dotazu, šipky (rozbalit ↓ / sbalit ↑) | `shown`, `visible`, `arrow`, `Shown`, `Fold` |
| `src-tauri/src/paletteconf.rs` | sekce configu `palette` → `Snapshot.palette_compact` (`paletteCompact`) | `PaletteConfig` |
| `src/palette/timer.ts` | příkazy časovače: „Časovač N min“ z předvoleb, pauza / pokračovat / +5 min / zrušit podle stavu, stopky; přes `timer/TimerCard.timerDo` | `timerItems` |
| `src/palette/items.ts` · `Palette.tsx` · `goto.ts` | položky ze snapshotu · UI palety · skok na panel Nastavení | `buildItems` `PItem` `Nav` `Palette` `lazyViews.tsx` `IconSearch` `icons.tsx` |
| `src/types/actions.ts` | typy + `Snapshot.actions`/`scripts`/`paletteWinSettings` | `ActionCfg`, `ActionView`, `ScriptView`, `ScriptsListing`, `ActionOutcome` |

Další soubory (podrobnosti v detailu):
`src-tauri/src/{gitinfo.rs,personal.rs}`
