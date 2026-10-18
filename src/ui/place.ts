// Umístění plovoucích prvků (menu dlaždice, popover Vzhled… / Upravit…, náhled po najetí) a počet sloupců
// mřížky dlaždic — čisté funkce bez DOM (test `place.test.mjs`). Volající změří kotvu, velikost a okno.

export type Box = { left: number; top: number; right: number; bottom: number };
export type Size = { w: number; h: number };
export type Placed = { x: number; y: number; flipped: boolean; maxH: number };

/** Bod (pravý klik myší) jako kotva nulové velikosti. */
export const pointBox = (x: number, y: number): Box => ({ left: x, top: y, right: x, bottom: y });

/**
 * Pod kotvu (zarovnané na levý okraj, `align: "end"` na pravý); když se dolů nevejde, otočí se nad ni;
 * když ani tam, dá se na stranu s víc místem a posune dovnitř okna. Vždy celé v okně s okrajem `margin`;
 * vyšší než okno → nahoře s `maxH` (obsah se posouvá uvnitř). Širší než okno → od levého okraje.
 */
export function placePopup(anchor: Box, size: Size, view: Size, opts: { margin?: number; gap?: number; align?: "start" | "end" } = {}): Placed {
  const m = opts.margin ?? 8;
  const gap = opts.gap ?? 4;
  const maxH = Math.max(0, view.h - 2 * m);
  const h = Math.min(size.h, maxH);
  const w = Math.min(size.w, Math.max(0, view.w - 2 * m));
  const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(v, Math.max(lo, hi)));
  const x = clamp(opts.align === "end" ? anchor.right - w : anchor.left, m, view.w - m - w);
  const below = anchor.bottom + gap;
  const above = anchor.top - gap - h;
  let y: number;
  let flipped = false;
  if (below + h <= view.h - m) y = below;
  else if (above >= m) {
    y = above;
    flipped = true;
  } else {
    const roomBelow = view.h - m - below;
    const roomAbove = anchor.top - gap - m;
    flipped = roomAbove > roomBelow;
    y = clamp(flipped ? above : below, m, view.h - m - h);
  }
  return { x: Math.round(x), y: Math.round(y), flipped, maxH };
}

/**
 * Kolik sloupců dlaždic: tolik, kolik se jich vejde při minimální šířce `min` a mezeře `gap`, ale nejvýš
 * počet dlaždic — sloupce pak vyplní celou šířku karty (žádné prázdné místo za poslední ikonou), přebytek
 * se zalomí na další řádek. Aspoň 1.
 */
export function tileColumns(width: number, min: number, gap: number, count: number): number {
  if (!(width > 0) || !(min > 0)) return Math.max(1, Math.min(count, 1));
  const fit = Math.max(1, Math.floor((width + gap) / (min + Math.max(0, gap))));
  return Math.max(1, Math.min(fit, Math.max(1, count)));
}
