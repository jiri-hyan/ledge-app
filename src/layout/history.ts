// Zpět v režimu úprav šuplíku bez DOM (test `history.test.mjs`): zásobník předchozích rozložení — každá
// změna (velikost, výška, přesun, odebrání, záložky, Obnovit výchozí) jde vrátit tlačítkem Zpět / Ctrl+Z.
// Změna bez efektu se nezapíše (Zpět by jinak „nic nedělalo“).
import { clone, defaultLayout, sameLayout, type Layout } from "./model.ts";

export const MAX_UNDO = 30;

/** Zásobník po změně `prev` → `next` (beze změny = stejný zásobník). */
export function pushUndo(stack: readonly Layout[], prev: Layout, next: Layout): Layout[] {
  if (sameLayout(prev, next)) return [...stack];
  return [...stack, clone(prev)].slice(-MAX_UNDO);
}

/** Poslední rozložení ze zásobníku (prázdný = `layout: null`). */
export function popUndo(stack: readonly Layout[]): { stack: Layout[]; layout: Layout | null } {
  if (!stack.length) return { stack: [], layout: null };
  return { stack: stack.slice(0, -1), layout: stack[stack.length - 1] };
}

/** Obnovit výchozí: výchozí záložky a karty; značky jednorázových úprav zůstanou (jinak by se po
 *  uložení spustily znovu). */
export function resetLayout(cur: Layout): Layout {
  const out = defaultLayout();
  const upgrades = [...new Set([...(out.upgrades ?? []), ...(cur.upgrades ?? [])])];
  if (upgrades.length) out.upgrades = upgrades;
  return out;
}
