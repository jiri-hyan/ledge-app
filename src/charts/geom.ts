// Geometrie vlastních SVG grafů (kolo 22) — čisté funkce bez DOM, bez knihoven a bez CDN (test `geom.test.mjs`).
// Komponenty v `src/charts/*.tsx` jsou jen tenké obaly: vezmou čísla, zavolají tyhle funkce a nakreslí výsledek.
// Pravidla pro všechny grafy:
//  - poctivý prázdný stav: méně než 2 konečná čísla = `null` / prázdné pole (graf se nekreslí, nic se nevymýšlí);
//    výjimka je prstenec — ten ukazuje jedno číslo (podíl 0–1), nekonečné / chybějící = `null`;
//  - nekonečná a chybějící čísla (`NaN`, `Infinity`, `null`) se vynechají, nikdy se nenahradí nulou;
//  - souřadnice zaokrouhlené na 2 desetiny (krátké řetězce, stabilní testy);
//  - osa y roste dolů (SVG); `pad` = okraj, aby tloušťka čáry nepřetekla viewBox.

export type Pt = { x: number; y: number };
export type Rect = { x: number; y: number; w: number; h: number };

const r2 = (n: number) => Math.round(n * 100) / 100;
const clamp01 = (n: number) => Math.min(1, Math.max(0, n));

/** Jen konečná čísla (pořadí zůstává). */
export const finite = (values: readonly (number | null | undefined)[]): number[] => values.filter((v): v is number => typeof v === "number" && Number.isFinite(v));

/** Rozsah osy: `min`/`max` z voleb, jinak z dat; plochá data (min = max) dostanou rozsah 1, ať se dá dělit. */
export function extent(values: readonly number[], opts: { min?: number; max?: number } = {}): { lo: number; hi: number } {
  const lo = opts.min ?? Math.min(...values);
  const hi = opts.max ?? Math.max(...values);
  return hi > lo ? { lo, hi } : { lo, hi: lo + 1 };
}

export type SparkOpts = { min?: number; max?: number; pad?: number };
export type Spark = { line: string; area: string; last: Pt; points: Pt[] };

/** Body křivky přes celou šířku `w` (první vlevo, poslední vpravo). < 2 čísla = `null`. */
export function sparkPoints(values: readonly (number | null | undefined)[], w: number, h: number, opts: SparkOpts = {}): Pt[] | null {
  const v = finite(values);
  if (v.length < 2 || !(w > 0) || !(h > 0)) return null;
  const pad = opts.pad ?? 1.5;
  const { lo, hi } = extent(v, opts);
  const step = (w - 2 * pad) / (v.length - 1);
  return v.map((y, i) => ({ x: r2(pad + i * step), y: r2(pad + (h - 2 * pad) * (1 - clamp01((y - lo) / (hi - lo)))) }));
}

/** Sparkline: `line` = path čáry, `area` = path plochy pod ní (uzavřená k dolnímu okraji), `last` = poslední bod. */
export function sparkline(values: readonly (number | null | undefined)[], w: number, h: number, opts: SparkOpts = {}): Spark | null {
  const pts = sparkPoints(values, w, h, opts);
  if (!pts) return null;
  const line = pts.map((p, i) => `${i ? "L" : "M"}${p.x} ${p.y}`).join(" ");
  const first = pts[0];
  const last = pts[pts.length - 1];
  return { line, area: `${line} L${last.x} ${r2(h)} L${first.x} ${r2(h)} Z`, last, points: pts };
}

export type Ring = { r: number; c: number; dash: string; frac: number };

/** Prstenec (podíl 0–1) jako `stroke-dasharray` kružnice o poloměru `size/2 − stroke/2`; mimo 0–1 se ořízne,
 *  chybějící / nekonečné číslo = `null` (nic se nekreslí). */
export function ring(frac: number | null | undefined, size: number, stroke: number): Ring | null {
  if (typeof frac !== "number" || !Number.isFinite(frac) || !(size > 0) || !(stroke > 0) || stroke * 2 >= size) return null;
  const f = clamp01(frac);
  const r = r2(size / 2 - stroke / 2);
  const c = r2(2 * Math.PI * r);
  return { r, c, dash: `${r2(c * f)} ${c}`, frac: f };
}

export type BarOpts = { gap?: number; max?: number; minH?: number };

/** Sloupce zdola nahoru přes šířku `w`; výška podle `max` (jinak největší hodnota). Záporné = 0.
 *  Nulový sloupec má výšku `minH` (výchozí 0 = nevidět), ať je poznat „den bez dat“ od chybějícího grafu.
 *  < 2 čísla = prázdné pole. */
export function bars(values: readonly (number | null | undefined)[], w: number, h: number, opts: BarOpts = {}): Rect[] {
  const v = finite(values).map((x) => Math.max(0, x));
  if (v.length < 2 || !(w > 0) || !(h > 0)) return [];
  const gap = opts.gap ?? 2;
  const top = opts.max ?? Math.max(...v);
  const bw = (w - gap * (v.length - 1)) / v.length;
  if (bw <= 0) return [];
  const minH = opts.minH ?? 0;
  return v.map((x, i) => {
    const bh = top > 0 ? Math.max(minH, h * clamp01(x / top)) : minH;
    return { x: r2(i * (bw + gap)), y: r2(h - bh), w: r2(bw), h: r2(bh) };
  });
}

export type HeatOpts = { rows?: number; gap?: number; levels?: number; max?: number };
export type HeatCell = Rect & { level: number; i: number };

/** Úroveň 0…`levels` (0 = nula / nic, nejvyšší = maximum). */
export function heatLevel(v: number, max: number, levels = 4): number {
  if (!(v > 0) || !(max > 0)) return 0;
  return Math.max(1, Math.min(levels, Math.ceil(clamp01(v / max) * levels)));
}

/** Pás (heat): buňky po sloupcích shora dolů (`rows` = 1 → jeden řádek; 7 → týdny jako u GitHubu).
 *  < 2 čísla = prázdné pole. Hodnota se nikdy nevymýšlí — chybějící číslo buňku vynechá (pořadí zůstává). */
export function heat(values: readonly (number | null | undefined)[], w: number, h: number, opts: HeatOpts = {}): HeatCell[] {
  const v = finite(values).map((x) => Math.max(0, x));
  if (v.length < 2 || !(w > 0) || !(h > 0)) return [];
  const rows = Math.max(1, Math.floor(opts.rows ?? 1));
  const cols = Math.ceil(v.length / rows);
  const gap = opts.gap ?? 2;
  const levels = Math.max(1, Math.floor(opts.levels ?? 4));
  const cw = (w - gap * (cols - 1)) / cols;
  const ch = (h - gap * (rows - 1)) / rows;
  if (cw <= 0 || ch <= 0) return [];
  const top = opts.max ?? Math.max(...v);
  return v.map((x, i) => {
    const col = Math.floor(i / rows);
    const row = i % rows;
    return { i, x: r2(col * (cw + gap)), y: r2(row * (ch + gap)), w: r2(cw), h: r2(ch), level: heatLevel(x, top, levels) };
  });
}

/** Trend posledního bodu vůči předposlednímu: znaménko a rozdíl (< 2 čísla = `null`). Pro šipku u velkého čísla. */
export function trend(values: readonly (number | null | undefined)[]): { dir: -1 | 0 | 1; delta: number } | null {
  const v = finite(values);
  if (v.length < 2) return null;
  const delta = v[v.length - 1] - v[v.length - 2];
  return { dir: delta > 0 ? 1 : delta < 0 ? -1 : 0, delta };
}
