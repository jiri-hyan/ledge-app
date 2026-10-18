# Codemapa: průvodce prvním spuštěním a čistý start

Nový uživatel (jiné PC, jiná repa) dostane prázdný config a při prvním otevření šuplíku průvodce.
Autorův existující config se chová beze změny. Cituj symbolem, ne číslem řádku.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/onboarding/mod.rs` | sekce `onboarding`, čistý config, dorovnání starého configu při `config::load` | `OnboardingConfig` (výchozí `done: true`), `fresh_config`, `upgrade_loaded`, `legacy_notes_sync`, `legacy_update_repo` (`jiri-hyan/ledge` → `jiri-hyan/ledge-app`) |
| `src-tauri/src/uiconf.rs` | sekce `ui` (jazyk) | `UiConfig`, `Lang` (`cs`/`en`) |
| `onboarding/repos.rs` | mělký sken rep (hloubka ≤ 2) + návrh projektu | `default_roots`, `scan`, `facts`/`Facts`, `suggest`, `dev_port`, `cargo_name`, `slug`, `reflog_last_ms`, `discover`, `FoundRepo` |
| `onboarding/apps.rs` | Start menu `.lnk` → exe přes `IShellLinkW`; argumenty jako `CommandLineToArgvW` (uvozovky, `\"`), cesta s mezerou bez uvozovek se slepí zpět, když existuje | `start_menu_dirs`, `read_lnk`, `keep`, `dedupe`, `dedupe_key`, `split_args`, `split_args_fs`, `discover`, `FoundApp` |
| `onboarding/store.rs` | aplikace z Microsoft Store (`shell:AppsFolder`, COM STA, termín 6 s, jen na vyžádání z MCP `apps_discover`; průvodce je nenabízí) + spuštění `shell:AppsFolder\<aumid>`; `pick` a `shell_target` čisté | `list_raw`, `pick`, `shell_target`, `launch`, `StoreApp` |
| `onboarding/claude.rs` | `claude` na PATH / `~\.local\bin`, `~\.claude`, verze | `find_exe`, `parse_version`, `detect`, `ClaudeInfo` |
| `onboarding/dialog.rs` | výběr složky (`IFileOpenDialog`, STA vlákno, vlastník = šuplík) | `pick_folder` |
| `onboarding/themes.rs` | motivy `appearance.theme` ze schématu configu | `theme_options`, `ThemeOption` |
| `onboarding/apply.rs` | zápis výběru do configu (čistá funkce) | `Patch`, `Applied`, `apply`, `same_project`, `unique_id`, `add_links` |
| `src-tauri/src/commands/onboarding.rs` | Tauri příkazy (skeny ve `spawn_blocking`) | `onboarding_repos`, `onboarding_apps`, `onboarding_icons`, `onboarding_claude`, `onboarding_pick_folder`, `onboarding_themes`, `onboarding_apply`, `KNOWN` |
| `src/onboarding/state.ts` | kdy se průvodce ukáže, jazyk z configu | `checkFirstRun`, `startWizard`, `closeWizard`, `wizardOpen` |
| `src/onboarding/store.ts` | stav průvodce, načítání, ukládání po krocích | `wiz`, `prefetch`, `loadRepos`, `addFolder`, `loadIcons`, `applyStep`, `pending`, `finish` |
| `src/onboarding/logic.ts` · `api.ts` · `en.ts` | čisté pomůcky (kroky, výběr, URL) · IPC · anglický slovník | `STEPS`, `defaultRepoSelection`, `appProject`, `normalizeUrl`, `LINK_SUGGESTIONS` · `ob` |
| `src/onboarding/claudeSetup.ts` (+ `.test.mjs`) | krok „S Claudem“: čistá logika bez i18n/DOM — zadání pro Clauda a volba zobrazení | `PRO_CLAUDE_URL`, `buildSetupPrompt(t)`, `setupView` (`manualOnly`/`loading`/`choose`/`connect`), `hasConnectionKit` |
| `src/onboarding/assist.ts` · `steps/Assist.tsx` · `src/mcp/api.ts` | stav kroku (`mcp_state` → `mcp_set_enabled`, kopírování přes `focusDrawer` + schránka) · vykreslení · IPC kontraktu MCP (vlastní ho worker MCP) | `assist`, `loadMcp`, `chooseClaude`, `disableMcp`, `copy`, `promptText` · `AssistStep` |
| `src/onboarding/Wizard.tsx` + `steps/*` | stránka `welcome`: hlavička s postupem, kroky, patička | `Wizard`, `WelcomeStep`, `AssistStep`, `ProjectsStep`, `AppsStep`, `ClaudeStep`, `LinksStep`, `LookStep`, `DoneStep`, `parts` (`Logo`, `CheckRow`, `Skeleton`, `Preview`) |
| `src/settings/panels/Start.tsx` | panel Nastavení → Začínáme (ruční spuštění průvodce, tipy, novinky) | `StartPanel` |

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
4. Krok `assist` („S Claudem“, hned za Vítej): při mountu `loadMcp` → `mcp_state`. Selže-li (starší jádro,
   mimo Tauri) → `setupView` = `manualOnly`, jen věta a Další, žádný pád. „Nastavit s Claudem“ = jeden klik
   → `mcp_set_enabled(true)` → `connect`: `cliCommand` (Claude Code), `desktopConfig` (Claude Desktop),
   „Zkopírovat zadání pro Clauda“ (`buildSetupPrompt`, odkaz na `pro-claude.md` ze vydání). Krok nic
   neukládá do configu (`pending` = false); MCP stav drží jádro. Návod pro Clauda `docs/pro-claude.md`
   jde jako příloha vydání (`scripts/publish.ps1`: `pro-claude.md`, `SHA256SUMS.txt`).

## Invarianty a pasti

- **Kompatibilita:** chybějící `onboarding` = `done: true`; čistý config se ukládá celý (vč. `done:false`).
  Starý config bez `notes.sync.remote` → `origin` pracovní kopie poznámek (`resolve::read_origin`), bez
  ní sync vypnutý; výslovný `remote` se nemění nikdy. Nic se při načtení nezapisuje.
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
- Krok „S Claudem“ nikdy nespouští Clauda ani nic jiného; zapnutí MCP je výslovný klik na volbu a jde vypnout
  tamtéž i v Nastavení → Claude (MCP). `claudeSetup.ts` bez importu `i18n` (testovatelné); `buildSetupPrompt(t)` dostává překladač.
- `claude --version` jen v appce (bez okna, 8 s), nikdy v testech. Aplikace ze Storu (bez `.lnk`) chybí.

## Testy

`onboarding::tests` (čistý config, round-trip existujícího configu `fixtures/config-existing-user.json`,
`legacy_notes_sync` a `legacy_update_repo` tabulky, `origin` z kopie poznámek, převod repa vydání při `load`
bez zápisu), `onboarding::repos::tests` (hloubka, přeskakování,
worktree, `suggest` tabulka, Rust/.NET, `facts` z disku, slug, reflog), `onboarding::apps::tests` (skutečný
`.lnk` vyrobený přes `IShellLinkW` v tempu, `keep`, `split_args`, `dedupe`), `onboarding::claude::tests`,
`onboarding::themes::tests`, `onboarding::apply::tests`, `uiconf::tests`; UI: `src/onboarding/claudeSetup.test.mjs`
(zadání, volba zobrazení vč. selhání `mcp_state`).

## Co nečíst

`update*.rs` (aktualizace — [core.md](core.md)), `icons.rs`/`iconimg.rs` (jen `resolve`/`exe_png`),
`hooksetup.rs` (průvodce volá jen `hooks_status`/`hooks_install`).
