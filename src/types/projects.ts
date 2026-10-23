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
  /** ztišené upozornění na neuloženou / nepushnutou práci v proužku a v hlavičce šuplíku */
  muteDirty?: boolean;
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
