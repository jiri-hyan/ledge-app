// Typy snapshotu a IPC (zrcadlo serde struktur v `src-tauri/src`) — jen tvary dat, žádná logika.
// Volání jádra jsou v `api.ts`, který tyto typy zároveň re-exportuje (`import type { … } from "./api"`).

export type View = "hidden" | "peek" | "full";

export interface ViewMsg {
  view: View;
  morph: boolean;
  fromW: number;
  fromH: number;
  toW: number;
  toH: number;
  /** strop výšky obsahu na monitoru (logické px); nad ním se posouvá uvnitř stránky */
  maxH: number;
}
export type Phase = "preparing" | "starting" | "ready" | "running" | "exited" | "failed";

export interface GitInfo { branch: string | null; dirty: number; ahead: number; behind: number }
export interface Link { label: string; url: string }

export interface ProjectView {
  id: string;
  name: string;
  kind: "dev" | "app";
  accent: string;
  monogram: string;
  /** klíč obsahu loga; data URL přes `api.projectLogo` (jednou na klíč) */
  logoKey: string | null;
  path: string | null;
  pathOk: boolean;
  /** složka chybí a jde naklonovat do roots[0]; `cloning` = klon běží */
  clonable: boolean;
  cloning: boolean;
  github: string | null;
  links: Link[];
  git: GitInfo | null;
  run: string | null;
}

export interface ProcView {
  id: string;
  phase: Phase;
  pid: number | null;
  port: number | null;
  url: string | null;
  startedAt: number;
  readyAt: number | null;
  exitCode: number | null;
  message: string | null;
  lastLine: string | null;
  errors: number;
}

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
}

export interface PermRule { toolName: string; ruleContent?: string }
export interface PermAlways { rules: PermRule[]; label: string; fromClaude: boolean }
/** kam Claude Code pravidlo zapíše (`destination` v `updatedPermissions`) */
export type PermDest = "localSettings" | "userSettings" | "session";
/** doplněk `decide`: „Vždy povolit“ do cíle (jen s allow) / důvod zamítnutí (jen bez allow) */
export interface DecideExtra { always?: PermDest; reason?: string }

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

export interface Sections { projects: boolean; running: boolean; claude: boolean; notes: boolean }

/** Poznámka z `%USERPROFILE%\Ledge\notes` (jeden `.md` s frontmatter). */
export interface NoteView {
  /** absolutní cesta k `.md` — zároveň klíč */
  path: string;
  /** id projektu, null = obecná */
  project: string | null;
  /** ISO 8601 s offsetem */
  created: string;
  createdMs: number;
  remind: string | null;
  remindMs: number | null;
  done: boolean;
  /** první řádek bez Markdown značek */
  title: string;
  images: number;
  /** kategorie (`kind:`), "info" když chybí; neznámé id z druhého PC projde beze změny */
  kind: string;
  /** poslední úprava v Ledge (ISO s offsetem) */
  updated: string | null;
  tasksDone: number;
  tasksTotal: number;
}

/** Kategorie poznámek z `notes.kinds`. */
export interface NoteKind { id: string; label: string; color: string; icon: string }

/** Poznámka k úpravě (`note_read`). */
export interface NoteDetail extends NoteView {
  /** tělo bez frontmatteru; řádky úkolů se počítají v něm */
  body: string;
  /** otisk souboru při čtení — uložení ho chce zpátky */
  hash: string;
  /** relativní obrázky (`./a-1.png`) jako data URL */
  imageData: { src: string; data: string }[];
}

export interface SaveNote {
  path: string;
  expectedHash: string;
  body: string;
  project: string | null;
  kind: string | null;
  /** lokální čas `YYYY-MM-DDTHH:MM` — offset doplní Rust; null = bez připomínky */
  remind: string | null;
  /** nové PNG (base64 bez prefixu), připojí se na konec */
  images: string[];
}
export type SaveOutcome = { outcome: "saved"; note: NoteDetail } | { outcome: "changed" };

export interface NoteQuery { kind?: string | null; project?: string | null; includeDone?: boolean; limit?: number }
export interface QueryResult { notes: NoteView[]; total: number }

/** Rozepsaná úprava existující poznámky (`<root>\.draft\edit.json`). */
export interface EditDraft { path: string; hash: string; text: string; project: string; kind: string; remind: string }

export interface NewNote {
  text: string;
  project: string | null;
  /** hodnota z `datetime-local` (lokální čas) — offset doplní Rust */
  remind: string | null;
  /** PNG v base64 (bez prefixu data:) */
  images: string[];
  /** kategorie; null = výchozí */
  kind: string | null;
}

/** Počasí z Open-Meteo (stahuje Rust, jen při otevření Osobní a datech starších 30 min). */
export interface Weather {
  /** °C */
  temp: number;
  /** WMO kód */
  code: number;
  desc: string;
  icon: string;
  isDay: boolean;
  /** dnešní min/max °C */
  min: number | null;
  max: number | null;
  /** dnešní srážky celkem (mm) a nejvyšší pravděpodobnost (%) */
  precipSum: number | null;
  precipProb: number | null;
  /** čas měření v místním čase místa */
  observed: string;
}

export interface WeatherView {
  /** unset = místo nenastavené (žádná síť) · stale = poslední dobrá data, poslední pokus selhal */
  status: "unset" | "loading" | "ok" | "stale" | "unavailable" | "idle";
  label: string;
  data: Weather | null;
  fetchedMs: number | null;
  error: string | null;
  loading: boolean;
}

export interface LinkView {
  name: string;
  target: string;
  /** pending = ověření cíle na disku ještě běží (na pozadí v Rustu) */
  kind: "web" | "folder" | "file" | "app" | "invalid" | "pending";
  monogram: string;
  /** proč nejde otevřít (jen kind=invalid) */
  problem: string | null;
}

export interface PersonalView {
  weather: WeatherView;
  /** pořadí = `personal.links` v configu (index + target pro `openLink`) */
  links: LinkView[];
  /** obecné poznámky (bez projektu): připomínky na dnes / po termínu, pak nejnovější — nejvýš 8 */
  notes: NoteView[];
  /** kolik obecných poznámek se nevešlo */
  notesMore: number;
}

/** Synchronizace poznámek přes git (`notesync/`). `off` = vypnutá; `syncing` = právě běží (i první běh). */
export interface NotesSyncView {
  state: "off" | "ok" | "syncing" | "offline" | "neprihlaseno" | "konflikt" | "chyba";
  /** poslední úspěšný sync v tomto běhu Ledge (nic se nedopočítává) */
  lastOkMs: number | null;
  message: string | null;
  /** soubory v konfliktu */
  files: string[];
  /** kořen poznámek */
  root: string | null;
}

/** Kontrola nové verze (Rust `update.rs`, dotazy přes přihlášené `gh`). */
export type UpdateStatus =
  | { state: "off" }
  /** zapnuto, první kontrola ještě neproběhla (60 s po startu) */
  | { state: "neovereno" }
  | { state: "aktualni" }
  | { state: "dostupna"; verze: string; poznamky: string; publikovano: string }
  /** gh chybí / nepřihlášené / offline — jen v Nastavení, hlavička mlčí */
  | { state: "nelze_overit"; duvod: string };

export type UpdateView = UpdateStatus & {
  /** verze běžícího Ledge */
  current: string;
  repo: string;
  checking: boolean;
  checkedMs: number | null;
  installing: boolean;
  installError: string | null;
};

export type InstallOutcome = { outcome: "confirm"; running: string[] } | { outcome: "started" };

export interface Snapshot {
  mode: "prace" | "osobni";
  personalEnabled: boolean;
  view: View;
  cfgError: string | null;
  sections: Sections;
  claudeEnabled: boolean;
  projects: ProjectView[];
  procs: ProcView[];
  pending: Pending[];
  /** otázky Clauda (AskUserQuestion) — typ v bloku „questions“ na konci */
  questions: Ask[];
  sessions: SessionView[];
  hooks: HooksStatus;
  notesEnabled: boolean;
  /** kompaktní paleta: jen pole, výsledky až při psaní (`palette.compact`, `palette/compact.ts`) */
  paletteCompact: boolean;
  /** karta Poznámky: nehotové připomínky do konce dneška, pak nejnovější — nejvýš 8 */
  notes: NoteView[];
  /** kolik poznámek se do karty nevešlo (zbytek přes noteList) */
  notesMore: number;
  /** první (max 3) připomínky po termínu (peek) */
  reminders: NoteView[];
  remindersCount: number;
  /** kategorie z configu (pořadí záložek) a počty nehotových podle `kind` */
  noteKinds: NoteKind[];
  notesKindCounts: Record<string, number>;
  /** roste s každým přenačtením poznámek */
  notesRev: number;
  /** záložka Osobní */
  personal: PersonalView;
  notesSync: NotesSyncView;

  update: UpdateView;
}

/** Rozepsaná poznámka — leží na disku (`<root>\.draft\`), přežije zavření šuplíku i pád. */
export interface DraftMeta {
  text: string;
  project: string;
  projectTouched: boolean;
  remind: string;
  kind: string;
}
export interface Draft extends DraftMeta {
  /** PNG v base64 (bez prefixu data:) */
  images: string[];
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
/** Odpověď na jednu otázku: vybrané možnosti + vlastní text („Jiné…“). */
export interface QuestionReply { labels: string[]; other: string | null }

// --- actions ---
/** Rychlá akce v configu (`actions`, zrcadlo `actions.rs`). */
export type ActionKind = "url" | "file" | "folder" | "app" | "command";
export type ShowOn = "work" | "personal" | "both";
export interface ActionCfg {
  id: string;
  label: string;
  kind: ActionKind;
  target: string;
  args?: string[];
  cwd?: string;
  /** chybí / "auto" = favicon nebo ikona souboru; "monogram"; jinak cesta k obrázku */
  icon?: string;
  group?: string;
  confirm?: boolean;
  showOn?: ShowOn;
  shell?: "cmd" | "powershell";
  terminal?: boolean;
  timeoutS?: number;
}
/** Akce ve snapshotu (dlaždice, paleta). */
export interface ActionView {
  id: string;
  label: string;
  kind: ActionKind;
  target: string;
  group: string | null;
  showOn: ShowOn;
  /** klíč obsahu ikony; data URL přes `actionIcon` (jednou na klíč) */
  iconKey: string | null;
  monogram: string;
  color: string;
  needsConfirm: boolean;
  confirmText: string;
  problem: string | null;
  running: boolean;
}
export interface ActionOutcome {
  message: string;
  output: string | null;
  hide: boolean;
}
export interface ActionCheck {
  error: string | null;
  warning: string | null;
}
export interface Snapshot {
  /** rychlé akce (chybí jen ve starém jádře) */
  actions?: ActionView[];
}

// --- appearance ---
// Zrcadlo `AppearanceConfig` (src-tauri/src/appearance.rs) — `config.appearance`, událost `appearance`.
export type ThemeId = "dark" | "midnight" | "graphite" | "nord" | "light" | "contrast";
export type MotionStyle = "spring" | "smooth" | "snappy" | "none";
export type StripPosition = "center" | "left" | "right";
export interface Appearance {
  theme: ThemeId;
  /** `#rrggbb`, prázdné = výchozí akcent motivu */
  accent: string;
  density: "compact" | "normal" | "comfy";
  /** 0.85–1.25 */
  fontScale: number;
  radius: "sharp" | "normal" | "round";
  /** krytí pozadí šuplíku jako násobek krytí motivu (0.25–1) */
  glass: { opacity: number };
  /** speed 0.5–2 (2 = dvakrát rychleji) */
  motion: { style: MotionStyle; speed: number };
  /** color: prázdné = výchozí přechod, `accent`, nebo `#rrggbb` */
  strip: { style: "pill" | "line" | "dot"; size: "small" | "normal" | "large"; color: string; position: StripPosition };
  /** pořadí karet záložky Práce (`running`, `claude`, `repos`, `notes`) */
  cards: string[];
}

// --- onboarding ---
/** Repo z průvodce (Rust `onboarding::repos::FoundRepo`); `project` = hotový návrh pro config. */
export interface FoundRepo {
  name: string;
  path: string;
  remote: string | null;
  lastMs: number | null;
  kind: "dev" | "app" | "none";
  hint: string;
  project: Record<string, unknown> | null;
  already: boolean;
}
/** Aplikace ze Start menu (Rust `onboarding::apps::FoundApp`). */
export interface FoundApp {
  name: string;
  exe: string;
  args: string[];
  already: boolean;
}
export interface ClaudeInfo {
  exe: string | null;
  dir: boolean;
  version: string | null;
  installed: boolean;
}
export interface ThemeOption {
  value: string;
  label: string;
}
export interface OnboardingPatch {
  lang?: "cs" | "en";
  projects?: Record<string, unknown>[];
  links?: { label: string; url: string }[];
  theme?: string;
  done?: boolean;
}
export interface OnboardingApplied {
  added: string[];
  skipped: string[];
  linksAdded: number;
  linksTo: "actions" | "personal.links" | null;
  themeSaved: boolean | null;
}

// --- settings ---
/** Chyba validace configu s cestou k poli (`projects.2.run`); `field` null = nejde přiřadit. */
export interface FieldError { field: string | null; message: string }
export interface SectionDiff { key: string; changed: boolean; risk: string; detail: string }
export interface ImportPreview { cfg: any; sections: SectionDiff[] }
/** Návrh projektu ze složky (Rust `projectdetect.rs`) — UI ho upraví a uloží. */
export interface DetectedProject {
  id: string;
  name: string;
  kind: "dev" | "app";
  path: string;
  remote?: string;
  subdir?: string;
  run?: string;
  port?: number;
  exe?: string;
  notes: string[];
}
export interface RepoCandidate { path: string; name: string; remote?: string }
export interface AppInfo { version: string; configPath: string; configDir: string; logPath: string }
export type PickKind = "folder" | "file" | "exe" | "image" | "json";

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
export interface Snapshot {
  /** limity Claude (chybí jen ve starém jádře) */
  limits?: LimitsView;
}
