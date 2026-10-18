// Mechanika mřížky `.deck` bez DOM (test `size.test.mjs`): kolik sloupců zabere karta, „bez děr“
// (rozpětí karet v řadě), a přímá změna velikosti tažením za okraj v režimu úprav (šířka se přichytí na
// sloupce, výška na řádky `--deck-row`). Počet sloupců určuje styl (`--deck-cols`, vlna 6 F) — tady jen
// počty. Jen typy z modelu, žádné importy za běhu.
//
// Bez děr: karty jdou za sebou v pořadí záložky; řada, do které se další karta nevejde, se uzavře a její
// volné sloupce dostanou karty té řady (po jednom od poslední). Pořadí karet se nemění (žádné
// přeskládání „dense“ — v úpravách by karta skákala jinam, než ji uživatel položil), jen se roztáhnou.
import type { CardHeight, CardSize } from "./model";

const ORDER: readonly CardSize[] = ["s", "m", "l", "xl"];

/** Sloupce karty velikosti `size` při `cols` sloupcích (zrcadlo `spans` v `theme/looks.ts`). */
export const spanOf = (size: CardSize, cols: number): number => (size === "s" ? 1 : size === "m" ? Math.min(2, cols) : size === "l" ? Math.min(3, cols) : cols);

/** Počet sloupců z hodnoty `--deck-cols` (prázdné / nesmysl = výchozí 4). */
export function deckCols(raw: string | null | undefined): number {
  const n = Number.parseInt(String(raw ?? "").trim(), 10);
  return Number.isFinite(n) && n >= 1 ? Math.min(n, 12) : 4;
}

/** Rozpětí karet tak, aby žádná řada nekončila prázdným místem. */
export function packSpans(sizes: readonly CardSize[], cols: number): number[] {
  const out = sizes.map((s) => Math.max(1, Math.min(cols, spanOf(s, cols))));
  let row: number[] = [];
  let used = 0;
  const flush = () => {
    let free = cols - used;
    for (let k = row.length - 1; free > 0 && row.length; k = (k - 1 + row.length) % row.length) {
      out[row[k]]++;
      free--;
    }
    row = [];
    used = 0;
  };
  out.forEach((span, i) => {
    if (used + span > cols) flush();
    row.push(i);
    used += span;
  });
  if (row.length) flush();
  return out;
}

/** Velikost pro rozpětí `span` (nejbližší dostupná; celá řada = xl, při shodě menší). */
export function sizeForSpan(span: number, cols: number): CardSize {
  if (span >= cols) return "xl";
  let best: CardSize = "s";
  for (const s of ORDER) if (s !== "xl" && Math.abs(spanOf(s, cols) - span) < Math.abs(spanOf(best, cols) - span)) best = s;
  return best;
}

/** Výška v px pro `h` řádků (řádek + mezery mezi nimi). */
export const heightPx = (h: number, row: number, gap: number): number => h * row + (h - 1) * gap;

/** Řádky pro výšku v px (zaokrouhleně, 1–4). */
export function rowsForPx(px: number, row: number, gap: number): CardHeight {
  const n = Math.round((px + gap) / Math.max(1, row + gap));
  return Math.max(1, Math.min(4, n)) as CardHeight;
}

/** Rozměry mřížky v okamžiku, kdy uživatel chytí okraj karty (px z `getBoundingClientRect`). */
export type DeckMetrics = { cols: number; colW: number; gap: number; row: number; left: number; top: number };
export type Edge = "e" | "s" | "se";
export type Dim = { size: CardSize; h: CardHeight | undefined };

/** Cílová velikost při tažení za pravý okraj (`e` = šířka), spodní (`s` = výška) nebo roh (`se` = obojí). */
export function resizeTo(m: DeckMetrics, edge: Edge, start: Dim, px: number, py: number): Dim {
  let size = start.size;
  let h = start.h;
  if (edge !== "s") {
    const span = Math.round((px - m.left + m.gap) / Math.max(1, m.colW + m.gap));
    size = sizeForSpan(Math.max(1, Math.min(m.cols, span)), m.cols);
  }
  if (edge !== "e") h = rowsForPx(py - m.top, m.row, m.gap);
  return { size, h };
}

/** Šířka obrysu cíle v px (`span` sloupců). */
export const spanPx = (span: number, colW: number, gap: number): number => span * colW + (span - 1) * gap;
