// Barevné povrchy karet (kolo 28): Přechod · Přelévání · Záře hrany · Proužky a jejich barevné sady.
// Čistá data a matematika bez importů (test `surfaces.test.mjs`): sada dává tři barvy (`--sf-a`, `--sf-b`
// přechodu a `--sf-e` hrany), CSS (`surfaces.css`) z nich skládá plochu. Sada „card“ (výchozí = pole chybí)
// bere barvu z akcentu karty, takže bez volby se nemění nic.

export type SurfaceKind = "gradient" | "flow" | "edge" | "stripes";
export type CardPalette = "aurora" | "sunset" | "ocean" | "embers";
type Text = readonly [string, string];

export const COLOR_SURFACES: readonly SurfaceKind[] = ["gradient", "flow", "edge", "stripes"];
export const isColorSurface = (s: unknown): s is SurfaceKind => typeof s === "string" && (COLOR_SURFACES as readonly string[]).includes(s);
/** Povrchy s tmavým přechodem a světlým textem (u „Záře“ a „Proužků“ zůstává text podle stylu). */
export const isGradientSurface = (s: unknown): boolean => s === "gradient" || s === "flow";

export const SURFACE_KIND_NAMES: Record<SurfaceKind, Text> = {
  gradient: ["cards.cardlook.surface.gradient", "Přechod"],
  flow: ["cards.cardlook.surface.flow", "Přelévání"],
  edge: ["cards.cardlook.surface.edge", "Záře hrany"],
  stripes: ["cards.cardlook.surface.stripes", "Proužky"],
};

/** `card` = podle akcentu karty (pole `palette` chybí). */
export const PALETTES = ["card", "aurora", "sunset", "ocean", "embers"] as const;
export const PALETTE_NAMES: Record<(typeof PALETTES)[number], Text> = {
  card: ["cards.cardlook.palette.card", "Podle akcentu karty"],
  aurora: ["cards.cardlook.palette.aurora", "Polární záře"],
  sunset: ["cards.cardlook.palette.sunset", "Západ slunce"],
  ocean: ["cards.cardlook.palette.ocean", "Oceán"],
  embers: ["cards.cardlook.palette.embers", "Žhavé uhlíky"],
};

/** Barvy sad: a = začátek přechodu, b = střed, e = barva zářící hrany a proužků. */
export const PALETTE_COLORS: Record<CardPalette, { a: string; b: string; e: string }> = {
  aurora: { a: "#0c5d50", b: "#2f3f8f", e: "#34d399" },
  sunset: { a: "#8f2d0a", b: "#7a1d58", e: "#fb923c" },
  ocean: { a: "#0a5469", b: "#1b2f78", e: "#38bdf8" },
  embers: { a: "#7f340a", b: "#7a1a1a", e: "#f59e0b" },
};
/** Dno přechodu (nejtmavší konec, kde sedí velký obsah). */
export const SURFACE_DARK = "#0c0c12";
/** Podíl akcentu v sadě „card“ — zrcadlo `color-mix` v `surfaces.css`. */
export const CARD_MIX_A = 0.34;
export const CARD_MIX_B = 0.2;
/** Barvy textu na přechodu (přepisují `--t1/--t2/--t3` karty). */
export const GRADIENT_TEXT = { t1: "#f6f6fa", t2: "#e2e3ec", t3: "#cfd1de" } as const;

export const normalizePalette = (x: unknown): CardPalette | undefined =>
  typeof x === "string" && x !== "card" && Object.prototype.hasOwnProperty.call(PALETTE_COLORS, x) ? (x as CardPalette) : undefined;

/** Proměnné pro `.deck-item` (prázdné = sada „card“ / bez barevného povrchu). */
export function paletteStyle(surface: unknown, palette: CardPalette | undefined): Record<string, string> {
  if (!isColorSurface(surface) || !palette) return {};
  const c = PALETTE_COLORS[palette];
  return { "--sf-a": c.a, "--sf-b": c.b, "--sf-e": c.e };
}

// ---- kontrast (WCAG) ----
const rgb = (h: string): [number, number, number] => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];
const lin = (v: number) => ((v /= 255) <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
export const luminance = (h: string): number => {
  const [r, g, b] = rgb(h);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
};
export const contrast = (a: string, b: string): number => {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};
/** `color-mix(in srgb, fg k%, bg)` jako hex. */
export function mixHex(fg: string, bg: string, k: number): string {
  const f = rgb(fg), g = rgb(bg);
  return "#" + f.map((v, i) => Math.round(v * k + g[i]! * (1 - k)).toString(16).padStart(2, "0")).join("");
}
/** Všechny stopy přechodu pro danou sadu a (u „card“) akcent — pod nimi leží text. */
export function gradientStops(palette: CardPalette | undefined, accentHex?: string): string[] {
  if (palette) return [PALETTE_COLORS[palette].a, PALETTE_COLORS[palette].b, SURFACE_DARK];
  const ac = accentHex ?? "#a78bfa";
  return [mixHex(ac, SURFACE_DARK, CARD_MIX_A), mixHex(ac, SURFACE_DARK, CARD_MIX_B), SURFACE_DARK];
}
