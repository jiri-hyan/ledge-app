# Codemapa: Claude session, subagenti, hooky

Karta Claude v šuplíku = všechny Claude Code session na PC + schvalování + otázky Clauda + subagenti + klik „otevřít“; Nastavení umí zapojit hooky Ledge do `~/.claude/settings.json`. Návod pro uživatele/agenty: [agents/sessions.md](../agents/sessions.md) → „Session a hooky“.

Detail (toky, invarianty, testy, co nečíst, historie): [sessions-detail.md](sessions-detail.md).

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/sessions.rs` | čistá logika: parsování zdrojů, sloučení, stav, co udělá klik, strop spawnů, náhled textu | `parse_agents` `Agent.job_id` `id` `parse_registry` `is_registry_file` |
| `src-tauri/src/agentview.rs` | Agent View (čistá): stav úlohy, `waitingFor`, které akce, validace id, argumenty `attach/logs/stop/rm/respawn` a `--bg`, úklid výstupu, nalezení `claude` | `JobState` `job_state` `WaitFor` `wait_for` `JobAction` |
| `src-tauri/src/agentrun.rs` · `commands/agents.rs` | I/O Agent View: běh `claude` bez okna a bez Job Objectu (termín, strop výstupu), `attach` v terminálu · Tauri příkazy | `program` `run` `Out` `attach` `agent_action(id, action, confirmed)` |
| `src-tauri/src/sessionlive.rs` | I/O: vlákno `sessions` (spawn `claude agents`), watcher registru, cache | `start` `request` `Wake` `with_cache` `Cache` `unseen` `claude_dir` |
| `src-tauri/src/winfocus.rs` | Win32: strom procesů, okna, přepnutí, `claude --resume` | `switch_to` `launch` `ancestors` `pick_window` `pid_alive` |
| `src-tauri/src/subagents.rs` | subagenti jedné session (čistá) | `Subagents::{start, stop, clear_running}` `shorten` `DONE_KEEP` |
| `src-tauri/src/hooksetup.rs` | merge/remove/status hooků nad textem + atomický zápis | `EVENTS` `QUESTIONS_EVENT` `PLAN_EVENT` `label` `merge` `remove` |
| `src-tauri/src/claude/mod.rs` | stav session a schvalování; `handle_event` plní `Session::subagents` | `Claude` `Session.subagents` `SubagentStart` `SubagentStop` |
| `src-tauri/src/claude/rules.rs` | „Vždy povolit“ + důvod zamítnutí (čisté): výběr pravidla, tělo odpovědi `/permission` | `propose` `fallback` `Rule` `Always` `Destination` `Grant` `Extra` |
| `src-tauri/src/claude/diff.rs` | řádkový diff (čistý): Myers s limitem editací, řádky karty s kontextem a stropem | `ops` `render` `Op` `Kind` `Line` `Rendered` `CONTEXT` `MAX_LINES` |
| `src-tauri/src/claude/preview.rs` | náhled změny Edit/Write: čtení souboru s limitem a termínem (jen při příchodu žádosti), sestavení | `for_request` `build` `Preview` `Shown` `Disk` `read_capped` |
| `src-tauri/src/claude/plan.rs` | review plánu ExitPlanMode: plán z `tool_input.plan`/`planFilePath`, titulek, akce → rozhodnutí podle cesty | `TOOL` `for_request` `from_request` `parse` `title` `Plan` `Via` |
| `src-tauri/src/claude/planhook.rs` | plán přes `PreToolUse(ExitPlanMode)` → `/plan`: karta, tvar odpovědi, deduplikace s `PermissionRequest` | `Claude::handle_plan` `body` `updatedInput` `permissionDecisionReason` |
| `src-tauri/src/claude/bridge.rs` | HTTP most 127.0.0.1 (`/permission`, `/question`, `/plan`, `/event`, `/status`, `/ctl`) | `serve`, `Ctl`, `respond`, `header` |
| `src-tauri/src/claude/questions.rs` | otázky Clauda (AskUserQuestion): parsování, odpovědi, držení po úsecích (`unseen::step`) | `parse` `build_answers` `answer_body` `MULTI_SEP` `on_event` |
| `src-tauri/src/claude/unseen.rs` | pojistka neviděné otázky (`claude.questionUnseenS`): co dělá vlákno hooku, brána potvrzení, puštění do Claude + deník `releasedUnseen` | `step` `Step` `seen_counts` `clamp_s` `released_detail` `DEFAULT_S` |
| `src-tauri/src/attention.rs` (část) | otázka čeká se zavřeným šuplíkem → puls po `REPULSE_MS`; peek po konci hry / herního režimu (volá vlákno `emit`) | `repulse_due` `next_repulse` `revive_peek` `catch_up` `next_deadline` |
| `src-tauri/src/claude/hygiene.rs` · `paths.rs` | hlídač nedokončené práce (git) · čisté cesty a porcelain | `Track` `git_baseline` `git_hygiene` `run_check` `block_reason` |
| `src-tauri/src/sessiongit/` (`mod.rs` · `parse.rs` · `plan.rs` · `tests.rs`) | panel Git session: I/O s termíny · rozbor `status --porcelain=v2 -z`, `worktree list --porcelain`, numstat, unified diff → `Preview` · argumenty, plán pushe … | `read_repo` `Repo` `linked` `push_plan` `view` `View` `FileView` |
| `src-tauri/src/commands/sessiongit.rs` | Tauri příkazy panelu (id session → cwd → repo; blokace znovu v Rustu) | `session_git` `session_git_diff` `session_git_commit` |
| `src-tauri/src/snapshot.rs`, `commands/sessions.rs`, `refresh.rs`, `threads/ui.rs`, `lib.rs` | lepidlo | `Snapshot.sessions` `Vec<sessions::Row>` `Snapshot.hooks` |
| `src/ClaudeSessions.tsx` · `src/sessions/order.ts` | karta Claude (pod hlavičkou `LimitsRow` → [limits.md](limits.md), dole `CostsSection` → [costs.md](costs.md)); strop session (vlna 9) … | `ClaudeSessions` `SessionRow` `statusPill` `BgPill` `AgentActions` |
| `src/sessions/AgentActions.tsx` · `BgLaunch.tsx` · `agentview.ts` · `agentApi.ts` · `agentview.css` | Agent View v UI: akce pod session na pozadí (potvrzení v kartě, logy do rozbalení) · „Claude na pozadí…“ (formulář na `MainPage`, tlačítko pod dlaždicí … | `AgentActions` `BgPill` `BgLaunch` `BgTileButton` `openBgLaunch` |
| `src/sessions/ContextBar.tsx` · `ctx.ts` | pruh kontextu pod řádkem session (kolo 23): `api.sessionContext` → Rust `ctxsize` (poslední usage v `Session.transcript` z hooku `transcript_path`) … | `ContextBar`, `Claude::transcript_of`, `session_context` |
| `src-tauri/src/claude/activity.rs` · `claude/activity_tests.rs` | živá aktivita session (kolo 23): fáze = ODHAD z posledního nástroje (`PostToolUse` … | `Phase` `Activity` `Session.activity` `Row.activity` `next` `phase_of` |
| `src/sessions/ActivityCard.tsx` · `activity.ts` · `activity.css` · `en-activity.ts` | karta „Claude aktivita“ (id `activity`, jen z katalogu): session s fází (štítek „odhad podle posledního nástroje“, bez kroku k/4 … | `ActivityCard` `PHASES` `baseName` `shown` `orderActivity` `livePhase` |
| `src/sessions/` | doplňky schválení pod Povolit/Zamítnout (karta i `cards/Peek.tsx`) · náhled diffu · review plánu · čistá logika (`permission.test.mjs`, `diff.test.mjs` … | `PermExtra` `DiffPreview` `PlanReview` `notes/Markdown.tsx` |
| `src/sessions/GitPanel.tsx` · `git.ts` · `git.css` | panel Git pod session („▸ Git“ v `SessionRow`): větev, ahead/behind, worktree, soubory + diff (`DiffPreview open`), Commit…/Push…/Pull/Zahodit…/Odstranit … | `GitPanel` `badge` `defaultPicked` `togglePicked` `commitReady` |
| `src/questions/` (`QuestionCard.tsx` · `model.ts` · `seen.ts`) | otázka jako chat (peek `cards/Peek.tsx` + pod session) · čistá logika · potvrzení „zobrazeno“ · sekce Otázky Clauda v panelu Claude … | `QuestionCard` `model.ts` `pick` `toggleOther` `answered` `toReply` |
| `src/Settings.tsx` | sekce Claude Code | `HooksSetup` |
| `src/types/claude.ts` · `src/api.ts` | typy · volání | `SessionView` `HooksStatus` `Pending.always` `preview` `plan` `via` |

Další soubory (podrobnosti v detailu):
`src/sessions/{diff.ts,permission.ts}` · `src/settings/panels/Questions.tsx`
