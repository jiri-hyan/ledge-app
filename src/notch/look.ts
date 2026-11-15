// Čistá kresba sekcí lišty notche (kolo 27): barvy podle volby (akcent / zátěž / přechod / vlastní hex), tloušťka a
// vyhlazení čar grafů, úhly ručiček, otočné kolečko a sloupky hlasitosti, textový pruh htop, příští schůzka a
// počasí z existujících dat. Nic se nevymýšlí: bez vzorků / dat vrací null a sekce ukáže prázdný stav.
// Bez DOM, bez `t()` (test `look.test.mjs`).
import { FULL_PCT, WARN_PCT } from "../charts/thresholds.ts";
import { sparkPoints, type Pt } from "../charts/geom.ts";
import { nextEvents } from "../board/weekModel.ts";
import { weatherAge } from "../cards/weatherAge.ts";
import type { CalendarItem, CalendarView } from "../types/calendar.ts";

/** Prahy „zelená → jantar → červená“: JEDNA tabulka s Rustem (`looks/capsule.rs` `WARN_PCT` / `FULL_PCT`, test `look.test.mjs`)
 *  a `integrations/format.ts` `loadOf`. */
export const HEAT_WARN = WARN_PCT;
export const HEAT_FULL = FULL_PCT;
/** Kotvy spojité barvy (sRGB 0–1) = Rust `Rgba::GREEN` / `AMBER` / `RED`. */
const HEAT_RGB = { green: [0.29, 0.87, 0.5], amber: [0.98, 0.75, 0.14], red: [0.97, 0.44, 0.44] } as const;

/** Barva podle zátěže (diskrétně): `var(--green)` do 75 %, `var(--amber)` od 75 %, `var(--red)` od 90 %. */
export const loadColor = (pct: number | null | undefined): string =>
  pct == null || !Number.isFinite(pct) ? "var(--green)" : pct >= HEAT_FULL ? "var(--red)" : pct >= HEAT_WARN ? "var(--amber)" : "var(--green)";

/** Spojitá barva `#rrggbb` podle hodnoty (zrcadlo Rust `capsule::heat`): zelená → jantar v `HEAT_WARN` → červená ve `HEAT_FULL`. */
export function heatHex(pct: number | null | undefined): string {
  const mix = (a: readonly number[], b: readonly number[], t: number) => a.map((x, i) => x * (1 - t) + b[i] * t);
  const p = pct == null || !Number.isFinite(pct) ? 0 : pct;
  const c = p <= 0 ? HEAT_RGB.green : p < HEAT_WARN ? mix(HEAT_RGB.green, HEAT_RGB.amber, p / HEAT_WARN) : p < HEAT_FULL ? mix(HEAT_RGB.amber, HEAT_RGB.red, (p - HEAT_WARN) / (HEAT_FULL - HEAT_WARN)) : HEAT_RGB.red;
  return "#" + c.map((x) => Math.round(x * 255).toString(16).padStart(2, "0")).join("");
}

/** Výplň prvku sekce: plná barva (CSS), nebo přechod (`gradient` — komponenta kreslí SVG / CSS přechod). */
export type Paint = { kind: "solid"; css: string } | { kind: "gradient" };

export function paintOf(color: string | undefined, pct?: number | null): Paint {
  if (color === "gradient") return { kind: "gradient" };
  if (color === "load") return { kind: "solid", css: loadColor(pct) };
  if (color && /^#[0-9a-fA-F]{6}$/.test(color)) return { kind: "solid", css: color };
  return { kind: "solid", css: "var(--accent)" };
}

/** CSS barva pro prvky, které přechod neumí (text, tečka): přechod = akcent. */
export const solidOf = (color: string | undefined, pct?: number | null): string => {
  const p = paintOf(color, pct);
  return p.kind === "solid" ? p.css : "var(--accent)";
};

/** CSS přechod (pruhy, sloupky): akcent → druhý akcent. */
export const GRADIENT_CSS = "linear-gradient(90deg, var(--accent), var(--accent-2))";

/** Tloušťka čáry grafu v px; chybí = střední (dnešní 1,6 z `charts.css`). */
export const weightPx = (w: string | undefined): number => (w === "thin" ? 1 : w === "bold" ? 2.6 : 1.6);

/** Lomená čára přes body. */
export const linePath = (pts: readonly Pt[]): string => pts.map((p, i) => `${i ? "L" : "M"}${p.x} ${p.y}`).join(" ");

const r2 = (n: number) => Math.round(n * 100) / 100;

/** Vyhlazená čára (Catmull-Rom → kubické Bézierovy úseky) přes tytéž body; nepřestřelí mimo [minY, maxY] okna. */
export function smoothPath(pts: readonly Pt[], h: number): string {
  if (pts.length < 3) return linePath(pts);
  const cl = (y: number) => Math.max(0, Math.min(h, y));
  let d = `M${pts[0].x} ${pts[0].y}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i - 1] ?? pts[i];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[i + 2] ?? p2;
    const c1 = { x: r2(p1.x + (p2.x - p0.x) / 6), y: r2(cl(p1.y + (p2.y - p0.y) / 6)) };
    const c2 = { x: r2(p2.x - (p3.x - p1.x) / 6), y: r2(cl(p2.y - (p3.y - p1.y) / 6)) };
    d += ` C${c1.x} ${c1.y} ${c2.x} ${c2.y} ${p2.x} ${p2.y}`;
  }
  return d;
}

/** Graf z reálných vzorků: čára (lomená / vyhlazená) a plocha pod ní. < 2 vzorky = null (prázdno, nic se nekreslí). */
export function chartPaths(values: readonly (number | null | undefined)[], w: number, h: number, smooth: boolean, max?: number): { line: string; area: string } | null {
  const pts = sparkPoints(values, w, h, { min: 0, max });
  if (!pts) return null;
  const line = smooth ? smoothPath(pts, h) : linePath(pts);
  return { line, area: `${line} L${pts[pts.length - 1].x} ${h} L${pts[0].x} ${h} Z` };
}

/** Posledních `n` vzorků jako sloupky 0–1 (výška podle `max`; bez vzorků prázdné pole). */
export function lastBars(values: readonly (number | null | undefined)[], n: number, max = 100): number[] {
  const v = values.filter((x): x is number => typeof x === "number" && Number.isFinite(x)).slice(-n);
  return v.map((x) => Math.max(0, Math.min(1, x / (max > 0 ? max : 1))));
}

/** Úhly ručiček (stupně, 0 = nahoru, po směru hodin). */
export function handAngles(d: Date): { h: number; m: number; s: number } {
  const s = d.getSeconds();
  const m = d.getMinutes() + s / 60;
  const h = (d.getHours() % 12) + m / 60;
  return { h: r2(h * 30), m: r2(m * 6), s: s * 6 };
}

/** Úhel rysky otočného kolečka hlasitosti: 0 % = −135°, 100 % = +135°. */
export const knobAngle = (level: number): number => -135 + 2.7 * Math.max(0, Math.min(100, level));

/** Sloupky hlasitosti: kolik z `n` svítí (statické podle úrovně; ztlumeno = 0). */
export const eqLit = (level: number, n: number, muted = false): number => (muted ? 0 : Math.round((Math.max(0, Math.min(100, level)) / 100) * n));

/** Textový pruh jako htop: `[|||||     ]` se šířkou `w` znaků uvnitř. */
export function htopBar(pct: number, w = 14): string {
  const k = Math.round((Math.max(0, Math.min(100, pct)) / 100) * w);
  return `[${"|".repeat(k)}${" ".repeat(w - k)}]`;
}

/** Krok kolečka myši / šipky na hlasitosti (±2 %), ořízne 0–100. */
export const nudge = (level: number, dir: number): number => Math.max(0, Math.min(100, Math.round(level + (dir > 0 ? 2 : dir < 0 ? -2 : 0))));

/** Příští schůzka z kalendáře v paměti: nejbližší neskončená událost s časem (celodenní ne); nic = null. */
export function nextMeeting(cal: CalendarView | null | undefined, now: number): CalendarItem | null {
  return nextEvents(cal, now, 8).find((it) => !it.allDay) ?? null;
}

/** Počasí pro sekci: teplota, min / max dne a poloha teploty na pruhu dne (0–1), jen z platných dat. */
export type WeatherIn = { status: string; label: string; fetchedMs?: number | null; data: { temp: number; min: number | null; max: number | null; desc: string; icon: string } | null } | null | undefined;
/** `now` zadané = počasí ≥ 3 h staré (nebo bez času stažení) se neukáže (`cards/weatherAge.ts`); bez `now` se věk nehlídá. */
export function weatherNow(w: WeatherIn, now?: number): { temp: number; min: number | null; max: number | null; desc: string; icon: string; label: string; pos: number | null } | null {
  const d = w?.data;
  if (!w || !d || !Number.isFinite(d.temp) || (w.status !== "ok" && w.status !== "stale")) return null;
  if (now !== undefined && weatherAge(w, now) === "gone") return null;
  const pos = d.min != null && d.max != null && d.max > d.min ? Math.max(0, Math.min(1, (d.temp - d.min) / (d.max - d.min))) : null;
  return { temp: d.temp, min: d.min, max: d.max, desc: d.desc, icon: d.icon, label: w.label, pos };
}

/** Kolik událostí má kalendář v paměti v den `date` (YYYY-MM-DD) → počet teček pod dnem (nejvýš `max`). */
export function dotsOf(cal: CalendarView | null | undefined, date: string, max = 3): number {
  const day = cal?.days.find((d) => d.date === date);
  return Math.min(max, day?.items.length ?? 0);
}
