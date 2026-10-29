// Čistá logika vzhledů karty Týden (kolo 25) — bez DOM a bez `t()`, test `weekModel.test.mjs`. Data jen z kalendáře,
// který Ledge už má v paměti (`CalendarView`: dnes / zítra / později); co kalendář neposlal, tu není (tečky v měsíci
// svítí jen u dnů, které kalendář vrací). Časy jsou místní čas počítače, jako u pásu dnů v `model.ts`.
import type { CalendarItem, CalendarView } from "../types/calendar.ts";
import { localDate } from "./model.ts";

/** Minuty od místní půlnoci dne `ms`. */
export function dayMinutes(ms: number): number {
  const d = new Date(ms);
  return d.getHours() * 60 + d.getMinutes();
}

/** Dnešní události s časem (bez celodenních), seřazené podle začátku. */
export function timedToday(cal: CalendarView | null | undefined): CalendarItem[] {
  const today = cal?.days.find((x) => x.kind === "today");
  return (today?.items ?? []).filter((it) => !it.allDay).sort((a, b) => a.start - b.start);
}

/** Dnešní celodenní události. */
export const allDayToday = (cal: CalendarView | null | undefined): CalendarItem[] =>
  (cal?.days.find((x) => x.kind === "today")?.items ?? []).filter((it) => it.allDay);

/** Kolik dnešních událostí ještě neskončilo (celodenní se počítají). */
export function remainingToday(cal: CalendarView | null | undefined, now: number): number {
  return (cal?.days.find((x) => x.kind === "today")?.items ?? []).filter((it) => it.allDay || it.end > now).length;
}

export interface Block {
  item: CalendarItem;
  /** 0–100 % šířky osy */
  left: number;
  width: number;
  /** řádek, do kterého se blok vejde bez překrytí (0 = první) */
  lane: number;
}

/** Bloky událostí na ose `fromH`–`toH` hodin dneška. Co na osu nezasahuje, se vynechá (a počítá se v `outside`). */
export function timelineBlocks(items: readonly CalendarItem[], fromH = 7, toH = 22): { blocks: Block[]; outside: number; lanes: number } {
  const [lo, hi] = [fromH * 60, toH * 60];
  const blocks: Block[] = [];
  const laneEnd: number[] = [];
  let outside = 0;
  for (const item of [...items].sort((a, b) => a.start - b.start)) {
    const sm = dayMinutes(item.start);
    const s = Math.max(lo, sm);
    const e = Math.min(hi, sm + Math.round((item.end - item.start) / 60_000));
    if (e <= s) {
      outside++;
      continue;
    }
    let lane = laneEnd.findIndex((end) => end <= s);
    if (lane < 0) lane = laneEnd.length;
    laneEnd[lane] = e;
    blocks.push({ item, left: ((s - lo) / (hi - lo)) * 100, width: Math.max(1.5, ((e - s) / (hi - lo)) * 100), lane });
  }
  return { blocks, outside, lanes: Math.max(1, laneEnd.length) };
}

/** Poloha ryska „teď“ na ose 0–100 %, nebo null mimo osu. */
export function nowPos(now: number, fromH = 7, toH = 22): number | null {
  const m = dayMinutes(now);
  return m < fromH * 60 || m > toH * 60 ? null : ((m - fromH * 60) / ((toH - fromH) * 60)) * 100;
}

/** Úhel (0° = nahoře, po směru hodin) pro minutu dne na 24h ciferníku (půlnoc nahoře). */
export const dialAngle = (minutes: number): number => (minutes / 1440) * 360;

/** Bod na kružnici; úhel od svislice po směru hodin. */
export function polar(cx: number, cy: number, r: number, deg: number): { x: number; y: number } {
  const a = ((deg - 90) * Math.PI) / 180;
  return { x: cx + r * Math.cos(a), y: cy + r * Math.sin(a) };
}

/** SVG cesta výseče mezi dvěma úhly (prstenec tloušťky `r1`–`r0`). */
export function wedgePath(cx: number, cy: number, r0: number, r1: number, a0: number, a1: number): string {
  const end = Math.max(a1, a0 + 1.5);
  const large = end - a0 > 180 ? 1 : 0;
  const [p0, p1, q1, q0] = [polar(cx, cy, r1, a0), polar(cx, cy, r1, end), polar(cx, cy, r0, end), polar(cx, cy, r0, a0)];
  const f = (n: number) => n.toFixed(2);
  return `M${f(p0.x)} ${f(p0.y)} A${r1} ${r1} 0 ${large} 1 ${f(p1.x)} ${f(p1.y)} L${f(q1.x)} ${f(q1.y)} A${r0} ${r0} 0 ${large} 0 ${f(q0.x)} ${f(q0.y)} Z`;
}

export type Countdown = { kind: "now" } | { kind: "min" | "hour" | "day"; n: number };

/** Odpočet k začátku události: probíhá / za N min / za N h / za N dní. Minulé = „probíhá“ (ten, kdo volá, filtruje konec). */
export function countdown(start: number, now: number): Countdown {
  const min = Math.ceil((start - now) / 60_000);
  if (min <= 0) return { kind: "now" };
  if (min < 60) return { kind: "min", n: min };
  if (min < 24 * 60) return { kind: "hour", n: Math.floor(min / 60) };
  return { kind: "day", n: Math.floor(min / 1440) };
}

/** Nejbližší neskončené události napříč dny kalendáře (probíhající první), nejvýš `max`. */
export function nextEvents(cal: CalendarView | null | undefined, now: number, max = 4): CalendarItem[] {
  const all = (cal?.days ?? []).flatMap((d) => d.items).filter((it) => it.end > now && !(it.allDay && it.start > now + 24 * 3600_000));
  return all.sort((a, b) => Number(b.ongoing) - Number(a.ongoing) || Number(a.allDay) - Number(b.allDay) || a.start - b.start).slice(0, max);
}

export interface MonthCell {
  date: string;
  day: number;
  inMonth: boolean;
  today: boolean;
  weekend: boolean;
  busy: boolean;
}

/** Měsíční mřížka od pondělí: celé týdny, které pokrývají měsíc `now` (4–6 řádků po 7). */
export function monthGrid(now: number, cal?: CalendarView | null): MonthCell[][] {
  const d = new Date(now);
  const first = new Date(d.getFullYear(), d.getMonth(), 1);
  const last = new Date(d.getFullYear(), d.getMonth() + 1, 0);
  const start = new Date(first.getFullYear(), first.getMonth(), 1 - ((first.getDay() + 6) % 7));
  const weeks = Math.ceil(((last.getTime() - start.getTime()) / 86_400_000 + 1) / 7);
  const busy = new Set((cal?.days ?? []).filter((x) => x.items.length > 0).map((x) => x.date));
  const today = localDate(now);
  return Array.from({ length: weeks }, (_, w) =>
    Array.from({ length: 7 }, (_, i) => {
      const x = new Date(start.getFullYear(), start.getMonth(), start.getDate() + w * 7 + i);
      const date = localDate(x.getTime());
      return { date, day: x.getDate(), inMonth: x.getMonth() === d.getMonth(), today: date === today, weekend: x.getDay() === 0 || x.getDay() === 6, busy: busy.has(date) };
    }),
  );
}
