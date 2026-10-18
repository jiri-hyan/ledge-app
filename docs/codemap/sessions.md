# Codemapa: Claude session, subagenti, hooky

Karta Claude v šuplíku = všechny Claude Code session na PC + schvalování + otázky Clauda + subagenti + klik „otevřít“;
Nastavení umí zapojit hooky Ledge do `~/.claude/settings.json`. Návod pro uživatele/agenty: `AGENTS.md`
→ „Session a hooky“.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/sessions.rs` | čistá logika: parsování zdrojů, sloučení, stav, co udělá klik, strop spawnů, náhled textu | `parse_agents` (`Agent.job_id` z `id`), `parse_registry`, `is_registry_file`, `live_status`, `agent_status`, `waiting_ids`, `Status`, `merge`, `Live`, `Row` (`background`, `job`, `waiting_for`), `decide_open`, `OpenAction`, `open_hint`, `spawn_wait`, `resume_command`, `Launch`, `clean_preview`, `SYSTEM_TAGS` |
| `src-tauri/src/agentview.rs` | Agent View (čistá): stav úlohy, `waitingFor`, které akce, validace id, argumenty `attach/logs/stop/rm/respawn` a `--bg`, úklid výstupu, nalezení `claude` | `JobState`, `job_state`, `WaitFor`, `wait_for`, `JobAction` (`needs_confirm`, `wait`), `Job`, `job`, `actions`, `valid_job_id`, `cli_args`, `attach_command`, `clean_prompt`, `clean_name`, `bg_args`, `for_batch`, `backgrounded_id`, `clean_output`, `Program`, `resolve_claude`, `LOG_MAX`, `PROMPT_MAX`, `NAME_MAX` |
| `src-tauri/src/agentrun.rs` · `commands/agents.rs` | I/O Agent View: běh `claude` bez okna a bez Job Objectu (termín, strop výstupu), `attach` v terminálu · Tauri příkazy | `program`, `run`, `Out`, `attach` · `agent_action(id, action, confirmed)`, `agent_spawn(projectId, prompt, name)`, `AgentOut` |
| `src-tauri/src/sessionlive.rs` | I/O: vlákno `sessions` (spawn `claude agents`), watcher registru, cache | `start`, `request`, `Wake`, `with_cache`, `Cache`, `unseen`, `claude_dir`, `fetch_agents`, `reload_registry`, `registry_fallback` |
| `src-tauri/src/winfocus.rs` | Win32: strom procesů, okna, přepnutí, `claude --resume` | `switch_to`, `launch`, `ancestors`, `pick_window`, `pid_alive`, `is_claude_desktop_image` |
| `src-tauri/src/subagents.rs` | subagenti jedné session (čistá) | `Subagents::{start, stop, clear_running}`, `shorten`, `DONE_KEEP`, `MSG_CHARS` |
| `src-tauri/src/hooksetup.rs` | merge/remove/status hooků nad textem + atomický zápis | `EVENTS`, `QUESTIONS_EVENT`, `PLAN_EVENT`, `label`, `merge`, `remove`, `status`, `hint`, `ours`, `apply`, `status_at`, `status_cached`, `settings_path`, `J` |
| `src-tauri/src/claude/mod.rs` | stav session a schvalování; `handle_event` plní `Session::subagents` | `Claude`, `Session.subagents`, větve `SubagentStart`/`SubagentStop`/`SessionEnd` v `Claude::handle_event`, `handle_permission`, `decide`, `decide_with`, `decide_by`, `hold` (vystavení karty + čekání, `reject` pod zámkem), `Decision` (`Allow`/`AllowAlways`/`Deny`/`DenyWith`/`AllowMode`/`ReturnPlan`/`Pass`), `Pending.always`/`preview`/`plan`/`via`/`tool_use_id` |
| `src-tauri/src/claude/rules.rs` | „Vždy povolit“ + důvod zamítnutí (čisté): výběr pravidla, tělo odpovědi `/permission` | `propose`, `fallback`, `Rule`, `Always`, `Destination`, `Grant`, `Extra`, `resolve`, `clean_reason`, `REASON_MAX`, `body` |
| `src-tauri/src/claude/diff.rs` | řádkový diff (čistý): Myers s limitem editací, řádky karty s kontextem a stropem | `ops`, `render`, `Op`, `Kind`, `Line`, `Rendered`, `CONTEXT`, `MAX_LINES`, `MAX_D` |
| `src-tauri/src/claude/preview.rs` | náhled změny Edit/Write: čtení souboru s limitem a termínem (jen při příchodu žádosti), sestavení | `for_request`, `build`, `Preview`, `Shown`, `Disk`, `read_capped`, `read_with_deadline`, `READ_MAX`, `READ_WAIT`, `NEW_HEAD` |
| `src-tauri/src/claude/plan.rs` | review plánu ExitPlanMode: plán z `tool_input.plan`/`planFilePath`, titulek, akce → rozhodnutí podle cesty | `TOOL`, `for_request`, `from_request`, `parse`, `title`, `Plan`, `Via` (`Permission`/`PreToolUse`), `Mode`, `resolve(allow, extra, via)`, `allow_mode`, `return_message`, `PLAN_MAX` |
| `src-tauri/src/claude/planhook.rs` | plán přes `PreToolUse(ExitPlanMode)` → `/plan`: karta, tvar odpovědi, deduplikace s `PermissionRequest` | `Claude::handle_plan`, `body` (allow + `updatedInput` / deny + `permissionDecisionReason` / `None`), `key`, `tool_use_id`, `same_card`, `duplicate`, `Released`, `take_released`, `remember`, `RELEASED_MS` |
| `src-tauri/src/claude/bridge.rs` | HTTP most 127.0.0.1 (`/permission`, `/question`, `/plan`, `/event`, `/status`, `/ctl`) | `serve`, `Ctl`, `respond`, `header` |
| `src-tauri/src/claude/questions.rs` | otázky Clauda (AskUserQuestion): parsování, odpovědi, držení | `parse`, `build_answers`, `answer_body`, `MULTI_SEP`, `on_event`, `Claude::{handle_question, answer_question, release_question, questions, attention, set_question_hold}`, `Ask`, `Reply` |
| `src-tauri/src/claude/hygiene.rs` · `paths.rs` | hlídač nedokončené práce (git) · čisté cesty a porcelain | `Track`, `git_baseline`, `git_hygiene`, `run_check`, `block_reason`, `Budgets` · `lexical`, `absolutize`, `porcelain_paths` |
| `src-tauri/src/sessiongit/` (`mod.rs` · `parse.rs` · `plan.rs` · `tests.rs`) | panel Git session: I/O s termíny · rozbor `status --porcelain=v2 -z`, `worktree list --porcelain`, numstat, unified diff → `Preview` · argumenty, plán pushe, blokace, texty potvrzení, výběr ke commitu | `read_repo`, `Repo` (`linked`, `push_plan`), `view`, `View`, `FileView`, `WorktreeView`, `file_diff`, `commit`, `push`, `pull`, `discard`, `remove_worktree`, `Waits` · `status_z`, `Porcelain`, `GitFile`, `FileKind`, `numstat_z`, `worktrees`, `unified` · `PushPlan`, `push_plan`, `push_args`, `push_text`, `statuses_in`, `destructive_block`, `discardable`, `discard_text`, `remove_text`, `clean_message`, `selection`, `ENV`, `MAX_FILES` |
| `src-tauri/src/commands/sessiongit.rs` | Tauri příkazy panelu (id session → cwd → repo; blokace znovu v Rustu) | `session_git`, `session_git_diff`, `session_git_commit`, `session_git_push`, `session_git_pull`, `session_git_discard`, `session_git_remove_worktree`, `session_git_open` (→ `commands::projects::open_dir`) |
| `src-tauri/src/snapshot.rs`, `commands/sessions.rs`, `refresh.rs`, `threads/ui.rs`, `lib.rs` | lepidlo | `Snapshot.sessions` (`Vec<sessions::Row>`), `Snapshot.hooks`, `session_rows`, `session_open`, `hooks_status`/`hooks_install`/`hooks_remove`, `on_claude_change` (Wake::Hook), `UiCmd::Show` (Wake::Open), `sessionlive::start` v `run` |
| `src/ClaudeSessions.tsx` · `src/sessions/order.ts` | karta Claude (pod hlavičkou `LimitsRow` → [limits.md](limits.md), dole `CostsSection` → [costs.md](costs.md)); **strop session** (vlna 9): nejvýš `layout…cards[].limit` řádků (výchozí 4, ⋯ karty „Řádků“ / Nastavení), pod nimi „Zobrazit další (k) · +3 nečinné“ ↔ „Méně“ (`layout/ShowMore.tsx`, rozbalení zvlášť pro každou záložku); pořadí `orderSessions`: čeká na tebe (i nečinná s otázkou / schválením — nikdy se neschová) > pracuje > nečinná podle `updatedAt` > skončila; test `layout/cap.test.mjs` | `ClaudeSessions`, `SessionRow` (pilulka `statusPill` / `BgPill`, pod řádkem `AgentActions`), `PendingRow`, `onlyQuestions` („Doplnit otázky Clauda“) |
| `src/sessions/AgentActions.tsx` · `BgLaunch.tsx` · `agentview.ts` · `agentApi.ts` · `agentview.css` | Agent View v UI: akce pod session na pozadí (potvrzení v kartě, logy do rozbalení) · „Claude na pozadí…“ (formulář na `MainPage`, tlačítko pod dlaždicí `cards/Projects.tsx`, položka palety `palette/items.ts`) · čistá logika (`agentview.test.mjs`) · IPC | `AgentActions`, `BgPill` · `BgLaunch`, `BgTileButton`, `openBgLaunch`, `canBgLaunch` · `waitText`, `statusPill`, `shownActions`, `needsConfirm`, `actionLabel`, `actionHint`, `confirmText`, `promptProblem`, `nameProblem`, `bgKey` · `agentApi.{action, spawn}`; slovník `src/i18n/en-agentview.ts` (`sessions.av.*`) |
| `src/sessions/` | doplňky schválení pod Povolit/Zamítnout (karta i `cards/Peek.tsx`) · náhled diffu · review plánu · čistá logika (`permission.test.mjs`, `diff.test.mjs`, `plan.test.mjs`) | `PermExtra` (u plánu „Vrátit s připomínkou…“) · `DiffPreview` · `PlanReview` (Markdown přes `notes/Markdown.tsx`) · `permission.ts` (`DESTS`, `destFile`, `reasonOf`, `denyExtra`, `alwaysExtra`, `reasonKey`, `REASON_MAX`) · `diff.ts` (`counts`, `headKind`, `expandable`, `sign`, `gutter`, `gutterWidth`) · `plan.ts` (`planDecide`, `planSecondary`, `planTitle`, `addonsMissing`, `ADDON_HOOKS`) |
| `src/sessions/GitPanel.tsx` · `git.ts` · `git.css` | panel Git pod session („▸ Git“ v `SessionRow`): větev, ahead/behind, worktree, soubory + diff (`DiffPreview open`), Commit…/Push…/Pull/Zahodit…/Odstranit worktree… · čistá logika (`git.test.mjs`) | `GitPanel` · `badge`, `defaultPicked`, `togglePicked`, `commitReady`, `diffModes`, `messageKey`, `hiddenFiles`, `canPush` · typy `GitView`, `GitFileView`, `GitPushPlan` (`types.ts`), `api.sessionGit*` |
| `src/questions/` | otázka jako chat (peek `cards/Peek.tsx` + pod session) · čistá logika · sekce Otázky Clauda v panelu Claude `settings/panels/Questions.tsx` | `QuestionCard` · `model.ts` (`pick`, `toggleOther`, `answered`, `toReply`, `keyTarget`, `instant`) |
| `src/Settings.tsx` | sekce Claude Code | `HooksSetup` |
| `src/types.ts` · `src/api.ts` | typy · volání | `SessionView`, `HooksStatus`, `Pending.always`/`preview`/`plan`/`via`, `PendingVia`, `PermAlways`, `PermDest`, `DecideExtra` (`mode`, `inClaude`), `EditPreview`, `DiffLine`, `PlanView`, `PlanMode` · `api.decide(id, allow, extra?)`, `api.sessionOpen`, `api.hooks*` |

## Tok dat

`claude agents --json --all` + registr → `sessionlive::Cache` → `snapshot::session_rows` →
`sessions::merge(Live, claude.sessions(), dirs, name_of)` → `Snapshot.sessions` → `ClaudeSessions`.
Klik → `session_open(id)` → `Row::open_action()` → `winfocus::switch_to` / `winfocus::launch` → skrýt šuplík.
Agent View: `parse_agents` (`kind`/`id`/`state`/`waitingFor`) → `merge` → `Row.job` (`agentview::job` → `actions`) +
`waiting_for` → `AgentActions` → `agent_action(id, akce, confirmed)` → znovu `session_rows` + kontrola `job.actions` →
`agentrun::run` (`cli_args`) / `agentrun::attach` → `sessionlive::request(Wake::Open)`. Nová: `BgLaunch` → `agent_spawn` →
`bg_args` → `agentrun::run` ve složce projektu. Proužek: `stripview` (segment `waiting`) → `sessions::waiting_ids` +
`attention::waiting_count_all` (sjednocení se `sessionId` z hooků).
Schválení: hook `PermissionRequest` → `/permission` → `handle_permission` (`rules::propose` z
`permission_suggestions` / vstupu → `Pending.always`; mimo zámek `preview::for_request` → `Pending.preview`,
u `ExitPlanMode` `plan::for_request` → `Pending.plan` + titulek do `summary`) → `PendingRow`/`Peek` + `DiffPreview`
/ `PlanReview` + `PermExtra` → `decide(id, allow, extra?)` → `Claude::decide_with` → plán: `plan::resolve`, jinak
`rules::resolve` → `rules::body` (allow · allow + `updatedPermissions` addRules / setMode · deny + `message`). Zkratky (`hotkeys.rs`) dál volají `decide(None, Allow/Deny)`.
Plán (hlavní cesta): hook `PreToolUse(ExitPlanMode)` → `/plan` → `handle_plan` (`plan::for_request`, `same_card` pod
zámkem v `hold`) → `Pending { via: PreToolUse }` → stejná karta → `decide_with` → `plan::resolve(.., via)` → `planhook::body`
(allow + `updatedInput` = vstup / deny + `permissionDecisionReason` / prázdno). `Pass` → `Released` → následný
`PermissionRequest` téhož plánu (`planhook::duplicate`) = `Pass` bez karty.
Otázka: hook `PreToolUse(AskUserQuestion)` → `/question` → `handle_question` (drží `questionHoldS`) →
`Snapshot.questions` → `QuestionCard` → `question_answer` → `answer_body` (allow + `updatedInput` s `answers`).

## Invarianty a pasti

- **Spawn `claude agents` jen při otevřeném šuplíku** (`is_open` v `sessionlive::start`), max 1×/5 s
  (`spawn_wait`). Zavřený šuplík: jen levné přečtení změněných souborů registru (cache mtime+délka).
- Debounce watcheru 2 s, ale nejvýš 10 s (session pracují → registr se mění pořád).
- `Wake::Hook` jen pro session, kterou poslední seznam nezná A ozvala se po něm (`unseen`) — mrtvé
  hook session nespouští spawn dokola.
- Registr je nedokumentovaný: všechno `Option`, neznámá pole se ignorují; `*.key` (tokeny) **nečíst**
  (`is_registry_file`).
- Hook-only session po 10 s od posledního eventu, kterou čerstvý seznam nezná → `ended`; bez
  nedokončené práce se nezobrazí.
- Běžící session se nikdy neresumuje; `resume_command` odmítne id/cwd s metaznaky (`"`, `%`, `&`, `|`,
  `<`, `>`, `^`), `;` v cwd → jen `cmd` (wt ho bere jako oddělovač).
- `SetForegroundWindow` jen z kliku uživatele (Ledge pak smí). Deep link do desktopové appky neexistuje
  → přepne jen do okna appky.
- `hooksetup`: serde_json je **bez** `preserve_order` → vlastní `J` drží pořadí klíčů; nevalidní
  settings.json = chyba, nikdy přepsání. Timeout `PermissionRequest` 600 s > max `approvalTimeoutS` 550.
- Otázky: nečitelný vstup / termín / „Odpovědět v Claude“ / konec turnu = prázdná odpověď (otázka v okně
  Claude); `PermissionRequest` pro `AskUserQuestion` = vždy `Pass`. Vícevýběr `MULTI_SEP` („, “) sedí
  se schématem Claude Code 2.1.259 (test `multi_select_sends_every_checked_option`; UI `src/questions/model.test.mjs`). Hook `/question` má pevný timeout 600 s, držení max 550 (`MAX_HOLD_S`). Peek/proužek/Hide řídí
  `Claude::attention()` (schválení NEBO otázka); klávesy karty až po kliku (`focus_drawer` smí i v peeku).
- **Vždy povolit:** Ledge settings.json NIKDY needituje — vrátí `updatedPermissions` (`addRules`, `behavior:"allow"`)
  a pravidlo zapíše Claude Code. Pravidlo se vybere při příchodu žádosti a karta ho ukáže před klikem; IPC nese jen
  cíl (`localSettings` výchozí / `userSettings` / `session`), pravidla bere Rust z `Pending` → UI nemůže poslat jiné.
  Jen s konkrétním `id` (ne „nejstarší“), jen klikem — žádná klávesa. Z návrhů Clauda jen `addRules`+`allow` pro
  týž nástroj (`setMode`/`addDirectories` ignorovat; destinaci návrhu přepíše volba člověka); shell nikdy bez
  `ruleContent`; víc než 8 pravidel → nic. Bez návrhu jen Bash/PowerShell s přesným jednoduchým příkazem (bez
  `*`, `&`, `|`, `;`, `<`, `>`, `` ` ``, `$(`, řídicích znaků, mezer na krajích, ≤ 300 znaků); Edit/Write a ostatní
  bez návrhu = tlačítko schované. Allow z hooku nepřebije deny pravidlo Clauda.
- **Zamítnout s důvodem:** pole až po kliku „Zamítnout s důvodem…“ (jen tehdy `focusDrawer` + fokus do pole, po
  odeslání/zrušení `releaseFocus`). `decision.message` = „Zamítnuto v Ledge: <důvod>“; `clean_reason` ořízne,
  zahodí řídicí znaky (kromě `
`), strop 500 znaků; prázdný = obyčejné zamítnutí jako dřív.
- **Náhled diffu:** soubor se čte jen v `handle_permission` (1× při příchodu, mimo zámek stavu, `READ_MAX` 512 KB,
  termín `READ_WAIT` 1,5 s ve vlákně) — snapshot jen kopíruje `Pending.preview`. NUL / ne-UTF-8 = `Binary`, chyba =
  žádné porovnání (Write `Unread` = jen nový obsah, Edit = jen řetězce bez čísel); nic se nedopočítává odhadem. Myers
  nad `MAX_D` editací → `rough` (nahrazení bloku), strop `MAX_LINES` 400 řádků + `cut`. Řádky do DOM jen jako text.
- **Review plánu:** `Pending.plan` = karta plánu; `always` je u plánu vždy `None`. `extra.mode` / `extra.inClaude` platí
  jen u plánu a jen s konkrétním `id`; u jiné karty = nic se nerozhodne. `mode` jen u `via: Permission` (PreToolUse
  režim nastavit neumí — docs), `inClaude` = `Pass` (session dál `waiting`). Zkratky dál jen Allow/Deny (nejstarší).
  Markdown jde přes `note_render` (Rust escapuje) — žádné vlastní innerHTML. Allow přes `/plan` vždy s celým
  `updatedInput` (bez něj ExitPlanMode neprojde). Že ExitPlanMode chodí i přes `PermissionRequest`, docs neříkají —
  cesta zůstává, ale **jeden plán = jedna karta** (`same_card`, `duplicate`, `RELEASED_MS` 2 min). Nedostupný most /
  termín hooku = non-blocking error → normální dialog v Claude (docs HTTP hooků).
- `hooksetup::merge`: víc našich hooků pod jednou událostí (PreToolUse `/question` + `/plan`) — odebere se jen náš hook,
  který nesedí žádnému řádku `EVENTS` té události (jinak by se navzájem mazaly a merge by nebyl idempotentní).
- **Agent View:** jen dokumentované pole a příkazy (`claude <příkaz> --help` v 2.1.259 + docs agent-view); zprávu do
  běžící session Ledge neposílá (docs to z shellu neuvádějí), `~/.claude/jobs` nečte. Akce jen s `valid_job_id`, jen
  klikem, měnící (`stop`/`rm`/`respawn`) s potvrzením v kartě a `confirmed` v IPC; `rm` jen u done/failed/stopped.
  **Nikdy `gitinfo::capture`** pro `claude` akce: Job Object s KILL_ON_JOB_CLOSE by zabil session, kterou `--bg` /
  `respawn` nechává běžet; `agentrun::run` zabije při termínu jen přímého potomka a na konec roury čeká max 500 ms.
  Žádný shell s uživatelským textem: `claude.exe` přímo, `claude.cmd` jen s escapováním Rustu (`for_batch` = bez
  nového řádku). Úkol/jméno nesmí začínat `-`. Logy jen na klik (žádné pollování), konec 32 KB, `clean_output`
  (bez ANSI), do DOM jen jako text. Background row: klik na řádek nic neotevře (`--resume` vedle živé úlohy by
  založil druhou session) → Připojit. `waiting_ids` jen pro segment proužku, ne pro `Claude::attention()` (peek).
- Podtitul / jméno přes `clean_preview` (titulek z hooku se filtruje už v `handle_event` před zkrácením na 60 znaků).
- **Panel Git:** git jen na klik (rozbalení, akce, ↻, změna `status` session) — žádné pollování; bez okna, termíny
  `Waits` (čtení 10 s, commit 5 min kvůli hookům, síť 2 min), argumenty jako pole, `GIT_LITERAL_PATHSPECS=1`,
  `GIT_TERMINAL_PROMPT=0`, `core.fsmonitor=false`, čtení s `GIT_OPTIONAL_LOCKS=0`. Nikdy `--force`, `--no-verify`,
  `reset --hard`, `+refspec`. UI posílá jen id session + cestu; Rust ji ověří proti čerstvému statusu (`selection`,
  `Repo::file`), cestu worktree bere z `worktree list`. Destruktivní (zahodit / odstranit worktree) blokuje
  `destructive_block` (session `working`/`waiting` s cwd v kořeni repa, `statuses_in`) v Rustu i v UI; zahodit jen
  `discardable` (sledovaný soubor v HEAD — nový/nesledovaný by se smazal). Commit pathspecem = `--only` (jiné
  připravené soubory zůstanou), u přejmenování i původní cesta. Fokus do pole zprávy jen po kliku „Commit…“.
- `Row.state`/`title`/`project`/`ended` drží kompatibilitu s hlavičkou a peekem (`src/cards/Head.tsx`, `src/cards/Peek.tsx`).
- Termíny gitu hlídače: `Budgets::default()` = 2 s (produkce); git testy volají `Claude::patient`
  (60 s), aby nezávisely na vytížení stroje — v produkci nepoužívat.

## Testy

`sessions::tests` (parsování, `live_status`, `decide_open`, `merge_sources`, `resume_command`,
`spawn_throttle`), `winfocus::tests` (předci, výběr okna, cesta appky), `subagents::tests`,
`hooksetup::tests` (merge/remove/status/apply nad tempdir, matcher otázek, přechod na hook plánu, `hint`),
`claude::questions::tests` (parsování, odpovědi, držení/puštění/termín, jen ukázat), `claude::tests::subagents_from_hooks`,
`claude::rules::tests` (výběr pravidla, těla odpovědí, důvod, `resolve`, IPC `Extra`), `src/sessions/permission.test.mjs`,
`claude::diff::tests` (tabulka diffu, čísla řádků, strop, rough, náhodné vstupy proti LCS), `claude::preview::tests`
(čtení s limitem, Edit/Write nad tempdir: existuje/neexistuje/velký/binární/CRLF/replace_all), `claude::plan::tests`
(titulek, plán ze vstupu/souboru, tělo akcí, IPC `mode`/`inClaude`, `resolve_by_via`), `claude::planhook::tests`
(tvar odpovědi PreToolUse, parsování vstupu, akce karty, jeden plán = jedna karta, paměť puštěných, termín), `claude::tests::{write_request_carries_preview,
plan_request_and_actions}`, `src/sessions/diff.test.mjs`, `src/sessions/plan.test.mjs`,
`sessions::tests::{agent_view_rows, clean_preview_table}`, `agentview::tests` (stavy, waitingFor, akce podle
stavu, id, argumenty, attach, `--bg`, výpis, úklid výstupu, nalezení `claude`), `attention::tests::waiting_counts_distinct_sessions`,
`src/sessions/agentview.test.mjs`,
`sessiongit::parse::tests` (porcelain v2 -z, worktree list, numstat, unified diff), `sessiongit::plan::tests` (argumenty bez
metaznaků, plán pushe, blokace, texty, výběr), `sessiongit::tests` (git nad tempdir: diffy, commit jen výběru + doslovné
pathspecy, chyba hooku, přejmenování, push -u / push / pull --ff-only s bare remotem, zahození, worktree bez force),
`src/sessions/git.test.mjs`,
`claude::bridge::tests` (HTTP, blokace `Stop`, `/question`, `/plan` + deduplikace, Vždy povolit + důvod zamítnutí end-to-end), `claude::hygiene::tests` (git; pomůcky v `claude/testkit.rs`).
Žádný test nesahá na skutečné `~/.claude` ani nespouští `claude`.

## Co nečíst

`edge.rs`, `rawpush.rs`, `strip.rs`, `weather.rs`, `personal.rs`, `migrate.rs`, `notesync/`,
`update.rs`; `claude/hygiene.rs` + `claude/paths.rs` (hlídač gitu) — pro session nepotřebné.
