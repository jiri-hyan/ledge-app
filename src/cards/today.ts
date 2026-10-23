// Karta Dnes: čistá logika (bez DOM, Solidu a `t()`), test `today.test.mjs`. Řádky se skládají VÝHRADNĚ z dat,
// která snapshot už má — nic se nestahuje a nic se nevymýšlí: další schůzka dnes (kalendář), připomínky do konce
// dneška (poznámky), PR čekající na mou revizi (paměť GitHubu) a reset 5h limitu Claude (jen od 50 %).
// Řádek bez dat se nekreslí; bez jediného řádku se nekreslí ani karta (`Facts.today` v `layout/model.ts`).
import { joinable } from "../calendar/model.ts";
import { winState } from "../limits/model.ts";
import { GH_FRESH_MS } from "../suggest/model.ts";
import type { CalendarJoin, CalendarView } from "../types/calendar";
import type { LimitsView } from "../types/claude";
import type { NoteView } from "../types/notes";
import type { PrRef } from "../types/suggest";

/** Limit 5h se na kartě ukáže od téhle spotřeby (pod ní reset nikoho nezajímá). */
export const LIMIT_FROM_PCT = 50;
/** Nejvýš tolik připomínek na kartě; zbytek jako „+N“. */
export const MAX_REMINDERS = 3;

export interface TodayInput {
  calendar: CalendarView | null | undefined;
  /** `Snapshot.notes` (nehotové připomínky do konce dneška, pak nejnovější) */
  notes: readonly Pick<NoteView, "path" | "title" | "remindMs" | "done">[];
  /** `Snapshot.reminders` (první připomínky po termínu) */
  reminders: readonly Pick<NoteView, "path" | "title" | "remindMs" | "done">[];
  github: { review: readonly PrRef[]; fetchedMs: number | null } | null | undefined;
  limits: LimitsView | null | undefined;
}

export type TodayRow =
  | { kind: "meeting"; key: string; title: string; start: number; end: number; ongoing: boolean; join: CalendarJoin | null }
  | { kind: "reminder"; key: string; path: string; title: string; remindMs: number; overdue: boolean }
  | { kind: "reminderMore"; key: string; n: number }
  | { kind: "review"; key: string; repo: string; number: number; title: string; url: string; more: number }
  | { kind: "limit"; key: string; pct: number; resetsAt: number };

/** Konec dnešního dne v místním čase (půlnoc následujícího dne, ms). */
export function endOfDay(now: number): number {
  const d = new Date(now);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1).getTime();
}

/** Další schůzka dnes: první necelodenní událost dnešního dne, která ještě neskončila (probíhající se počítá). */
export function nextMeeting(cal: CalendarView | null | undefined, now: number): TodayRow | null {
  const today = cal?.days.find((d) => d.kind === "today");
  const it = today?.items.filter((i) => !i.allDay && now < Math.max(i.end, i.start + 60_000)).sort((a, b) => a.start - b.start)[0];
  if (!it) return null;
  return { kind: "meeting", key: `meet:${it.start}:${it.title}`, title: it.title, start: it.start, end: it.end, ongoing: it.start <= now && now < it.end, join: joinable(it, now) ? (it.join ?? null) : null };
}

/** Připomínky do konce dneška (i po termínu), nejstarší první; sloučené z `notes` a `reminders` podle cesty. */
export function dueReminders(input: Pick<TodayInput, "notes" | "reminders">, now: number): TodayRow[] {
  const end = endOfDay(now);
  const seen = new Set<string>();
  const out: Extract<TodayRow, { kind: "reminder" }>[] = [];
  for (const n of [...input.reminders, ...input.notes]) {
    if (n.done || n.remindMs == null || n.remindMs >= end || seen.has(n.path)) continue;
    seen.add(n.path);
    out.push({ kind: "reminder", key: `rem:${n.path}|${n.remindMs}`, path: n.path, title: n.title, remindMs: n.remindMs, overdue: n.remindMs <= now });
  }
  out.sort((a, b) => a.remindMs - b.remindMs);
  const rows: TodayRow[] = out.slice(0, MAX_REMINDERS);
  if (out.length > MAX_REMINDERS) rows.push({ kind: "reminderMore", key: "rem-more", n: out.length - MAX_REMINDERS });
  return rows;
}

/** PR čekající na mou revizi z paměti GitHubu (čerstvá data; jinak nic). */
export function reviewRow(gh: TodayInput["github"], now: number): TodayRow | null {
  if (!gh || gh.fetchedMs === null || now - gh.fetchedMs > GH_FRESH_MS || now < gh.fetchedMs - 60_000) return null;
  const first = gh.review[0];
  if (!first) return null;
  return { kind: "review", key: `rev:${first.repo}#${first.number}`, repo: first.repo, number: first.number, title: first.title, url: first.url, more: gh.review.length - 1 };
}

/** Reset 5h limitu: jen čerstvý vzorek od `LIMIT_FROM_PCT` s časem resetu v budoucnu. */
export function limitRow(lv: LimitsView | null | undefined, now: number): TodayRow | null {
  const w = lv?.fiveHour;
  if (!lv || !w || w.resetsAt == null) return null;
  const st = winState(w, lv.at, now, lv.staleMs);
  if (st.kind !== "fresh" || st.pct < LIMIT_FROM_PCT) return null;
  return { kind: "limit", key: "limit5h", pct: st.pct, resetsAt: w.resetsAt };
}

/** Řádky karty v pořadí schůzka · připomínky · PR · limit; prázdné pole = nic k ukázání. */
export function todayRows(input: TodayInput, now: number): TodayRow[] {
  return [nextMeeting(input.calendar, now), ...dueReminders(input, now), reviewRow(input.github, now), limitRow(input.limits, now)].filter((r): r is TodayRow => r !== null);
}

/** Stejné řádky (podle obsahu)? Tik času nesmí kartu každou sekundu překreslit. */
export const sameRows = (a: readonly TodayRow[], b: readonly TodayRow[]): boolean => JSON.stringify(a) === JSON.stringify(b);
