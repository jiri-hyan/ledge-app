import type { TileLook } from "./projects";

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
