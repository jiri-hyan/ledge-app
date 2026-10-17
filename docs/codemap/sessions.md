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
| `src-tauri/src/claude/mod.rs` | stav session a schvalování; `handle_event` plní `Session::subagents` | `Claude`, `Session.subagents`, větve `SubagentStart`/`SubagentStop`/`SessionEnd` v `Claude::handle_event`, `handle_permission`, `decide` |
| `src-tauri/src/claude/bridge.rs` | HTTP most 127.0.0.1 (`/permission`, `/question`, `/event`, `/status`, `/ctl`) | `serve`, `Ctl`, `respond`, `header` |
| `src-tauri/src/claude/questions.rs` | otázky Clauda (AskUserQuestion): parsování, odpovědi, držení | `parse`, `build_answers`, `answer_body`, `MULTI_SEP`, `on_event`, `Claude::{handle_question, answer_question, release_question, questions, attention, set_question_hold}`, `Ask`, `Reply` |
| `src-tauri/src/claude/hygiene.rs` · `paths.rs` | hlídač nedokončené práce (git) · čisté cesty a porcelain | `Track`, `git_baseline`, `git_hygiene`, `run_check`, `block_reason`, `Budgets` · `lexical`, `absolutize`, `porcelain_paths` |
| `src-tauri/src/snapshot.rs`, `commands/sessions.rs`, `refresh.rs`, `threads/ui.rs`, `lib.rs` | lepidlo | `Snapshot.sessions` (`Vec<sessions::Row>`), `Snapshot.hooks`, `session_rows`, `session_open`, `hooks_status`/`hooks_install`/`hooks_remove`, `on_claude_change` (Wake::Hook), `UiCmd::Show` (Wake::Open), `sessionlive::start` v `run` |
| `src/ClaudeSessions.tsx` | karta Claude | `ClaudeSessions`, `SessionRow`, `PendingRow`, `onlyQuestions` („Doplnit otázky Clauda“) |
| `src/questions/` | otázka jako chat (peek `cards/Peek.tsx` + pod session) · čistá logika · panel `settings/panels/Questions.tsx` | `QuestionCard` · `model.ts` (`pick`, `toggleOther`, `answered`, `toReply`, `keyTarget`, `instant`) |
| `src/Settings.tsx` | sekce Claude Code | `HooksSetup` |
| `src/types.ts` · `src/api.ts` | typy · volání | `SessionView`, `HooksStatus` · `api.sessionOpen`, `api.hooks*` |

## Tok dat

`claude agents --json --all` + registr → `sessionlive::Cache` → `snapshot::session_rows` →
`sessions::merge(Live, claude.sessions(), dirs, name_of)` → `Snapshot.sessions` → `ClaudeSessions`.
Klik → `session_open(id)` → `Row::open_action()` → `winfocus::switch_to` / `winfocus::launch` → skrýt šuplík.
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
- `Row.state`/`title`/`project`/`ended` drží kompatibilitu s hlavičkou a peekem (`src/cards/Head.tsx`, `src/cards/Peek.tsx`).
- Termíny gitu hlídače: `Budgets::default()` = 2 s (produkce); git testy volají `Claude::patient`
  (60 s), aby nezávisely na vytížení stroje — v produkci nepoužívat.

## Testy

`sessions::tests` (parsování, `live_status`, `decide_open`, `merge_sources`, `resume_command`,
`spawn_throttle`), `winfocus::tests` (předci, výběr okna, cesta appky), `subagents::tests`,
`hooksetup::tests` (merge/remove/status/apply nad tempdir, matcher otázek, `hint`),
`claude::questions::tests` (parsování, odpovědi, držení/puštění/termín, jen ukázat), `claude::tests::subagents_from_hooks`,
`claude::bridge::tests` (HTTP, blokace `Stop`, `/question`), `claude::hygiene::tests` (git; pomůcky v `claude/testkit.rs`).
Žádný test nesahá na skutečné `~/.claude` ani nespouští `claude`.

## Co nečíst

`edge.rs`, `rawpush.rs`, `strip.rs`, `weather.rs`, `personal.rs`, `migrate.rs`, `notesync/`,
`update.rs`; `claude/hygiene.rs` + `claude/paths.rs` (hlídač gitu) — pro session nepotřebné.
