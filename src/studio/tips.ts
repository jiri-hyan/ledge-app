// Ledge Studio — „Dobré vědět“ a „Co je nového“ na domově (kolo 24, zpět po zrušených Doporučených).
// Čistá data a verzová logika bez DOM a `t()`; texty skládá `Home.tsx`. Tipy jsou jen funkce, které existují
// a nejsou vidět na první pohled; novinky se ukážou jen po přechodu z nižší verze a jdou zavřít.

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

/** Co s novinkami: `show` = přešel z nižší verze (karta, dokud ji nezavře), `first` = poprvé (jen si zapsat
 *  verzi, nic neukazovat), `none` = nic. `seen` = poslední verze, kterou uživatel viděl / zavřel. */
export type NewsState = "show" | "first" | "none";
export function newsState(seen: string | null | undefined, current: string | null | undefined): NewsState {
  if (!parseVersion(current)) return "none";
  if (seen == null || !parseVersion(seen)) return "first";
  return cmpVersion(seen, current) === -1 ? "show" : "none";
}

/** Tip = klávesa / znak, id textu a kam případně skočit (kotva volby ve Studiu). */
export type Tip = { id: "palette" | "edit" | "card" | "search"; k: string; go?: string };
export const TIPS: readonly Tip[] = [
  { id: "palette", k: "Ctrl+K", go: "f-palette.compact" },
  { id: "edit", k: "✎" },
  { id: "card", k: "⋯" },
  { id: "search", k: "Ctrl+F" },
];

/** Novinky, které opravdu existují (cíl `go` = volba, kde je vidět). */
export type News = { id: "studio" | "motion" | "media"; k: string; go?: string };
export const NEWS: readonly News[] = [
  { id: "studio", k: "⚙" },
  { id: "motion", k: "↧", go: "f-appearance.motion" },
  { id: "media", k: "♪", go: "f-media.variant" },
];
