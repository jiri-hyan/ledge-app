// Ledge Studio — lidské názvy hodnot v časové ose změn (kolo 24): „Barvy: Tmavý → Nord“ místo „dark → nord“.
// Čistá tabulka path → (hodnota → [klíč, česky]); neznámá cesta / hodnota = null (volající ukáže hodnotu syrově).
// Test `named.test.mjs`.
import { FONTS, LAYOUTS, MATERIALS, STYLES, WIDTHS, type Pair } from "../theme/looks.ts";
import { THEMES } from "../theme/themes.ts";
import { STRIP_LOOKS } from "../strip/looks.ts";
import { HOVER_MODES } from "../hover/modes.ts";

const byId = (xs: readonly { id: string; key: string; cs: string }[]) => (v: string): Pair | null => {
  const x = xs.find((y) => y.id === v);
  return x ? [x.key, x.cs] : null;
};
const rec = (r: Record<string, { name: Pair }>) => (v: string): Pair | null => r[v]?.name ?? null;

const TABLE: Record<string, (v: string) => Pair | null> = {
  "strip.look": byId(STRIP_LOOKS),
  "drawer.hover": byId(HOVER_MODES),
  "appearance.style": (v) => STYLES.find((s) => s.id === v)?.name ?? null,
  "appearance.theme": (v) => { const x = (THEMES as Record<string, { key: string; cs: string }>)[v]; return x ? [x.key, x.cs] : null; },
  "appearance.layout": rec(LAYOUTS),
  "appearance.font": rec(FONTS),
  "appearance.material": rec(MATERIALS),
  "appearance.width": rec(WIDTHS),
};

/** Název hodnoty volby, nebo null. */
export function namedValue(path: string, v: string): Pair | null {
  return TABLE[path]?.(v) ?? null;
}
