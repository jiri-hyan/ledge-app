# Codemapa: průvodce prvním spuštěním a čistý start

Nový uživatel (jiné PC, jiná repa) dostane prázdný config s MALÝM výchozím rozložením (`layout::clean_default`, proužek Hodiny · Režim · Claude čeká), ikonu v oznamovací oblasti (`tray.rs`, [core.md](core.md)) a šuplík se mu jednou sám otevře s průvodcem. Autorův existující config se chová beze změny …

Detail (toky, invarianty, testy, co nečíst, historie): [onboarding-detail.md](onboarding-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/onboarding/mod.rs` | sekce `onboarding`, čistý config, dorovnání starého configu při `config::load` | `OnboardingConfig` `done: true` `fresh_config` `upgrade_loaded` `legacy_notes_sync` |
| `src-tauri/src/uiconf.rs` | sekce `ui` (jazyk) | `UiConfig`, `Lang` (`cs`/`en`) |
| `onboarding/repos.rs` | mělký sken rep (hloubka ≤ 2) + návrh projektu | `default_roots` `scan` `facts` `Facts` `suggest` `dev_port` `cargo_name` `slug` |
| `onboarding/apps.rs` | Start menu `.lnk` → exe přes `IShellLinkW`; argumenty jako `CommandLineToArgvW` (uvozovky, `\"`), cesta s mezerou bez uvozovek se slepí zpět, když existuje | `start_menu_dirs` `read_lnk` `keep` `dedupe` `dedupe_key` `split_args` `split_args_fs` |
| `onboarding/store.rs` | aplikace z Microsoft Store (`shell:AppsFolder`, COM STA, termín 6 s, jen na vyžádání z MCP `apps_discover`; průvodce je nenabízí) + spuštění `shell:AppsFolder\<aumid>`; `pick` a `shell_target` čisté | `list_raw`, `pick`, `shell_target`, `launch`, `StoreApp` |
| `onboarding/first_run.rs` | první spuštění: šuplík se novému uživateli (`onboarding.done == false`) jednou sám otevře (`Opener::Remote` = bez fokusu), jen mimo hru / herní režim, značka `first-run-shown` vedle configu … | `should_open`, `Verdict`, `schedule`, `was_shown`, `mark_shown`, `FILE` |
| `onboarding/claude.rs` | `claude` na PATH / `~\.local\bin`, `~\.claude`, verze | `find_exe`, `parse_version`, `detect`, `ClaudeInfo` |
| `onboarding/dialog.rs` | výběr složky (`IFileOpenDialog`, STA vlákno, vlastník = šuplík) | `pick_folder` |
| `onboarding/themes.rs` | motivy `appearance.theme` ze schématu configu | `theme_options`, `ThemeOption` |
| `onboarding/apply.rs` | zápis výběru do configu (čistá funkce); při PRVNÍM dokončení (`done` z `false` na `true`) doplní Práci o karty podle detekce … | `Patch`, `Applied`, `apply`, `same_project`, `unique_id`, `add_links` |
| `src-tauri/src/commands/onboarding.rs` | Tauri příkazy (skeny ve `spawn_blocking`) | `onboarding_repos` `onboarding_apps` `onboarding_icons` `onboarding_claude` |
| `src/onboarding/state.ts` | kdy se průvodce ukáže, jazyk z configu; `startWizard()` = „spustit průvodce znovu“ (po „Přeskočit průvodce“; volá ho Nastavení → Aplikace → Jazyk a průvodce) | `checkFirstRun`, `startWizard`, `closeWizard`, `wizardOpen` |
| `src/onboarding/store.ts` | stav průvodce, načítání (Claude hned, jako první), ukládání po krocích; `finish` posílá `claude` z detekce | `wiz` `steps` `claudeInstalled` `prefetch` `loadRepos` `addFolder` `loadIcons` `applyStep` |
| `src/onboarding/plan.ts` (+ `plan.test.mjs`) | čistý plán: kroky (Claude poslední před Hotovo, jen s Claude Code; Vzhled jen s motivy), poctivý odhad času (30 s na pracovní krok), řádky souhrnu jen nenulové a motiv jménem | `STEPS`, `visibleSteps`, `estimateMinutes`, `doneRows` |
| `src/onboarding/logic.ts` · `src/onboarding/api.ts` · `en.ts` | čisté pomůcky (re-export kroků z `plan.ts`, výběr, URL) · IPC · anglický slovník | `STEPS`, `defaultRepoSelection`, `appProject`, `normalizeUrl`, `linkSuggestions` · `ob` |
| `src/onboarding/claudeSetup.ts` (+ `claudeSetup.test.mjs`) | krok „Claude“: čistá logika bez i18n/DOM — zadání pro Clauda a volba zobrazení | `PRO_CLAUDE_URL` `buildSetupPrompt(t)` `setupView` `manualOnly` `loading` `choose` |
| `src/onboarding/assist.ts` · `steps/Claude.tsx` · `src/mcp/api.ts` | stav kroku (`mcp_state` → `mcp_set_enabled`, kopírování přes `focusDrawer` + schránka) · vykreslení · IPC kontraktu MCP (vlastní ho worker MCP) | `assist` `loadMcp` `chooseClaude` `disableMcp` `copy` `promptText` `ClaudeStep` |
| `src/onboarding/Wizard.tsx` + `steps/*` (`steps/{Apps,Basics,Extras,parts}.tsx`) | stránka `welcome`: hlavička s postupem, kroky, patička | `Wizard` `WelcomeStep` `ProjectsStep` `AppsStep` `LinksStep` `LookStep` `ClaudeStep` |
| `src/settings/panels/Start.tsx` | panel Nastavení → Aplikace → Jazyk a průvodce (ruční spuštění průvodce, tipy, novinky) | `StartPanel` |

Další soubory (podrobnosti v detailu):
`src/projects/state.ts` · `src/sessions/plan.ts` · `src/tilelook/state.ts`
