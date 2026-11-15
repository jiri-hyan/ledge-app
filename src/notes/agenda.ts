// Záložka „Úkoly“ (`TasksView.tsx`): čisté skládání sekcí z řádků `note_tasks` a kalendáře — bez DOM, Solidu a `t()`,
// test `agenda.test.mjs`. Sekce: Po termínu · Dnes (schůzky + body podle času, celodenní nahoře) · Čeká na tebe
// (`[~]`) · Brzy (budoucí termíny) · Bez termínu (max `UNDATED_MAX`, zbytek „+N“). Bod předaný propojené aplikaci
// (`forwarded`) se nepočítá a nekreslí. Celodenní bod je po termínu až od půlnoci (Rust mu dává 9:00).
import { joinable } from "../calendar/model.ts";
import type { CalendarItem, CalendarView } from "../types/calendar";
import type { TaskRow } from "../types/notes";

export const UNDATED_MAX = 20;
export const SOON_MAX = 20;
const DAY = 86_400_000;

/** Konec dnešního dne v místním čase (půlnoc následujícího dne, ms). */
export function endOfDay(now: number): number {
  const d = new Date(now);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1).getTime();
}

export type AgendaItem =
  | { kind: "meeting"; key: string; title: string; start: number; end: number; allDay: boolean; ongoing: boolean; join: CalendarItem["join"] | null }
  | { kind: "task"; key: string; row: TaskRow };

export interface Agenda {
  overdue: TaskRow[];
  today: AgendaItem[];
  waiting: TaskRow[];
  soon: TaskRow[];
  soonMore: number;
  undated: TaskRow[];
  undatedMore: number;
  /** nic k ukázání (bez bodů a bez dnešních schůzek) */
  empty: boolean;
}

export const taskKey = (r: Pick<TaskRow, "path" | "line">) => `${r.path}:${r.line}`;
const startOfDay = (ms: number): number => {
  const d = new Date(ms);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
};

/** Po termínu: s časem od chvíle termínu, celodenní až od půlnoci dne termínu (`dueMs` je 9:00). */
export function isOverdue(r: Pick<TaskRow, "dueMs" | "allDay">, now: number): boolean {
  if (r.dueMs == null) return false;
  return r.allDay ? r.dueMs < startOfDay(now) : r.dueMs < now;
}

/** Rozdíl kalendářních dní (místní čas): 0 dnes, 1 zítra, -1 včera. */
export function dayDiff(ms: number, now: number): number {
  return Math.round((startOfDay(ms) - startOfDay(now)) / DAY);
}

const meetingsOf = (cal: CalendarView | null | undefined, now: number): AgendaItem[] => {
  if (!cal || (cal.status !== "ok" && cal.status !== "stale")) return [];
  const day = cal.days.find((d) => d.kind === "today");
  return (day?.items ?? [])
    .filter((i) => i.allDay || now < Math.max(i.end, i.start + 60_000))
    .map((i): AgendaItem => ({ kind: "meeting", key: `meet:${i.start}:${i.title}`, title: i.title, start: i.start, end: i.end, allDay: i.allDay, ongoing: !i.allDay && i.start <= now && now < i.end, join: joinable(i, now) ? i.join ?? null : null }));
};

const timeOf = (a: AgendaItem): number => (a.kind === "meeting" ? (a.allDay ? -Infinity : a.start) : a.row.allDay ? -Infinity : (a.row.dueMs ?? 0));

/** Sekce záložky Úkoly. `rows` už přišly seřazené z Rustu (termín, `[~]`, bez termínu); tady se jen rozdělí. */
export function buildAgenda(rows: readonly TaskRow[], calendar: CalendarView | null | undefined, now: number): Agenda {
  const open = rows.filter((r) => !r.forwarded);
  const eod = endOfDay(now);
  const overdue: TaskRow[] = [];
  const today: AgendaItem[] = meetingsOf(calendar, now);
  const waiting: TaskRow[] = [];
  const soonAll: TaskRow[] = [];
  const undatedAll: TaskRow[] = [];
  for (const r of open) {
    if (r.state === "proposed") waiting.push(r);
    else if (r.dueMs == null) undatedAll.push(r);
    else if (isOverdue(r, now)) overdue.push(r);
    else if (r.dueMs < eod) today.push({ kind: "task", key: taskKey(r), row: r });
    else soonAll.push(r);
  }
  const byDue = (a: TaskRow, b: TaskRow) => (a.dueMs ?? 0) - (b.dueMs ?? 0) || b.prio - a.prio;
  overdue.sort(byDue);
  soonAll.sort(byDue);
  // stabilní: shodný čas → schůzka před bodem (schůzky jsou v poli první)
  today.sort((a, b) => timeOf(a) - timeOf(b) || 0);
  const soon = soonAll.slice(0, SOON_MAX);
  const undated = undatedAll.slice(0, UNDATED_MAX);
  return {
    overdue, today, waiting, soon, soonMore: soonAll.length - soon.length, undated, undatedMore: undatedAll.length - undated.length,
    empty: !overdue.length && !today.length && !waiting.length && !soonAll.length && !undatedAll.length,
  };
}

const pad = (n: number) => String(n).padStart(2, "0");
const dateIso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** Nový termín po „Odložit“: `tomorrow` = zítřek, `monday` = nejbližší pondělí po dnešku. Čas dne zůstává (celodenní zůstane celodenní). */
export function snoozeIso(r: Pick<TaskRow, "dueMs" | "allDay">, to: "tomorrow" | "monday", now: number): string {
  const n = new Date(now);
  const add = to === "tomorrow" ? 1 : ((8 - n.getDay()) % 7) || 7;
  const day = dateIso(new Date(n.getFullYear(), n.getMonth(), n.getDate() + add));
  if (r.allDay || r.dueMs == null) return day;
  const d = new Date(r.dueMs);
  return `${day} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Posun kurzoru na začátek řádku `line` (od 0) v textu — `\n` i `\r\n`; za koncem textu = konec. */
export function offsetOfLine(text: string, line: number): number {
  let off = 0;
  for (let i = 0; i < line; i++) {
    const nl = text.indexOf("\n", off);
    if (nl < 0) return text.length;
    off = nl + 1;
  }
  return off;
}
