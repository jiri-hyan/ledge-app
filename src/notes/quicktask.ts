// Rychlá poznámka → úkol: čisté rozhodnutí, jestli se napsaný text uloží jako řádek `- [ ] … ⏰ … !` (výsledek
// `note_quick_parse`), nebo beze změny jako prostý text. Úkol jen pro jednořádkový text, ve kterém parser našel čas
// nebo prioritu; cokoli jiného (víc řádků, už hotový seznam, nic nenalezeno) = stará cesta. Test `quicktask.test.mjs`.
import type { QuickTask } from "../types/notes";

/** Nejdelší text, který se vůbec posílá parseru (delší je poznámka, ne věta). */
export const QUICK_MAX = 200;

/** Má smysl se parseru ptát? (jednořádkový, neprázdný, ne už seznam, v mezích) */
export function worthParsing(text: string): boolean {
  const s = text.trim();
  return s !== "" && s.length <= QUICK_MAX && !/[\r\n]/.test(s) && !/^(?:[-*+]|\d+[.)])\s/.test(s);
}

/** Řádek úkolu k uložení, nebo null = uložit text jak je. */
export function quickTaskLine(text: string, q: QuickTask | null | undefined): string | null {
  if (!q || !worthParsing(text)) return null;
  if (!q.whenIso && !(q.priority > 0)) return null;
  const line = (q.line ?? "").trim();
  return /^- \[ \] \S/.test(line) && !/[\r\n]/.test(line) ? line : null;
}

/** Text chipu pod polem: „⏰ 2026-11-14 09:00 · !“; null = nic k ukázání. */
export function quickChip(text: string, q: QuickTask | null | undefined): { when: string | null; prio: 0 | 1 | 2 } | null {
  if (quickTaskLine(text, q) == null) return null;
  return { when: q!.whenIso ?? null, prio: q!.priority };
}
