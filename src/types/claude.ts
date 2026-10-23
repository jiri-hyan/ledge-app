export interface Pending {
  id: number;
  sessionId: string;
  cwd: string;
  tool: string;
  summary: string;
  createdAt: number;
  expiresAt: number;
  /** návrh „Vždy povolit“ (přesné pravidlo, `claude/rules.rs`); null = tlačítko se neukáže */
  always: PermAlways | null;
  /** náhled změny Edit/Write (`claude/preview.rs`, sestavený při příchodu žádosti); null = bez náhledu */
  preview: EditPreview | null;
  /** plán k review (ExitPlanMode, `claude/plan.rs`); null = obyčejné schválení */
  plan: PlanView | null;
  /** odkud karta přišla (`plan::Via`): hook PermissionRequest, nebo PreToolUse(ExitPlanMode) — ten režim nastavit neumí */
  via: PendingVia;
}
export type PendingVia = "permission" | "preToolUse";

/** řádek náhledu diffu (`claude/diff.rs` `Line`); čísla 1-based, null = na té straně není */
export interface DiffLine { kind: "ctx" | "add" | "del" | "gap"; old: number | null; new: number | null; text: string }
/** co náhled ukazuje (`preview::Shown`) */
export type PreviewShown = "diff" | "newFile" | "unread" | "tooBig" | "binary";
export interface EditPreview {
  file: string;
  path: string;
  shown: PreviewShown;
  added: number;
  removed: number;
  lines: DiffLine[];
  /** řádků za stropem (neukázaných) */
  cut: number;
  /** čísla řádků jsou ze skutečného souboru */
  numbered: boolean;
  replaceAll: boolean;
  /** kolikrát je `old_string` v souboru (jen když se soubor přečetl) */
  occurrences: number | null;
  /** velký rozdíl ukázaný jako nahrazení bloku */
  rough: boolean;
  /** řádků nového obsahu (newFile / unread) */
  total: number;
}
export interface PlanView { markdown: string; title: string; file: string | null; cut: boolean }
/** režim po schválení plánu (`plan::Mode`) */
export type PlanMode = "acceptEdits";

export interface PermRule { toolName: string; ruleContent?: string }
export interface PermAlways { rules: PermRule[]; label: string; fromClaude: boolean }
/** kam Claude Code pravidlo zapíše (`destination` v `updatedPermissions`) */
export type PermDest = "localSettings" | "userSettings" | "session";
/** doplněk `decide`: „Vždy povolit“ do cíle (jen s allow) / důvod zamítnutí (jen bez allow) /
 *  u plánu režim „Schválit a povolit úpravy“ (jen s allow) */
export interface DecideExtra { always?: PermDest; reason?: string; mode?: PlanMode; inClaude?: boolean }

export interface SubagentRun { id: string; agentType: string; since: number }
export interface SubagentDone { agentType: string; durationMs: number | null; endedAt: number; message: string | null }

/** Claude Code session na PC (`sessions.rs`): `claude agents` + registr + hooky Ledge. */
export interface SessionView {
  id: string;
  name: string;
  /** první prompt z hooků, jen když se liší od `name` */
  title: string | null;
  cwd: string;
  /** název a id projektu podle cwd */
  project: string | null;
  projectId: string | null;
  status: "waiting" | "working" | "idle" | "ended";
  /** hrubší `status` pro hlavičku */
  state: "working" | "waiting" | "idle";
  /** od kdy je ve stavu (ms), null = nevíme */
  since: number | null;
  updatedAt: number;
  /** "claude-desktop" | "cli" | … (z registru, může chybět) */
  entrypoint: string | null;
  pid: number | null;
  /** null = nevíme, jestli proces běží */
  alive: boolean | null;
  /** session posílá Ledge hooky (schvalování, hlídač, subagenti) */
  hooked: boolean;
  /** soubory, které session sama upravila a nezacommitovala (relativně k repu) */
  uncommitted: string[];
  /** commity session od jejího startu, které nejsou na žádném remote */
  unpushed: number;
  /** session skončila, drží se jen kvůli nedokončené práci */
  ended: boolean;
  subagents: { running: SubagentRun[]; done: SubagentDone[] };
  canOpen: boolean;
  /** co udělá klik (nebo proč nejde) */
  openHint: string;
  /** Agent View (`agentview.rs`): `kind: background` */
  background: boolean;
  /** session na pozadí s krátkým id: stav úlohy a akce, které Rust nabízí */
  job: JobView | null;
  /** na co čeká (`waitingFor`), jen ve stavu „čeká“; neznámá hodnota = null */
  waitingFor: WaitFor | null;
}

/** Agent View — session na pozadí (`claude agents --json`, Rust `agentview.rs`). */
export type JobState = "working" | "blocked" | "done" | "failed" | "stopped";
export type JobAction = "attach" | "logs" | "stop" | "remove" | "respawn";
export type WaitFor = "permission" | "input" | "sandbox" | "worker" | "dialog";
export interface JobView {
  /** krátké id (`claude attach|logs|stop <id>`) */
  id: string;
  state: JobState | null;
  actions: JobAction[];
}
/** Výsledek `agent_action`: výstup logů (konec, strop 32 KB) nebo krátká zpráva CLI. */
export interface AgentOut {
  text: string;
  cut: boolean;
}

/** Hooky Ledge v `~/.claude/settings.json`. */
export interface HooksStatus {
  state: "none" | "partial" | "full" | "error";
  missing: string[];
  error: string | null;
  path: string;
  /** lidská hláška k `partial` (chybí jen hook otázek → „Doplnit otázky Clauda“) */
  hint: string | null;
}

// --- questions ---
/** Možnost otázky Clauda (`AskUserQuestion`); `preview` = volitelný náhled (text/markup od Clauda). */
export interface QuestionOption { label: string; description: string; preview: string | null }
export interface ClaudeQuestion {
  /** přesný text — klíč odpovědi */
  question: string;
  /** krátký štítek (může být prázdný) */
  header: string;
  options: QuestionOption[];
  multiSelect: boolean;
}
/** Otázka Clauda v Ledge (Rust `claude/questions.rs`). */
export interface Ask {
  id: number;
  sessionId: string;
  cwd: string;
  questions: ClaudeQuestion[];
  createdAt: number;
  /** held: do kdy Ledge drží odpověď; jen ukázaná: do kdy ji ukazuje */
  expiresAt: number;
  /** false = `questionHoldS` 0 — odpovídá se v okně Claude */
  answerable: boolean;
}
/** Řádek deníku otázek (Rust `questions::QuestionLog`, posledních 20 od spuštění, jen paměť). */
export interface QuestionLog {
  id: number;
  at: number;
  sessionId: string;
  /** štítek / text první otázky; "" = neznámá */
  title: string;
  outcome: "holding" | "shown" | "answered" | "released" | "expired" | "turnEnded" | "skipped";
  /** lidská věta (přeložená v jádře) */
  detail?: string | null;
}
/** Odpověď na jednu otázku: vybrané možnosti + vlastní text („Jiné…“). */
export interface QuestionReply { labels: string[]; other: string | null }

// --- limits ---
// Zrcadlo `limitslive::LimitsView` / `statuslinesetup::StatusLineStatus` (Rust) — limity Claude 5h / týden.
export interface LimitsView {
  /** kdy vznikl vzorek (ms); null = žádná data */
  at: number | null;
  source: "statusLine" | "usageLog" | null;
  fiveHour: { pct: number; resetsAt: number | null } | null;
  sevenDay: { pct: number; resetsAt: number | null } | null;
  /** starší vzorek UI ukáže jako „staré“ */
  staleMs: number;
  /** stavový řádek Ledge v settings.json; "" = ještě nečteno */
  statusLine: "none" | "foreign" | "full" | "error" | "";
  /** běžící připomínka resetu (peek) */
  notice: "fiveHour" | "sevenDay" | null;
}
export interface StatusLineStatus {
  state: "none" | "foreign" | "full" | "error";
  /** příkaz, který Ledge obaluje / obalí */
  command: string | null;
  error: string | null;
  path: string;
}

// --- costs ---
// Zrcadlo `costslive::CostsView` (Rust) — útrata Claude, odhad v cenách API (typy v `costs/model.ts`).
export type { CostsView } from "../costs/model";
/** panel Git v kartě Claude (`sessiongit/`, `src/sessions/GitPanel.tsx`) — soubor ze `git status` */
export interface GitFileView {
  /** relativně ke kořeni repa, s `/` */
  path: string;
  /** původní cesta u přejmenování */
  orig: string | null;
  kind: "changed" | "renamed" | "conflict" | "untracked";
  /** index / pracovní složka: `.` beze změny, `?` nesledovaný, jinak písmeno gitu (M A D R U…) */
  x: string;
  y: string;
  inHead: boolean;
  /** řádky proti HEAD; null = nevíme */
  added: number | null;
  removed: number | null;
  /** text druhého potvrzení zahození; null = zahodit nejde */
  discard: string | null;
}
export type GitPushPlan =
  | { kind: "upstream"; target: string; count: number }
  | { kind: "setUpstream"; remote: string; branch: string; count: number | null }
  | { kind: "no"; why: string };
export interface GitView {
  root: string;
  /** null = odpojená HEAD */
  branch: string | null;
  initial: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: GitFileView[];
  /** všech změněných souborů (files je nejvýš 200) */
  total: number;
  /** cwd je přidaný worktree */
  worktree: { path: string; branch: string | null; main: string; mainBranch: string | null; locked: boolean; remove: string | null } | null;
  push: GitPushPlan;
  pushText: string | null;
  canPull: boolean;
  /** proč jsou destruktivní akce zablokované (živá session v té složce) */
  block: string | null;
}
