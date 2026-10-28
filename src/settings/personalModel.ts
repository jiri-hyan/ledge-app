// Pořadí skupin stránky Obsah karet (dřív Osobní; bez DOM a Solid): u každé karty její volby pohromadě
// (kolo 21). Nahoře karty, které jsou v rozložení šuplíku (`layout.pages[].cards`) — otevřené; pod nimi
// ostatní karty a nakonec funkce bez vlastní karty — sbalené (ne „Pokročilé“: vidět jsou vždy, i v režimu
// Základní). Hledání najde volby ve všech skupinách a skupinu rozbalí. Vypínač karty na stránce
// (`setCardShown`) jen skládá existující `addCard` / `removeCard` rozložení.

import { PERSONAL, addCard, normalizeLayout, pageHas, removeCard, type CardId, type Layout } from "../layout/model.ts";

/** Skupiny stránky a karta, ke které patří (`layout.rs` `CARDS`); bez karty = funkce bez vlastní karty. */
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

/** Rozdělení skupin (pořadí jako v `PERSONAL_SECTIONS`): `shown` = karta je v rozložení, `more` = karta,
 *  která teď v šuplíku není, `helpers` = funkce bez vlastní karty. */
export function splitPersonal(cardIds: readonly string[]): { shown: PersonalSectionId[]; more: PersonalSectionId[]; helpers: PersonalSectionId[] } {
  const on = new Set(cardIds);
  const shown: PersonalSectionId[] = [];
  const more: PersonalSectionId[] = [];
  const helpers: PersonalSectionId[] = [];
  for (const s of PERSONAL_SECTIONS) (!s.card ? helpers : on.has(s.card) ? shown : more).push(s.id);
  return { shown, more, helpers };
}

/** Zapne / vypne kartu v rozložení: vypnutí ji odebere ze všech záložek (zůstane v katalogu), zapnutí ji
 *  přidá na konec záložky Osobní, a není-li, na první záložku. Už zapnutá / vypnutá = beze změny. */
export function setCardShown(raw: unknown, card: string, shown: boolean): Layout {
  const l = normalizeLayout(raw);
  const id = card as CardId;
  if (!shown) return l.pages.reduce((acc, p) => (pageHas(p, id) ? removeCard(acc, p.id, id).layout : acc), l);
  if (l.pages.some((p) => pageHas(p, id))) return l;
  const target = l.pages.find((p) => p.id === PERSONAL) ?? l.pages[0];
  return target ? addCard(l, target.id, id) : l;
}
