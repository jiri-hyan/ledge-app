# Codemapa: průvodce prvním spuštěním a čistý start

Nový uživatel (jiné PC, jiná repa) dostane prázdný config s MALÝM výchozím rozložením (`layout::clean_default`, proužek Hodiny · Režim · Claude čeká), ikonu v oznamovací oblasti (`tray.rs`, [core.md](core.md)) a šuplík se mu jednou sám otevře s průvodcem. Autorův existující config se chová beze změny. Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/onboarding/mod.rs` | sekce `onboarding`, čistý config, dorovnání starého configu při `config::load` | `OnboardingConfig` (výchozí `done: true`), `fresh_config`, `upgrade_loaded`, `legacy_notes_sync`, `legacy_update_repo` (`jiri-hyan/ledge` → `jiri-hyan/ledge-app`) |
| `src-tauri/src/uiconf.rs` | sekce `ui` (jazyk) | `UiConfig`, `Lang` (`cs`/`en`) |
| `onboarding/repos.rs` | mělký sken rep (hloubka ≤ 2) + návrh projektu | `default_roots`, `scan`, `facts`/`Facts`, `suggest`, `dev_port`, `cargo_name`, `slug`, `reflog_last_ms`, `discover`, `FoundRepo` |
| `onboarding/apps.rs` | Start menu `.lnk` → exe přes `IShellLinkW`; argumenty jako `CommandLineToArgvW` (uvozovky, `\"`), cesta s mezerou bez uvozovek se slepí zpět, když existuje | `start_menu_dirs`, `read_lnk`, `keep`, `dedupe`, `dedupe_key`, `split_args`, `split_args_fs`, `discover`, `FoundApp` |
| `onboarding/store.rs` | aplikace z Microsoft Store (`shell:AppsFolder`, COM STA, termín 6 s, jen na vyžádání z MCP `apps_discover`; průvodce je nenabízí) + spuštění `shell:AppsFolder\<aumid>`; `pick` a `shell_target` čisté | `list_raw`, `pick`, `shell_target`, `launch`, `StoreApp` |
| `onboarding/first_run.rs` | první spuštění: šuplík se novému uživateli (`onboarding.done == false`) jednou sám otevře (`Opener::Remote` = bez fokusu), jen mimo hru / herní režim, značka `first-run-shown` vedle configu; ve hře se zkouší znovu (30 s, ≤ 10×) | `should_open`, `Verdict`, `schedule`, `was_shown`, `mark_shown`, `FILE` |
| `onboarding/claude.rs` | `claude` na PATH / `~\.local\bin`, `~\.claude`, verze | `find_exe`, `parse_version`, `detect`, `ClaudeInfo` |
| `onboarding/dialog.rs` | výběr složky (`IFileOpenDialog`, STA vlákno, vlastník = šuplík) | `pick_folder` |
| `onboarding/themes.rs` | motivy `appearance.theme` ze schématu configu | `theme_options`, `ThemeOption` |
| `onboarding/apply.rs` | zápis výběru do configu (čistá funkce); při PRVNÍM dokončení (`done` z `false` na `true`) doplní Práci o karty podle detekce — `Patch.claude` (Claude Code nalezen) a git projekty (`layout::with_detected`) | `Patch`, `Applied`, `apply`, `same_project`, `unique_id`, `add_links` |
| `src-tauri/src/commands/onboarding.rs` | Tauri příkazy (skeny ve `spawn_blocking`) | `onboarding_repos`, `onboarding_apps`, `onboarding_icons`, `onboarding_claude`, `onboarding_pick_folder`, `onboarding_themes`, `onboarding_apply`, `KNOWN` |
| `src/onboarding/state.ts` | kdy se průvodce ukáže, jazyk z configu; **`startWizard()` = „spustit průvodce znovu“** (po „Přeskočit průvodce“; volá ho Nastavení → Aplikace → Jazyk a průvodce) | `checkFirstRun`, `startWizard`, `closeWizard`, `wizardOpen` |
| `src/onboarding/store.ts` | stav průvodce, načítání (Claude hned, jako první), ukládání po krocích; `finish` posílá `claude` z detekce | `wiz`, `steps`, `claudeInstalled`, `prefetch`, `loadRepos`, `addFolder`, `loadIcons`, `applyStep`, `pending`, `finish` |
| `src/onboarding/plan.ts` (+ `plan.test.mjs`) | čistý plán: kroky (Claude poslední před Hotovo, jen s Claude Code; Vzhled jen s motivy), poctivý odhad času (30 s na pracovní krok), řádky souhrnu jen nenulové a motiv jménem | `STEPS`, `visibleSteps`, `estimateMinutes`, `doneRows` |
| `src/onboarding/logic.ts` · `src/onboarding/api.ts` · `en.ts` | čisté pomůcky (re-export kroků z `plan.ts`, výběr, URL) · IPC · anglický slovník | `STEPS`, `defaultRepoSelection`, `appProject`, `normalizeUrl`, `linkSuggestions` · `ob` |
| `src/onboarding/claudeSetup.ts` (+ `claudeSetup.test.mjs`) | krok „Claude“: čistá logika bez i18n/DOM — zadání pro Clauda a volba zobrazení | `PRO_CLAUDE_URL`, `buildSetupPrompt(t)`, `setupView` (`manualOnly`/`loading`/`choose`/`connect`), `hasConnectionKit` |
| `src/onboarding/assist.ts` · `steps/Claude.tsx` · `src/mcp/api.ts` | stav kroku (`mcp_state` → `mcp_set_enabled`, kopírování přes `focusDrawer` + schránka) · vykreslení · IPC kontraktu MCP (vlastní ho worker MCP) | `assist`, `loadMcp`, `chooseClaude`, `disableMcp`, `copy`, `promptText` · `ClaudeStep` (hooky + MCP jedním klikem, příkazy/JSON/cesty/porty pod „Podrobnosti“) |
| `src/onboarding/Wizard.tsx` + `steps/*` (`steps/{Apps,Basics,Extras,parts}.tsx`) | stránka `welcome`: hlavička s postupem, kroky, patička | `Wizard`, `WelcomeStep`, `ProjectsStep`, `AppsStep`, `LinksStep`, `LookStep`, `ClaudeStep`, `DoneStep`, `parts` (`Logo`, `CheckRow`, `Skeleton`, `Preview`) |
| `src/settings/panels/Start.tsx` | panel Nastavení → Aplikace → Jazyk a průvodce (ruční spuštění průvodce, tipy, novinky) | `StartPanel` |

Lepidlo: `settings::load_or_seed` (soubor chybí → `fresh_config`), `config::load` (→ `upgrade_loaded`),
`App.tsx` (2 efekty: `checkFirstRun` při `view === "full"`, `wizardOpen` → stránka `welcome`; `Head`
se na ní neukazuje), `nav.ts` (`Page` + `typing` obsahují `welcome`).

## Tok

1. Šuplík `full` → `checkFirstRun` (1× za život WebView): `ui.lang` → `setLang`; `onboarding.done === false`
   → `wizardOpen`. Zavření šuplíku průvodce nezruší (stav v modulu `store.ts`), další otevření pokračuje.
2. `Wizard` při mountu `prefetch`: config (`actions`? → kam odkazy, `drawer.trigger` → tip), motivy,
   repa → Claude → aplikace na pozadí. Ikony jen pro viditelné položky (`onboarding_icons`, ≤ 48).
3. „Uložit a pokračovat“ = `applyStep` → `onboarding_apply` → `apply::apply` → `config::save` +
   `apply_config`. Konec / Přeskočit průvodce = `{ done: true }`.
4. Krok `claude` (poslední před Hotovo, jen když `claudeInstalled()`): hooky („Zapojit“) a MCP („Připojit“ →
   `mcp_set_enabled(true)` → „Zkopírovat zadání pro Clauda“, `buildSetupPrompt`, odkaz na `pro-claude.md` ze vydání).
   Při mountu `loadMcp` → `mcp_state`; selže-li (starší jádro) → `setupView` = `manualOnly`, MCP volba se neukáže.
   Příkaz pro terminál, JSON pro Claude Desktop, cesty a porty jsou v rozbalovacím „Podrobnosti“. Krok nic
   neukládá do configu (`pending` = false); MCP stav drží jádro. Návod pro Clauda `docs/pro-claude.md`
   jde jako příloha vydání (`scripts/publish.ps1`: `pro-claude.md`, `SHA256SUMS.txt`).
5. Konec (Hotovo i „Přeskočit průvodce“) = `{ done: true, claude }` → u čerstvého configu `apply` přidá na Práci
   Claude (nalezen) a Repa (mezi projekty je složka). Opakované spuštění (`startWizard`) rozložení nemění.
   Krok Hotovo ukáže jen nenulové řádky (motiv jménem z `THEMES`), jinak jednu větu.

## Invarianty a pasti

- **Kompatibilita:** chybějící `onboarding` = `done: true`; čistý config se ukládá celý (vč. `done:false`).
  Starý config bez `notes.sync.remote` → `origin` pracovní kopie poznámek (`resolve::read_origin`), bez
  ní sync vypnutý; výslovný `remote` se nemění nikdy. Nic se při načtení nezapisuje.
- **Čistý start jen pro nový config:** malé výchozí rozložení (`layout::clean_default`) a proužek (`StripConfig::default`: Hodiny,
  Režim, Claude čeká) platí jen tam, kde se bere výchozí hodnota. Soubor bez `layout` se převádí z původního plného rozložení
  (`from_legacy`), soubor s `layout` se nemění; test `old_default_is_kept_for_files_without_layout`. Počasí není ve výchozím
  rozložení (bez místa nemá co ukázat), Kalendář / Pošta / GitHub taky ne — jsou v katalogu.
- **První spuštění nekrade fokus:** auto-otevření = `Opener::Remote`; jen `onboarding.done == false`, jen jednou (značka
  `first-run-shown`), ne ve hře. Klik na ikonu v oznamovací oblasti fokus brát smí (výslovný klik).
- **Bez autorova světa:** v kódu žádná výchozí repa; `resolve::OWNER_ALIASES` je prázdné (přejmenovaný
  vlastník se najde přes jednoznačné jméno repa); testy používají `acme/*`.
- `apply`: duplicita = stejná složka (`repos::norm`), remote (`parse_remote`) nebo **absolutní** exe;
  relativní `electron.exe` duplicitu nedělá. Id z názvu (`slug`, bez diakritiky), kolize `-2…`.
- Odkazy jen http(s); do `actions` (`{id,label,kind:"url",target}`, strop `MAX_ACTIONS`, id ≤ 56 znaků
  slugu, popisek ≤ `MAX_LABEL`) — vždy, chybějící sekci založí (do `personal.links` už nic; starý tvar převádí `linkmigrate.rs`). Motiv jen když `appearance` existuje
  (`themeSaved: false` jinak) — krok Vzhled se bez motivů ve schématu neukáže.
- `onboarding_icons` čte disk jen pro klíče, které vrátil sken (`KNOWN`) — UI nemůže podstrčit cestu.
- Fokus: šuplík otevřený najetím nemá klávesnici — vstupy volají `api.focusDrawer()` na `pointerdown`
  (výslovný klik); „Přidat složku…“ taky, aby dialog (vlastník = šuplík) šuplík nezavřel.
- Krok „Claude“ nikdy nespouští Clauda ani nic jiného; zapnutí MCP je výslovný klik a jde vypnout
  tamtéž i v Nastavení → Claude (MCP). `claudeSetup.ts` bez importu `i18n` (testovatelné); `buildSetupPrompt(t)` dostává překladač.
- `claude --version` jen v appce (bez okna, 8 s), nikdy v testech. Aplikace ze Storu (bez `.lnk`) chybí.

## Testy

`onboarding::tests` (čistý config, round-trip existujícího configu `fixtures/config-existing-user.json`,
`legacy_notes_sync` a `legacy_update_repo` tabulky, `origin` z kopie poznámek, převod repa vydání při `load`
bez zápisu), `onboarding::repos::tests` (hloubka, přeskakování,
worktree, `suggest` tabulka, Rust/.NET, `facts` z disku, slug, reflog), `onboarding::apps::tests` (skutečný
`.lnk` vyrobený přes `IShellLinkW` v tempu, `keep`, `split_args`, `dedupe`), `onboarding::claude::tests`,
`onboarding::themes::tests`, `onboarding::apply::tests` (vč. karty podle detekce), `onboarding::first_run::tests` (rozhodnutí, značka v tempdiru), `tray::tests`, `uiconf::tests`; UI: `src/onboarding/claudeSetup.test.mjs`
(zadání, volba zobrazení vč. selhání `mcp_state`), `plan.test.mjs` (kroky, odhad, souhrn).

## Co nečíst

`update*.rs` (aktualizace — [core.md](core.md)), `icons.rs`/`iconimg.rs` (jen `resolve`/`exe_png`),
`hooksetup.rs` (průvodce volá jen `hooks_status`/`hooks_install`).
