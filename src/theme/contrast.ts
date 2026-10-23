// Kontrast textu (WCAG) jako čisté funkce bez DOM: jedna implementace pro test motivů (`theme.test.mjs`)
// i pro živou kontrolu v editoru vlastního motivu (`customThemes.ts`, Nastavení → Vzhled). Rust má zrcadlo
// `appearance::geom::contrast` (testy `contrast_table_matches_frontend` a „vlastní motiv: kontrast“ sdílejí tabulku).

export type Rgba = [number, number, number, number];

/** Hranice čitelného textu (WCAG AA). */
export const MIN_CONTRAST = 4.5;

/** `#rgb`, `#rrggbb`, `rgb()/rgba()` nebo `r g b` (token `--bg`); jiná podoba = výjimka. */
export function parseColor(c: string): Rgba {
  c = c.trim();
  let m = c.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
  if (m) {
    const h = m[1].length === 3 ? [...m[1]].map((x) => x + x).join("") : m[1];
    const n = parseInt(h, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  m = c.match(/^rgba?\(([^)]+)\)$/);
  if (m) {
    const p = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
    return [p[0], p[1], p[2], p[3] ?? 1];
  }
  m = c.match(/^(\d+) (\d+) (\d+)$/);
  if (m) return [+m[1], +m[2], +m[3], 1];
  throw new Error(`unknown color ${c}`);
}

/** Barva `top` s jejím krytím položená na `bottom`. */
export const over = (top: Rgba, bottom: Rgba): Rgba => {
  const a = top[3];
  return [0, 1, 2].map((i) => top[i] * a + bottom[i] * (1 - a)).concat(1) as Rgba;
};

/** Relativní svítivost 0–1. */
export const lum = ([r, g, b]: Rgba | number[]): number => {
  const f = (v: number) => ((v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
};

/** Kontrastní poměr dvou barev (1–21). */
export const contrast = (a: Rgba, b: Rgba): number => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

export const toHex = ([r, g, b]: Rgba | number[]): string =>
  "#" + [r, g, b].map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0")).join("");

/** Všechny texty motivu, které musí být čitelné na kartách (barvy stavů a jejich „inkoust“ na tlačítkách). */
export const TEXT_COLORS = ["t1", "t2", "t3", "green", "amber", "red", "blue", "violet", "teal", "amber-ink", "red-ink"] as const;

/** Plochy pod textem: karta a karta při najetí, na pozadí šuplíku (krytí 0,985) nad tmavou i světlou plochou. */
export function surfaces(v: Record<string, string>): Rgba[] {
  const bg = parseColor(v.bg);
  const out: Rgba[] = [];
  for (const desk of [[0, 0, 0, 1], [255, 255, 255, 1]] as Rgba[]) {
    const shell = over([bg[0], bg[1], bg[2], 0.985], desk);
    for (const layer of ["card", "card-2"]) out.push(over(parseColor(v[layer]), shell));
  }
  return out;
}

export type TextProblem = { key: string; ratio: number };

/** Texty s kontrastem pod 4.5:1 na nejhorší ploše (prázdné = v pořádku) + tlačítko `.btn.pri` (`on-light` na `t1`). */
export function textProblems(v: Record<string, string>, keys: readonly string[] = TEXT_COLORS): TextProblem[] {
  const planes = surfaces(v);
  const out: TextProblem[] = [];
  for (const key of keys) {
    const ratio = Math.min(...planes.map((s) => contrast(parseColor(v[key]), s)));
    if (ratio < MIN_CONTRAST) out.push({ key, ratio });
  }
  const btn = contrast(parseColor(v["on-light"]), parseColor(v.t1));
  if (btn < MIN_CONTRAST) out.push({ key: "on-light", ratio: btn });
  return out;
}
