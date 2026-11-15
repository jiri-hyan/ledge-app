// Čisté pomocné funkce „Poslat do {název}“ (bez DOM a bez `t()` — testy `model.test.mjs`).

export type Assignee =
  | { kind: "unset" }
  | { kind: "resolved"; email: string; name: string }
  | { kind: "ambiguous"; query: string; names: string[] }
  | { kind: "unknown"; query: string };

export type Item = { line: number; title: string; assignee: Assignee; priority: number | null; sent: boolean; requestId: string };
export type Plan = { mode: "tasks" | "note"; hasTasks: boolean; noteTitle: string; noteRequestId: string; noteSent: boolean; items: Item[]; truncated: boolean };
export type Outcome = { line: number; ok: boolean; id: string | null; url: string | null; error: string | null; duplicate: boolean };

/** Řešitel se nepodařilo určit (víc shod / není v týmu) — nikdy se nehádá, uživatel to rozhodne. */
export const needsDecision = (i: Item): boolean => i.assignee.kind === "ambiguous" || i.assignee.kind === "unknown";

/** Které body jsou v náhledu předem zaškrtnuté: ještě neodeslané a s jasným (nebo žádným) řešitelem. */
export const defaultLines = (p: Plan): number[] => p.items.filter((i) => !i.sent && !needsDecision(i)).map((i) => i.line);

/** Priorita propojené aplikace 1–5 → nízká / střední / vysoká (`null` = poznámka ji neříká, nastaví aplikace). */
export const priorityBand = (p: number | null): "low" | "mid" | "high" | null => (p === null ? null : p <= 2 ? "low" : p <= 4 ? "mid" : "high");

export type Summary = { ok: number; failed: number; duplicate: number; error: string | null };

/** Souhrn odeslání: kolik prošlo, kolik selhalo, kolik už propojená aplikace znala (opakovaný klik), první chyba. */
export function summarize(results: Outcome[]): Summary {
  const failed = results.filter((r) => !r.ok);
  return { ok: results.length - failed.length, failed: failed.length, duplicate: results.filter((r) => r.ok && r.duplicate).length, error: failed[0]?.error ?? null };
}

/** Vybrané řádky po přepnutí zaškrtnutí. */
export const toggled = (sel: number[], line: number): number[] => (sel.includes(line) ? sel.filter((l) => l !== line) : [...sel, line].sort((a, b) => a - b));
