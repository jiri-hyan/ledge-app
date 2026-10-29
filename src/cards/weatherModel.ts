// Čistá logika vzhledů karty Počasí (kolo 25) — bez DOM a bez `t()`, test `weatherModel.test.mjs`.
// Všechny časy z Open-Meteo jsou MÍSTNÍ čas místa („2026-10-16T07:00“) a porovnávají se jako text / minuty,
// nikdy přes `Date` — jinak by je posunulo pásmo počítače. Co zdroj nemá, vrací funkce null (karta ukáže prázdno).
import type { Weather, WeatherHour } from "../types/personal.ts";

export type WxKind = "clear" | "partly" | "cloud" | "fog" | "drizzle" | "rain" | "snow" | "storm";

/** WMO kód → druh počasí pro ikonu a pozadí. Neznámý kód = null (nic se nehádá). */
export function wxKind(code: number | null | undefined): WxKind | null {
  if (code == null) return null;
  if (code === 0 || code === 1) return "clear";
  if (code === 2) return "partly";
  if (code === 3) return "cloud";
  if (code === 45 || code === 48) return "fog";
  if (code >= 51 && code <= 57) return "drizzle";
  if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82)) return "rain";
  if ((code >= 71 && code <= 77) || code === 85 || code === 86) return "snow";
  if (code >= 95 && code <= 99) return "storm";
  return null;
}

/** Minuty od půlnoci z „YYYY-MM-DDTHH:MM“; jinak null. */
export function minutesOf(iso: string | null | undefined): number | null {
  const m = /T(\d{2}):(\d{2})/.exec(iso ?? "");
  return m ? +m[1] * 60 + +m[2] : null;
}

/** Datum „YYYY-MM-DD“ z času; jinak null. */
export const dateOf = (iso: string | null | undefined): string | null => /^\d{4}-\d{2}-\d{2}/.exec(iso ?? "")?.[0] ?? null;

/** „7:05“ z „…T07:05“; jinak „—“. */
export function clockOf(iso: string | null | undefined): string {
  const m = /T(\d{2}):(\d{2})/.exec(iso ?? "");
  return m ? `${+m[1]}:${m[2]}` : "—";
}

export interface SunPos {
  phase: "before" | "day" | "after";
  /** 0 = východ … 1 = západ (u before / after 0 / 1) */
  p: number;
}

/** Kde je slunce podle místního „teď“ (`observed`) a dnešního východu / západu. Jiný den než slunce nebo chybějící údaj = null. */
export function sunPos(sunrise: string | null | undefined, sunset: string | null | undefined, observed: string | null | undefined): SunPos | null {
  const [a, b, n] = [minutesOf(sunrise), minutesOf(sunset), minutesOf(observed)];
  if (a == null || b == null || n == null || b <= a) return null;
  if (dateOf(sunrise) !== dateOf(observed)) return null;
  if (n < a) return { phase: "before", p: 0 };
  if (n > b) return { phase: "after", p: 1 };
  return { phase: "day", p: (n - a) / (b - a) };
}

/** Hodiny od aktuální hodiny dál (nejvýš `n`); jen ty, co mají teplotu. */
export function hoursFrom(hours: readonly WeatherHour[] | undefined, observed: string, n = 24): WeatherHour[] {
  const cut = observed.slice(0, 13);
  return (hours ?? []).filter((h) => h.temp != null && h.time.slice(0, 13) >= cut).slice(0, n);
}

/** Poloha hodnoty v rozsahu 0–1 (oříznuto); nulový rozsah = střed. */
export function rangePos(v: number, lo: number, hi: number): number {
  if (!(hi > lo)) return 0.5;
  return Math.max(0, Math.min(1, (v - lo) / (hi - lo)));
}

/** Odstín podle teploty (°C): ≤0 modrá · 10 zelená · 20 žlutá · ≥30 červená, mezi tím lineárně. */
export function tempHue(c: number): number {
  const pts: [number, number][] = [[0, 215], [10, 130], [20, 55], [30, 0]];
  if (c <= pts[0][0]) return pts[0][1];
  for (let i = 1; i < pts.length; i++) {
    if (c <= pts[i][0]) {
      const [t0, h0] = pts[i - 1];
      const [t1, h1] = pts[i];
      return Math.round(h0 + ((h1 - h0) * (c - t0)) / (t1 - t0));
    }
  }
  return 0;
}

/** Barva teploty (°C), viz `tempHue`. */
export function tempColor(c: number): string {
  return `hsl(${tempHue(c)} 70% 55%)`;
}

/** Gradient pruhu od `lo` do `hi` °C, barvy podle absolutní teploty. */
export function tempGradient(lo: number, hi: number): string {
  const steps = 5;
  const stops = Array.from({ length: steps + 1 }, (_, i) => `${tempColor(lo + ((hi - lo) * i) / steps)} ${Math.round((i / steps) * 100)}%`);
  return `linear-gradient(90deg, ${stops.join(", ")})`;
}

/** Body křivky 0–1 (x podle indexu, y obrácené: nejvyšší teplota nahoře). Méně než 2 body = null. */
export function curvePoints(temps: readonly number[]): { x: number; y: number }[] | null {
  if (temps.length < 2) return null;
  const lo = Math.min(...temps);
  const hi = Math.max(...temps);
  return temps.map((v, i) => ({ x: i / (temps.length - 1), y: hi > lo ? 1 - (v - lo) / (hi - lo) : 0.5 }));
}

/** Index nejbližšího bodu pro polohu 0–1 na ose (hover nad křivkou). */
export const nearestIndex = (frac: number, count: number): number => (count < 1 ? 0 : Math.max(0, Math.min(count - 1, Math.round(frac * (count - 1)))));

/** Index světové strany 0–7 (S, SV, V, JV, J, JZ, Z, SZ) podle směru větru ve stupních, odkud vane. */
export const windIndex = (deg: number): number => ((Math.round((((deg % 360) + 360) % 360) / 45) % 8) + 8) % 8;

/** Den v týdnu (0 = neděle) z „YYYY-MM-DD“ bez zásahu pásma; špatný text = null. */
export function dowOf(date: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(date);
  return m ? new Date(Date.UTC(+m[1], +m[2] - 1, +m[3])).getUTCDay() : null;
}

export interface DayRow {
  date: string;
  min: number;
  max: number;
  code: number | null;
  today: boolean;
}

/** Řádky vzhledu „Pět dní“: dnešek (z `Weather.min/max`) a další dny; dny bez min nebo max se vynechají. */
export function dayRows(w: Pick<Weather, "observed" | "min" | "max" | "code" | "days">): DayRow[] {
  const rows: DayRow[] = [];
  const date = dateOf(w.observed);
  if (date && w.min != null && w.max != null) rows.push({ date, min: w.min, max: w.max, code: w.code, today: true });
  for (const d of w.days ?? []) if (d.min != null && d.max != null) rows.push({ date: d.date, min: d.min, max: d.max, code: d.code, today: false });
  return rows;
}

/** Id pozadí „Živé oblohy“: druh počasí + den / noc. Neznámý kód = jasná obloha podle dne a noci. */
export function skyId(code: number, isDay: boolean): string {
  return `${wxKind(code) ?? "clear"}-${isDay ? "day" : "night"}`;
}
