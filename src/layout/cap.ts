// Strop řádků karty bez DOM (test `cap.test.mjs`): karta, která může neomezeně růst (Claude session,
// Repa, Běží), ukáže nejvýš `limit` řádků (`layout…cards[].limit`, výchozí 4, 0 = bez stropu) a pod nimi
// „Zobrazit další (k)“ ↔ „Méně“. Připnuté řádky (session, které čekají na tebe) se nikdy neschovají.
// Rozbalení si šuplík pamatuje zvlášť pro každou záložku (`capKey`); komponenta `ShowMore.tsx`.

/** Rozdělení na viditelné a schované (pořadí zůstává). */
export function capList<T>(items: readonly T[], limit: number, open: boolean, pinned?: (x: T) => boolean): { shown: T[]; hidden: T[] } {
  if (open || limit <= 0 || items.length <= limit) return { shown: [...items], hidden: [] };
  const pins = pinned ? items.filter(pinned).length : 0;
  let free = Math.max(0, limit - pins);
  const shown: T[] = [];
  const hidden: T[] = [];
  for (const x of items) {
    if (pinned?.(x)) shown.push(x);
    else if (free > 0) {
      shown.push(x);
      free--;
    } else hidden.push(x);
  }
  return { shown, hidden };
}

/** Klíč rozbalení: záložka + karta (na Práci rozbalené, na Osobní sbalené). */
export const capKey = (page: string, card: string): string => `${page}:${card}`;
