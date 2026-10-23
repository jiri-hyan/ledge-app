// Pořadí sekcí panelu Osobní (bez DOM a Solid): nejdřív sekce karet, které jsou v rozložení šuplíku
// (`layout.pages[].cards`), ostatní za „Další sekce…“. Hledání najde volby ve všech sekcích vždy.

/** Sekce panelu a karta, ke které patří (`layout.rs` `CARDS`); bez karty = pomocná sekce bez vlastní karty. */
export const PERSONAL_SECTIONS = [
  { id: "weather", card: "weather" },
  { id: "calendars", card: "calendar" },
  { id: "media", card: "media" },
  { id: "timer", card: "timer" },
  { id: "github", card: "github" },
  { id: "system", card: "system" },
  { id: "suggest", card: null },
  { id: "clipboard", card: null },
] as const;

export type PersonalSectionId = (typeof PERSONAL_SECTIONS)[number]["id"];

/** Id karet na všech záložkách rozložení (chybějící nebo rozbitá struktura = žádné). */
export function layoutCardIds(layout: unknown): string[] {
  const pages = (layout as { pages?: unknown } | null | undefined)?.pages;
  if (!Array.isArray(pages)) return [];
  const out: string[] = [];
  for (const p of pages) {
    const cards = (p as { cards?: unknown } | null)?.cards;
    if (Array.isArray(cards)) for (const c of cards) if (typeof (c as { id?: unknown })?.id === "string") out.push((c as { id: string }).id);
  }
  return out;
}

/** Rozdělení sekcí: `shown` = jejich karta je v rozložení (pořadí jako v `PERSONAL_SECTIONS`), `rest` = ostatní. */
export function splitPersonal(cardIds: readonly string[]): { shown: PersonalSectionId[]; rest: PersonalSectionId[] } {
  const on = new Set(cardIds);
  const shown: PersonalSectionId[] = [];
  const rest: PersonalSectionId[] = [];
  for (const s of PERSONAL_SECTIONS) (s.card && on.has(s.card) ? shown : rest).push(s.id);
  return { shown, rest };
}
