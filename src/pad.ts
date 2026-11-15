// Drobná čísla pro čistou logiku (bez importů za běhu, jde z testů přes node): `pad2`, `clampNum`, `clampInt`.
// Další `clamp` v repu mají jinou sémantiku (`palette/media.ts` zaokrouhluje 0–100, `ui/place.ts` při hi < lo vrací lo) — ať zůstanou.

// Dvoumístné číslo s nulou vpředu („07“).
export const pad2 = (n: number): string => String(n).padStart(2, "0");

/** Ořez do <lo, hi>; nekonečno / NaN = `hi` (pin: průhlednost a čitelnost textu). */
export const clampNum = (v: number, lo: number, hi: number): number => Math.min(hi, Math.max(lo, Number.isFinite(v) ? v : hi));

/** Neznámá hodnota → celé číslo v <a, b>; nečíslo / nekonečno = `d` (konfigurace proužku, zrcadlo Rust ořezu). */
export const clampInt = (v: unknown, a: number, b: number, d: number): number => {
  const n = typeof v === "number" && Number.isFinite(v) ? Math.round(v) : d;
  return Math.min(b, Math.max(a, n));
};
