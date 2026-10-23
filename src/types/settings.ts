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
