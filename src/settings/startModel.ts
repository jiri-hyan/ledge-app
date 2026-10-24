// Čistá logika stránky Doporučené (bez DOM a Solid): kdy ukázat „Co je nového“ a starší tipy. Nový uživatel
// (Ledge poprvé v tomhle okně) je nezajímá; jen ten, kdo přešel z nižší verze. Poslední viděnou verzi
// drží stránka v localStorage (`Recommended.tsx`) — config pro ni pole nemá.

/** „0.4.1“ → [0, 4, 1]; cokoli, co není tečkované číslo (i prázdné), = null. Předpona `v` se ignoruje. */
export function parseVersion(v: string | null | undefined): number[] | null {
  const m = /^v?(\d+(?:\.\d+){0,3})$/.exec((v ?? "").trim());
  return m ? m[1].split(".").map(Number) : null;
}

/** Porovnání verzí po složkách (chybějící složka = 0); neplatná verze = null. */
export function cmpVersion(a: string | null | undefined, b: string | null | undefined): number | null {
  const x = parseVersion(a);
  const y = parseVersion(b);
  if (!x || !y) return null;
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d) return Math.sign(d);
  }
  return 0;
}

/** Přešel uživatel z nižší verze? `seen` = poslední verze, se kterou panel běžel (null = poprvé → nový uživatel). */
export function upgradedFrom(seen: string | null | undefined, current: string | null | undefined): boolean {
  return cmpVersion(seen, current) === -1;
}
