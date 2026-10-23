// Plán průvodce — čistá logika bez i18n, stavu a DOM (test `plan.test.mjs`): jaké kroky se ukážou, odhad
// času a řádky souhrnu v kroku Hotovo. Texty kroků jsou páry klíč + čeština (přeloží se při vykreslení).
export type StepId = "welcome" | "projects" | "apps" | "links" | "look" | "claude" | "done";
export interface StepDef {
  id: StepId;
  key: string;
  cs: string;
}
/** Všechny kroky v pořadí. „Claude“ je až na konci (před Hotovo) a ukáže se jen s nainstalovaným Claude Code. */
export const STEPS: readonly StepDef[] = [
  { id: "welcome", key: "onboarding.step.welcome", cs: "Vítej" },
  { id: "projects", key: "onboarding.step.projects", cs: "Projekty" },
  { id: "apps", key: "onboarding.step.apps", cs: "Aplikace" },
  { id: "links", key: "onboarding.step.links", cs: "Odkazy" },
  { id: "look", key: "onboarding.step.look", cs: "Vzhled" },
  { id: "claude", key: "onboarding.step.claude", cs: "Claude" },
  { id: "done", key: "onboarding.step.done", cs: "Hotovo" },
];

export interface StepFacts {
  /** motivy ze schématu už načtené */
  themesReady: boolean;
  themeCount: number;
  /** detekce Claude Code doběhla a našla ho */
  claudeInstalled: boolean;
}

/** Kroky, které se tomuhle uživateli ukážou: Vzhled jen když config motivy má (než se to ví, zůstává —
 *  jinak by počet kroků poskakoval), Claude jen když je Claude Code nainstalovaný. */
export function visibleSteps(f: StepFacts): StepDef[] {
  return STEPS.filter((s) => (s.id === "look" ? !f.themesReady || f.themeCount > 0 : s.id === "claude" ? f.claudeInstalled : true));
}

/** Kolik vteřin zabere jeden pracovní krok (výběr z nalezeného, ne psaní). */
export const SECONDS_PER_STEP = 30;

/** Poctivý odhad v minutách: pracovní kroky (bez Vítej a Hotovo) × `SECONDS_PER_STEP`, nahoru, aspoň 1. */
export function estimateMinutes(steps: readonly StepDef[]): number {
  const work = steps.filter((s) => s.id !== "welcome" && s.id !== "done").length;
  return Math.max(1, Math.ceil((work * SECONDS_PER_STEP) / 60));
}

export interface DoneSum {
  projects: string[];
  links: number;
  theme: string | null;
  themeSaved: boolean | null;
  hooks: boolean;
}
export type DoneRow =
  | { kind: "projects"; n: number; names: string }
  | { kind: "links"; n: number }
  | { kind: "theme"; name: string }
  | { kind: "hooks" };

/** Řádky souhrnu — jen to, co se opravdu stalo (nulové řádky nejsou). Motiv se ukáže jménem (`themeName`,
 *  `null` = jméno neznám → řádek není, surové id se nikdy neukazuje) a jen když se uložil. */
export function doneRows(s: DoneSum, hooked: boolean, themeName: string | null): DoneRow[] {
  const rows: DoneRow[] = [];
  if (s.projects.length) rows.push({ kind: "projects", n: s.projects.length, names: s.projects.slice(0, 6).join(", ") + (s.projects.length > 6 ? "…" : "") });
  if (s.links > 0) rows.push({ kind: "links", n: s.links });
  if (s.theme && s.themeSaved !== false && themeName) rows.push({ kind: "theme", name: themeName });
  if (s.hooks || hooked) rows.push({ kind: "hooks" });
  return rows;
}
