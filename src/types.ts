// Typy snapshotu a IPC (zrcadlo serde struktur v `src-tauri/src`) — jen tvary dat, žádná logika.
// Volání jádra jsou v `api.ts`, který tyto typy zároveň re-exportuje (`import type { … } from "./api"`).
import type { SectionKey } from "./cards/sections";
import type { Layout } from "./layout/model";

/** `compact` / `island` = náhled po najetí (`drawer.hover`, `src/hover/*`, Rust `hovermode.rs`) */
export type View = "hidden" | "peek" | "full" | "compact" | "island";

export interface ViewMsg {
  view: View;
  morph: boolean;
  fromW: number;
  fromH: number;
  toW: number;
  toH: number;
  /** strop výšky obsahu na monitoru (logické px); nad ním se posouvá uvnitř stránky */
  maxH: number;
  /** náhled po najetí: po kolika ms setrvání rozbalit celý šuplík (`drawer.hoverExpandMs`), 0 = jen klikem; chybí = staré jádro */
  expandMs?: number;
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
  /** port z configu (co a odkud — tooltip a „Upravit…“ na dlaždici) */
  port?: number | null;
  remote?: string | null;
  /** aplikace: exe, appName nebo aumid (Microsoft Store) */
  app?: string | null;
  /** odkud poběží (`resolve::Place`); chybí (starší jádro) = podle `pathOk` */
  place?: Place;
  /** vlastní vzhled dlaždice (`tilelook.rs`); chybí = automaticky */
  look?: TileLook;
}

/** Odkud projekt poběží: jasné · `path` neexistuje · víc složek s repem (vybrat) · nic na disku. */
export type Place = { kind: "ok" } | { kind: "missing"; path: string } | { kind: "choose"; count: number } | { kind: "none" };
/** Složka s repem projektu pro výběr (`project_folders`, nejčerstvější commit první). */
export interface FolderCandidate { path: string; lastCommit: number | null; worktree: boolean }
export interface FolderCandidates { current: string | null; items: FolderCandidate[] }
/** Port projektu drží proces, který nespustil Ledge (`port_holders`). */
export interface PortHolder { id: string; port: number; pid: number; process: string }

// --- vzhled zástupců (zrcadlo `src-tauri/src/tilelook.rs`, logika `src/tilelook/model.ts`) ---
/** Ikona místo automatické: emoji, systémová ikona (kód Segoe Fluent Icons) nebo vlastní obrázek (klíč v cache). */
export type LookIcon = { kind: "emoji" | "glyph" | "image"; value: string };
export type TileShape = "rounded" | "circle" | "square" | "glass";
export interface TileLook {
  icon?: LookIcon;
  /** barva podkladu `#rrggbb` */
  color?: string;
  shape?: TileShape;
  /** vlastní popisek */
  label?: string;
  hideLabel?: boolean;
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
  /** odkud a co běží (složka, příkaz s portem / exe) */
  cwd?: string | null;
  cmd?: string | null;
  /** poslední řádky výstupu při selhání */
  tail?: string[];
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

/** `drawer.sections` — viditelnost karet a jejich částí, ODVOZENÁ v Rustu z `layout` (seznam v `cards/sections.ts`). */
export type Sections = Record<SectionKey, boolean>;

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

/** Volby lístku na plochu (Rust `pins::PinPrefs`). `bg` 0–100 %, `text` 50–100 % neprůhlednost. */
export interface PinPrefs { onTop: boolean; locked: boolean; through: boolean; bg: number; text: number; collapsed: boolean }
export interface PinsView { open: { path: string; through: boolean }[]; key: string }
/** Start okna lístku (`pin_init`): poznámka, volby, zkratka průchozího režimu (a jestli je zaregistrovaná). */
export interface PinInit { path: string; prefs: PinPrefs; key: string; keyOk: boolean; hintSeen: boolean; lang: "cs" | "en"; fresh: boolean }

export interface NoteQuery { kind?: string | null; project?: string | null; includeDone?: boolean; /** archivované místo aktivních */ archived?: boolean; limit?: number }
/** Výsledek smazání poznámky: id pro „Zpět“ a titulek pro lištu. */
export interface Deleted { id: string; title: string }
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

/** Právě hraje (Rust `media::MediaView`, GSMTC). */
export interface MediaView {
  title: string;
  artist: string;
  /** čitelný název aplikace; "" = neznámá */
  app: string;
  status: "playing" | "paused" | "stopped" | "changing";
  canToggle: boolean;
  canPrev: boolean;
  canNext: boolean;
  /** klíč obalu pro `api.mediaCover`; null = bez obalu */
  coverKey: string | null;
}

/** Hlasitost výchozího výstupu (Rust `volume::VolumeView`). */
export interface VolumeView {
  /** 0–100 */
  level: number;
  muted: boolean;
  /** název zařízení; "" = nevíme */
  device: string;
}

/** Baterie (Rust `battery::BatteryView`); jen stroj s baterií. */
export interface BatteryView {
  percent: number;
  charging: boolean;
  plugged: boolean;
  /** odhad Windows v sekundách, jen na baterii; null = Windows neví */
  remainingS: number | null;
}

export interface PersonalView {
  weather: WeatherView;
  /** pořadí = `personal.links` v configu (index + target pro `openLink`) */
  links: LinkView[];
  /** obecné poznámky (bez projektu): připomínky na dnes / po termínu, pak nejnovější — nejvýš 8 */
  notes: NoteView[];
  /** kolik obecných poznámek se nevešlo */
  notesMore: number;
  /** právě hraje; null = nic nehraje / karta vypnutá / odběr ještě neběží */
  media: MediaView | null;
  /** null = bez výstupního zařízení / vypnuto */
  volume: VolumeView | null;
  /** null = stroj bez baterie / vypnuto */
  battery: BatteryView | null;
  /** karta Kalendář (Rust `calendar::live::view`); starý snapshot bez pole = nic */
  calendar?: CalendarView;
  /** karta Pošta — účet Microsoft (Rust `accounts::mail::view`); starý snapshot bez pole = nic */
  mail?: MailView;
}

/** Zpráva v kartě Pošta (jen hlavička — tělo Ledge nečte). */
export interface MailItem {
  subject: string;
  from: string;
  receivedMs: number;
  unread: boolean;
  /** odkaz na zprávu v Outlooku na webu (https) */
  link: string | null;
}

/** Karta Pošta (Rust `accounts::mail::MailView`). */
export interface MailView {
  status: "off" | "unset" | "idle" | "loading" | "ok" | "stale" | "unavailable";
  account: string;
  unread: number | null;
  items: MailItem[];
  fetchedMs: number | null;
  error: string | null;
  loading: boolean;
  /** kam vede „Otevřít Outlook“ */
  web: string;
}

/** Výskyt události na kartě Kalendář (Rust `calendar::view::Item`). */
export interface CalendarItem {
  /** ms epochy; u celodenní půlnoc data jako UTC — čas se neukazuje */
  start: number;
  end: number;
  allDay: boolean;
  title: string;
  location: string;
  /** index v `CalendarView.calendars` (= `personal.calendars`) */
  cal: number;
  ongoing: boolean;
  /** pásmo události Ledge nezná → čas brán jako místní */
  tzGuess: boolean;
  /** odkaz Připojit (Rust `calendar::meeting::Join`); null / starý snapshot bez pole = žádný */
  join?: CalendarJoin | null;
}

/** Odkaz na schůzku (ověřený Rustem: jen https známé služby). */
export interface CalendarJoin {
  provider: "teams" | "meet" | "zoom" | "webex" | "jitsi";
  url: string;
}

/** Zdroj kalendáře (`personal.calendars[].source`, Rust `calendar::source::SourceKind`). */
export type CalendarSource = "ics" | "outlook-local" | "graph" | "google";

export interface CalendarDay {
  kind: "today" | "tomorrow" | "later";
  /** místní datum YYYY-MM-DD */
  date: string;
  items: CalendarItem[];
}

/** Jeden kalendář z configu — adresa nikdy celá, jen doména. */
export interface CalendarInfo {
  name: string;
  color: string | null;
  domain: string;
  /** starý snapshot bez pole = ics */
  source?: CalendarSource;
  /** `outlook-closed` = Outlook v PC je zavřený (karta nabídne Načíst z Outlooku) */
  hint?: "outlook-closed" | null;
  enabled: boolean;
  status: "off" | "invalid" | "ok" | "stale" | "loading" | "unavailable" | "idle";
  error: string | null;
  fetchedMs: number | null;
  events: number;
  /** zdroj z přihlášeného účtu (Nastavení → Účty); chybí = odkaz ICS z `personal.calendars` */
  account?: "microsoft" | "google";
}

export interface CalendarView {
  status: "off" | "unset" | "ok" | "stale" | "loading" | "unavailable" | "idle";
  days: CalendarDay[];
  more: number;
  calendars: CalendarInfo[];
  loading: boolean;
}

/** `calendar_outlook_probe`: klasický Outlook nainstalovaný / běží. */
export interface OutlookProbe {
  installed: boolean;
  running: boolean;
}

/** Výsledek `calendar_test` (Nastavení: přidání / Otestovat). */
export interface CalendarTest {
  events: number;
  upcoming: number;
  /** název z feedu; "" = feed ho nemá */
  name: string;
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
  /** id záložky, která je vidět (`layout.pages[].id`; vestavěné `prace` / `osobni`) */
  mode: string;
  /** zastaralé — vždy true, záložky řídí `layout` */
  personalEnabled: boolean;
  view: View;
  cfgError: string | null;
  sections: Sections;
  /** záložky a karty na nich (`config.layout`, `layout/model.ts`) */
  layout: Layout;
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
  /** kolik poznámek je v archivu (`_archiv`) */
  notesArchived: number;
  /** lístky na plochu: připnuté poznámky (a jestli průchozí) a zkratka průchozího režimu (Rust `pins::win::view`) */
  pins?: PinsView;
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
export type ActionKind = "url" | "file" | "folder" | "app" | "command" | "web";
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
  /** jen `web`: velikost okna v logických px (320–3840 × 240–2160, výchozí 480 × 720) */
  width?: number;
  height?: number;
  /** vlastní vzhled dlaždice (`tilelook.rs`) */
  look?: TileLook;
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
  /** proč se ptá (skript: poprvé / po změně souboru / `@ledge.confirm`) */
  confirmNote?: string | null;
  /** položka je skript ze složky skriptů (`ScriptView`), ne akce z configu */
  script?: boolean;
  /** vlastní vzhled dlaždice (`tilelook.rs`); chybí = automaticky */
  look?: TileLook;
}
/** Skript ze složky skriptů (`scripts/mod.rs` → `ScriptView`): tvar akce + pár polí navíc. */
export interface ScriptView extends ActionView {
  script: true;
  description: string | null;
  mode: "hidden" | "output" | "terminal";
  lang: "powershell" | "cmd" | "python" | "node";
}
/** `scripts_list`: co je ve složce (paleta, Nastavení). */
export interface ScriptsListing {
  enabled: boolean;
  dir: string;
  exists: boolean;
  scripts: ScriptView[];
  /** souborů nad strop 200 */
  skipped: number;
  error: string | null;
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
  /** skripty jako dlaždice karty (jen se `scripts.inActions`; jinak chybí) */
  scripts?: ScriptView[];
  /** stránky Nastavení Windows v paletě (`palette.winSettings`; chybí = staré jádro → zapnuto) */
  paletteWinSettings?: boolean;
}

// --- appearance ---
// Zrcadlo `AppearanceConfig` (src-tauri/src/appearance.rs) — `config.appearance`, událost `appearance`.
export type ThemeId = "dark" | "midnight" | "graphite" | "nord" | "light" | "contrast" | "terminal" | "paper" | "neon" | "brutal" | "pastel";
export type MotionStyle = "spring" | "smooth" | "snappy" | "bounce" | "fade" | "notch" | "none";
export type StripPosition = "center" | "left" | "right";
export type StripStyle = "pill" | "line" | "dot" | "term" | "tag" | "glow" | "block" | "drop";
/** styl zavřeného Ledge (`strip.look`); seznam s názvy je `strip/looks.ts` */
export type { StripLookId } from "./strip/looks";
/** složky design stylu (vlna 6, `appearance/look.rs`); tabulka stylů je `theme/styles.ts` */
export type FontLook = "system" | "mono" | "serif" | "grotesk";
export type Material = "solid" | "glass" | "paper" | "glow" | "hard" | "soft" | "frame";
export type LayoutId = "classic" | "float" | "band" | "sheet" | "panorama" | "mosaic" | "tiles";
export type WidthPreset = "custom" | "narrow" | "medium" | "wide" | "full";
export interface Appearance {
  /** design styl (`classic` = vzhled před vlnou 6); hodnoty stylu jsou v polích níž */
  style: string;
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
  strip: { style: StripStyle; size: "small" | "normal" | "large"; color: string; position: StripPosition };
  font: FontLook;
  material: Material;
  layout: LayoutId;
  /** předvolba šířky; `custom` = `drawer.width` v px */
  width: WidthPreset;
}

// Co se stane po najetí myší (`drawer.hover`, data `src/hover/modes.ts`); chybí v configu = `full`.
// `StripLookId` (vzhled zavřeného Ledge) je re-export z `src/strip/looks.ts` výše.
export type { HoverModeId } from "./hover/modes";

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

// --- costs ---
// Zrcadlo `costslive::CostsView` (Rust) — útrata Claude, odhad v cenách API (typy v `costs/model.ts`).
export type { CostsView } from "./costs/model";
export interface Snapshot {
  /** útrata Claude (chybí jen ve starém jádře) */
  costs?: import("./costs/model").CostsView;
}
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

// --- timer ---
// Zrcadlo `timer::TimerView` (Rust) — odpočet a stopky jako časy (typy a logika v `timer/model.ts`).
export type { TimerOp, TimerState, TimerView } from "./timer/model";
export interface Snapshot {
  /** časovač (chybí jen ve starém jádře) */
  timer?: import("./timer/model").TimerView;
}

// --- media: průběh skladby a vzhled karty Právě hraje ---
// Zrcadlo Rust `mediatime::Timeline` / `mediaconf::MediaConfig` (typy a logika v `media/timeline.ts`).
export type { MediaLook, MediaRepeat, MediaStripTime, MediaTimeline, MediaVariant } from "./media/timeline";
export interface MediaView {
  /** průběh; null = zdroj timeline nedává (lišta se neukáže); chybí jen ve starém jádře */
  timeline?: import("./media/timeline").MediaTimeline | null;
  /** náhodně; null = zdroj nepodporuje (tlačítko se neukáže) */
  shuffle?: boolean | null;
  /** opakovat; null = zdroj nepodporuje */
  repeat?: import("./media/timeline").MediaRepeat | null;
}
export interface PersonalView {
  /** sekce `media` v configu (vzhled karty); chybí jen ve starém jádře = výchozí */
  mediaLook?: import("./media/timeline").MediaLook;
}
/** Akce `api.mediaControl` (Rust `media::Control`). */
export type MediaAction = "toggle" | "next" | "prev" | { seek: number } | { shuffle: boolean } | { repeat: import("./media/timeline").MediaRepeat };

// --- widgets ---
/** Barva z výstupu widgetu (předvolba, Rust `widgets::output::Tone`). */
export type WidgetTone = "accent" | "text" | "muted" | "violet" | "blue" | "teal" | "green" | "amber" | "red";
/** Akce položky widgetu — jen tyhle (Rust `widgets::output::Act`). */
export type WidgetAct = { type: "open"; path: string } | { type: "copy"; text: string } | { type: "run"; script: string; arg: string | null };
export interface WidgetItem {
  label: string;
  value?: string;
  icon?: string;
  color?: WidgetTone;
  url?: string;
  action?: WidgetAct;
  /** oddělovač */
  sep?: boolean;
}
/** Co widget vypsal (Rust `widgets::output::WidgetData`). */
export interface WidgetData {
  title: string | null;
  text: string | null;
  items: WidgetItem[];
  badge: string | null;
  /** 0–1 */
  progress: number | null;
  strip: string | null;
  color: WidgetTone | null;
  /** co z výstupu nešlo použít */
  notes?: string[];
}
export type WidgetStatus = "off" | "missing" | "problem" | "consent" | "loading" | "error" | "ok" | "idle";
export type WidgetKind = "powershell" | "cmd" | "python" | "node" | "exe";
/** Widget pro UI (Rust `widgets::live::WidgetView`). */
export interface WidgetView {
  /** `widget:<soubor>` — id karty i položky proužku */
  id: string;
  file: string;
  title: string;
  icon: string | null;
  description: string | null;
  show: "card" | "strip" | "both";
  intervalS: number;
  kind: WidgetKind | null;
  status: WidgetStatus;
  problem: string | null;
  consent: { changed: boolean; note: string; confirmText: string; preview: string } | null;
  data: WidgetData | null;
  error: string | null;
  updatedMs: number | null;
  running: boolean;
  hasOutput: boolean;
  /** je položkou proužku (`strip.items`) */
  inStrip: boolean;
}
/** `widgets_list`: co je ve složce widgetů (katalog, Nastavení). */
export interface WidgetsListing {
  enabled: boolean;
  dir: string;
  exists: boolean;
  widgets: WidgetView[];
  skipped: number;
  error: string | null;
}
/** Příklad z galerie (Rust `widgets::templates::TemplateView`). */
export interface WidgetTemplate {
  file: string;
  title: string;
  description: string | null;
  icon: string | null;
  kind: WidgetKind;
  /** kam sahá po síti (null = nikam) */
  net: string | null;
  installed: boolean;
  body: string;
}
export interface Snapshot {
  /** widgety umístěné na záložce nebo v proužku (chybí = žádné) */
  widgets?: WidgetView[];
}
