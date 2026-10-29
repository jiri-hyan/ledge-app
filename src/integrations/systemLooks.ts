// Vzhledy karty Systém (kolo 25, `look.design`): čistá logika bez DOM a bez `t()` (test `systemLooks.test.mjs`).
// Všechno se počítá z existujícího pohledu `system_view` (historie vzorků po 2 s, disky); nic navíc se neměří
// a nic se nevymýšlí — prázdná historie = prázdné pole, chybějící disk = null.
import { diskUsed, loadOf, type Load } from "./format.ts";

/** Odstup vzorků karty (ms) — shodně s `SystemCard.tsx`; slouží jen k popisu stáří vzorku v tooltipu. */
export const EVERY_MS = 2000;

/** Posledních `n` vzorků (nejnovější vpravo), nečísla pryč. */
export const lastN = (hist: readonly number[] | undefined, n: number): number[] => (n > 0 ? (hist ?? []).filter((x) => Number.isFinite(x)).slice(-n) : []);

/** Průměr a špička historie; prázdná = null. */
export function histStats(hist: readonly number[] | undefined): { avg: number; peak: number; n: number } | null {
  const v = lastN(hist, Infinity);
  if (!v.length) return null;
  return { avg: v.reduce((a, b) => a + b, 0) / v.length, peak: Math.max(...v), n: v.length };
}

/** Stáří i-tého z `n` vzorků v sekundách (poslední = 0). */
export const ageSec = (i: number, n: number, everyMs = EVERY_MS): number => Math.round(((n - 1 - i) * everyMs) / 1000);

/** Textové pruhy jako v htop: `cells` znaků, zaplněné ▮, prázdné ▯. Procento se zaokrouhlí na celé buňky,
 *  nenulová hodnota má vždy aspoň jednu (ať 1 % není nic); mimo rozsah se ořízne, nečíslo = samé prázdné. */
export function termCells(pct: number | null | undefined, cells: number): string {
  const n = Math.max(0, Math.floor(cells));
  const p = typeof pct === "number" && Number.isFinite(pct) ? Math.max(0, Math.min(100, pct)) : 0;
  const on = p > 0 ? Math.min(n, Math.max(1, Math.round((p / 100) * n))) : 0;
  return "▮".repeat(on) + "▯".repeat(n - on);
}

/** Nejzaplněnější disk (pro prstenec / budík / pruh); bez disků null. */
export function fullestDisk<D extends { letter: string; total: number; free: number }>(disks: readonly D[] | undefined): { disk: D; used: number } | null {
  let best: { disk: D; used: number } | null = null;
  for (const d of disks ?? []) {
    const used = diskUsed(d.free, d.total);
    if (d.total > 0 && (!best || used > best.used)) best = { disk: d, used };
  }
  return best;
}

/** Sloupky histogramu: výška 0–1 a úroveň zátěže (stejné meze jako číslo CPU) pro každý vzorek. */
export function loadBars(hist: readonly number[] | undefined, n: number): { h: number; load: Load; pct: number; age: number }[] {
  const v = lastN(hist, n);
  return v.map((pct, i) => ({ pct, h: Math.max(0, Math.min(1, pct / 100)), load: loadOf(pct), age: ageSec(i, v.length) }));
}

/** Věta tooltipu o historii: „posledních 2 min“ / „posledních 40 s“. */
export function windowSec(n: number, everyMs = EVERY_MS): number {
  return Math.round((n * everyMs) / 1000);
}
