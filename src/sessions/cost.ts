// Cena session (kolo 34, H6): odhad v cenách API od startu session, z `Snapshot.costs.summary.sessions`
// (Rust `usage/sess.rs`, počítáno inkrementálně při čtení transcriptů). Čisté, bez DOM a `t()`.
// Definice: součet odpovědí session včetně jejích subagentů uvnitř okna indexu (30 dní); starší odpovědi index
// zahazuje (`partial`) a model bez ceny v ceníku (`unpriced`) se nenaceňuje → v obou případech jde o dolní odhad „≥“.
// Co nevíme (session bez záznamu, žádný naceněný token), neukazujeme — žádné nuly pro efekt.
// Tempo (USD/h) záměrně chybí: transcript nezná dobu aktivní práce, jen časy odpovědí → průměr přes prostoje by lhal.

/** Rust `usage::sess::SessCost`. */
export interface SessCost {
  id: string;
  tokens: number;
  /** null = žádný token nemá cenu */
  cost: number | null;
  unpriced: number;
  requests: number;
  firstMs: number;
  lastMs: number;
  /** soubor má i odpovědi starší než okno → dolní odhad */
  partial: boolean;
}

export interface SessCostView {
  cost: number;
  /** částka je dolní odhad („≥“) */
  lower: boolean;
  partial: boolean;
  unpriced: number;
  requests: number;
  tokens: number;
  firstMs: number;
  lastMs: number;
}

/** Cena session podle id; chybí-li záznam nebo cena, null (UI nic nekreslí). */
export function sessionCost(list: readonly SessCost[] | undefined, id: string): SessCostView | null {
  const s = list?.find((x) => x.id === id);
  if (!s || s.cost == null || !Number.isFinite(s.cost)) return null;
  const unpriced = Math.max(0, s.unpriced);
  return { cost: Math.max(0, s.cost), lower: unpriced > 0 || s.partial, partial: s.partial, unpriced, requests: s.requests, tokens: s.tokens, firstMs: s.firstMs, lastMs: s.lastMs };
}

/** Text částky: „≥ 1,20 $“ u dolního odhadu, jinak „1,20 $“; `fmt` formátuje číslo s jednotkou. */
export function sessionCostText(c: SessCostView, fmt: (v: number) => string): string {
  return `${c.lower ? "≥ " : ""}${fmt(c.cost)}`;
}
