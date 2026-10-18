// Config se změnil mimo Nastavení (Claude přes MCP, přepínač MCP, druhé okno) a jádro poslalo událost
// `config-changed`. Politika (čistá funkce, test `merge.test.mjs`):
//  - beze změny proti základu → nic,
//  - bez neuložených úprav → načíst nový stav (draft = nový základ),
//  - neuložené úpravy v JINÝCH sekcích, než které se změnily venku → sloučit: venkovní sekce převzít,
//    rozpracované nechat (uživatel nic neztratí, Uložit pak nehlásí konflikt),
//  - neuložené úpravy ve STEJNÉ sekci → konflikt (lišta „Načíst nové / Přesto uložit“, nic se nepřepíše potichu).
export type External = { action: "same" } | { action: "load" } | { action: "merge"; draft: any } | { action: "conflict"; sections: string[] };

/** Sekce (klíče nejvyšší úrovně), ve kterých se `a` a `b` liší. */
export function changedSections(a: any, b: any, canon: (v: unknown) => string): string[] {
  const keys = new Set([...Object.keys(a ?? {}), ...Object.keys(b ?? {})]);
  return [...keys].filter((k) => canon(a?.[k]) !== canon(b?.[k])).sort();
}

export function external(base: any, draft: any, fresh: any, canon: (v: unknown) => string): External {
  const outside = changedSections(base, fresh, canon);
  if (!outside.length) return { action: "same" };
  const mine = changedSections(base, draft, canon);
  if (!mine.length) return { action: "load" };
  const both = outside.filter((k) => mine.includes(k));
  if (both.length) return { action: "conflict", sections: both };
  const next = structuredClone(draft);
  for (const k of outside) {
    if (fresh?.[k] === undefined) delete next[k];
    else next[k] = structuredClone(fresh[k]);
  }
  return { action: "merge", draft: next };
}
