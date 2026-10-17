# Codemapa: Claude session, subagenti, hooky

Karta Claude v šuplíku = všechny Claude Code session na PC + schvalování + otázky Clauda + subagenti + klik „otevřít“;
Nastavení umí zapojit hooky Ledge do `~/.claude/settings.json`. Návod pro uživatele/agenty: `AGENTS.md`
→ „Session a hooky“.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src-tauri/src/sessions.rs` | čistá logika: parsování zdrojů, sloučení, stav, co udělá klik, strop spawnů | `parse_agents`, `parse_registry`, `is_registry_file`, `live_status`, `Status`, `merge`, `Live`, `Row`, `decide_open`, `OpenAction`, `open_hint`, `spawn_wait`, `resume_command`, `Launch` |
| `src-tauri/src/sessionlive.rs` | I/O: vlákno `sessions` (spawn `claude agents`), watcher registru, cache | `start`, `request`, `Wake`, `with_cache`, `Cache`, `unseen`, `claude_dir`, `fetch_agents`, `reload_registry`, `registry_fallback` |
| `src-tauri/src/winfocus.rs` | Win32: strom procesů, okna, přepnutí, `claude --resume` | `switch_to`, `launch`, `ancestors`, `pick_window`, `pid_alive`, `is_claude_desktop_image` |
| `src-tauri/src/subagents.rs` | subagenti jedné session (čistá) | `Subagents::{start, stop, clear_running}`, `shorten`, `DONE_KEEP`, `MSG_CHARS` |
| `src-tauri/src/hooksetup.rs` | merge/remove/status hooků nad textem + atomický zápis | `EVENTS`, `merge`, `remove`, `status`, `ours`, `apply`, `status_at`, `status_cached`, `settings_path`, `J` |
| `src-tauri/src/claude/mod.rs` | stav session a schvalování; `handle_event` plní `Session::subagents` | `Claude`, `Session.subagents`, větve `SubagentStart`/`SubagentStop`/`SessionEnd` v `Claude::handle_event`, `handle_permission`, `decide`, `decide_with`, `decide_by`, `Decision` (`Allow`/`AllowAlways`/`Deny`/`DenyWith`/`AllowMode`/`ReturnPlan`/`Pass`), `Pending.always`/`preview`/`plan` |
| `src-tauri/src/claude/rules.rs` | „Vždy povolit“ + důvod zamítnutí (čisté): výběr pravidla, tělo odpovědi `/permission` | `propose`, `fallback`, `Rule`, `Always`, `Destination`, `Grant`, `Extra`, `resolve`, `clean_reason`, `REASON_MAX`, `body` |
| `src-tauri/src/claude/diff.rs` | řádkový diff (čistý): Myers s limitem editací, řádky karty s kontextem a stropem | `ops`, `render`, `Op`, `Kind`, `Line`, `Rendered`, `CONTEXT`, `MAX_LINES`, `MAX_D` |
| `src-tauri/src/claude/preview.rs` | náhled změny Edit/Write: čtení souboru s limitem a termínem (jen při příchodu žádosti), sestavení | `for_request`, `build`, `Preview`, `Shown`, `Disk`, `read_capped`, `read_with_deadline`, `READ_MAX`, `READ_WAIT`, `NEW_HEAD` |
| `src-tauri/src/claude/plan.rs` | review plánu ExitPlanMode: plán z `tool_input.plan`/`planFilePath`, titulek, tři akce → rozhodnutí | `TOOL`, `for_request`, `from_request`, `parse`, `title`, `Plan`, `Mode`, `resolve`, `allow_mode`, `return_message`, `PLAN_MAX` |
| `src-tauri/src/claude/bridge.rs` | HTTP most 127.0.0.1 (`/permission`, `/question`, `/event`, `/status`, `/ctl`) | `serve`, `Ctl`, `respond`, `header` |
| `src-tauri/src/claude/questions.rs` | otázky Clauda (AskUserQuestion): parsování, odpovědi, držení | `parse`, `build_answers`, `answer_body`, `MULTI_SEP`, `on_event`, `Claude::{handle_question, answer_question, release_question, questions, attention, set_question_hold}`, `Ask`, `Reply` |
| `src-tauri/src/claude/hygiene.rs` · `paths.rs` | hlídač nedokončené práce (git) · čisté cesty a porcelain | `Track`, `git_baseline`, `git_hygiene`, `run_check`, `block_reason`, `Budgets` · `lexical`, `absolutize`, `porcelain_paths` |
| `src-tauri/src/snapshot.rs`, `commands/sessions.rs`, `refresh.rs`, `threads/ui.rs`, `lib.rs` | lepidlo | `Snapshot.sessions` (`Vec<sessions::Row>`), `Snapshot.hooks`, `session_rows`, `session_open`, `hooks_status`/`hooks_install`/`hooks_remove`, `on_claude_change` (Wake::Hook), `UiCmd::Show` (Wake::Open), `sessionlive::start` v `run` |
| `src/ClaudeSessions.tsx` | karta Claude (pod hlavičkou `LimitsRow` → [limits.md](limits.md), dole `CostsSection` → [costs.md](costs.md)) | `ClaudeSessions`, `SessionRow`, `PendingRow`, `onlyQuestions` („Doplnit otázky Clauda“) |
| `src/sessions/` | doplňky schválení pod Povolit/Zamítnout (karta i `cards/Peek.tsx`) · náhled diffu · review plánu · čistá logika (`permission.test.mjs`, `diff.test.mjs`, `plan.test.mjs`) | `PermExtra` (u plánu „Vrátit s připomínkou…“) · `DiffPreview` · `PlanReview` (Markdown přes `notes/Markdown.tsx`) · `permission.ts` (`DESTS`, `destFile`, `reasonOf`, `denyExtra`, `alwaysExtra`, `reasonKey`, `REASON_MAX`) · `diff.ts` (`counts`, `headKind`, `expandable`, `sign`, `gutter`, `gutterWidth`) · `plan.ts` (`planDecide`, `planTitle`) |
| `src/questions/` | otázka jako chat (peek `cards/Peek.tsx` + pod session) · čistá logika · panel `settings/panels/Questions.tsx` | `QuestionCard` · `model.ts` (`pick`, `toggleOther`, `answered`, `toReply`, `keyTarget`, `instant`) |
| `src/Settings.tsx` | sekce Claude Code | `HooksSetup` |
| `src/types.ts` · `src/api.ts` | typy · volání | `SessionView`, `HooksStatus`, `Pending.always`/`preview`/`plan`, `PermAlways`, `PermDest`, `DecideExtra` (`mode`), `EditPreview`, `DiffLine`, `PlanView`, `PlanMode` · `api.decide(id, allow, extra?)`, `api.sessionOpen`, `api.hooks*` |

## Tok dat

`claude agents --json --all` + registr → `sessionlive::Cache` → `snapshot::session_rows` →
`sessions::merge(Live, claude.sessions(), dirs, name_of)` → `Snapshot.sessions` → `ClaudeSessions`.
Klik → `session_open(id)` → `Row::open_action()` → `winfocus::switch_to` / `winfocus::launch` → skrýt šuplík.
Schválení: hook `PermissionRequest` → `/permission` → `handle_permission` (`rules::propose` z
`permission_suggestions` / vstupu → `Pending.always`; mimo zámek `preview::for_request` → `Pending.preview`,
u `ExitPlanMode` `plan::for_request` → `Pending.plan` + titulek do `summary`) → `PendingRow`/`Peek` + `DiffPreview`
/ `PlanReview` + `PermExtra` → `decide(id, allow, extra?)` → `Claude::decide_with` → plán: `plan::resolve`, jinak
`rules::resolve` → `rules::body` (allow · allow + `updatedPermissions` addRules / setMode · deny + `message`). Zkratky (`hotkeys.rs`) dál volají `decide(None, Allow/Deny)`.
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
- **Review plánu:** `Pending.plan` = karta plánu; `always` je u plánu vždy `None`. `extra.mode` platí jen u plánu a jen
  s konkrétním `id`; u jiné karty = nic se nerozhodne. Zkratky dál jen Allow/Deny (nejstarší). Markdown jde přes
  `note_render` (Rust escapuje) — žádné vlastní innerHTML. Že ExitPlanMode chodí přes `PermissionRequest`, není
  v dokumentaci ověřené (lead ověří živě); když nepřijde, nic se nemění.
- `Row.state`/`title`/`project`/`ended` drží kompatibilitu s hlavičkou a peekem (`src/cards/Head.tsx`, `src/cards/Peek.tsx`).
- Termíny gitu hlídače: `Budgets::default()` = 2 s (produkce); git testy volají `Claude::patient`
  (60 s), aby nezávisely na vytížení stroje — v produkci nepoužívat.

## Testy

`sessions::tests` (parsování, `live_status`, `decide_open`, `merge_sources`, `resume_command`,
`spawn_throttle`), `winfocus::tests` (předci, výběr okna, cesta appky), `subagents::tests`,
`hooksetup::tests` (merge/remove/status/apply nad tempdir, matcher otázek, `hint`),
`claude::questions::tests` (parsování, odpovědi, držení/puštění/termín, jen ukázat), `claude::tests::subagents_from_hooks`,
`claude::rules::tests` (výběr pravidla, těla odpovědí, důvod, `resolve`, IPC `Extra`), `src/sessions/permission.test.mjs`,
`claude::diff::tests` (tabulka diffu, čísla řádků, strop, rough, náhodné vstupy proti LCS), `claude::preview::tests`
(čtení s limitem, Edit/Write nad tempdir: existuje/neexistuje/velký/binární/CRLF/replace_all), `claude::plan::tests`
(titulek, plán ze vstupu/souboru, tělo tří akcí, IPC `mode`), `claude::tests::{write_request_carries_preview,
plan_request_and_actions}`, `src/sessions/diff.test.mjs`, `src/sessions/plan.test.mjs`,
`claude::bridge::tests` (HTTP, blokace `Stop`, `/question`, Vždy povolit + důvod zamítnutí end-to-end), `claude::hygiene::tests` (git; pomůcky v `claude/testkit.rs`).
Žádný test nesahá na skutečné `~/.claude` ani nespouští `claude`.

## Co nečíst

`edge.rs`, `rawpush.rs`, `strip.rs`, `weather.rs`, `personal.rs`, `migrate.rs`, `notesync/`,
`update.rs`; `claude/hygiene.rs` + `claude/paths.rs` (hlídač gitu) — pro session nepotřebné.
