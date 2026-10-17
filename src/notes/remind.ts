// Čisté funkce pro výběr připomínky: lokální čas ve tvaru `YYYY-MM-DDTHH:MM` (stejný jako dřív
// z `datetime-local`; offset doplní Rust), rychlé volby, mřížka měsíce, popisky (přes `t`).
import { t } from "../i18n";
import { dayNames } from "../format";

export const pad2 = (n: number) => String(n).padStart(2, "0");
/** Zkratky dnů od pondělí (hlavička kalendáře). */
export const dnyKr = () => [
  t("notes.remind.dow.mon", "Po"),
  t("notes.remind.dow.tue", "Út"),
  t("notes.remind.dow.wed", "St"),
  t("notes.remind.dow.thu", "Čt"),
  t("notes.remind.dow.fri", "Pá"),
  t("notes.remind.dow.sat", "So"),
  t("notes.remind.dow.sun", "Ne"),
];
/** Názvy měsíců v 1. pádu (hlavička kalendáře). */
export const mesice = () => [
  t("notes.remind.month.jan", "leden"),
  t("notes.remind.month.feb", "únor"),
  t("notes.remind.month.mar", "březen"),
  t("notes.remind.month.apr", "duben"),
  t("notes.remind.month.may", "květen"),
  t("notes.remind.month.jun", "červen"),
  t("notes.remind.month.jul", "červenec"),
  t("notes.remind.month.aug", "srpen"),
  t("notes.remind.month.sep", "září"),
  t("notes.remind.month.oct", "říjen"),
  t("notes.remind.month.nov", "listopad"),
  t("notes.remind.month.dec", "prosinec"),
];

export const toLocalInput = (d: Date) => `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}T${pad2(d.getHours())}:${pad2(d.getMinutes())}`;

/** `YYYY-MM-DDTHH:MM` → Date (lokální), jinak null. */
export function parseLocal(v: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(v);
  if (!m) return null;
  const d = new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** Ruční zápis času: „9“, „9:30“, „0930“, „9.30“ → [h, m]. */
export function parseHm(s: string): [number, number] | null {
  const m = /^\s*(\d{1,2})(?:[:.]?(\d{2}))?\s*$/.exec(s);
  if (!m) return null;
  const h = +m[1];
  const min = m[2] ? +m[2] : 0;
  return h < 24 && min < 60 ? [h, min] : null;
}

const atTime = (d: Date, h: number, m = 0) => new Date(d.getFullYear(), d.getMonth(), d.getDate(), h, m);
const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n, d.getHours(), d.getMinutes());
const sameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();

/** Nejbližší celá hodina aspoň 30 min od teď. */
export function remindDefault(now = Date.now()): string {
  const d = new Date(now + 30 * 60_000);
  if (d.getMinutes() || d.getSeconds() || d.getMilliseconds()) d.setHours(d.getHours() + 1, 0, 0, 0);
  return toLocalInput(d);
}

/** Rychlé volby: za 1 h, dnes 17:00 / večer 20:00, zítra 9:00, v pondělí 9:00. */
export function quickPicks(now: Date): { label: string; value: string }[] {
  const out: { label: string; value: string }[] = [];
  const hour = new Date(Math.ceil((now.getTime() + 60 * 60_000) / 300_000) * 300_000);
  out.push({ label: t("notes.remind.inHour", "Za 1 h"), value: toLocalInput(hour) });
  const mins = now.getHours() * 60 + now.getMinutes();
  if (mins < 16 * 60 + 30) out.push({ label: t("notes.remind.today17", "Dnes 17:00"), value: toLocalInput(atTime(now, 17)) });
  else if (mins < 19 * 60 + 30) out.push({ label: t("notes.remind.tonight", "Večer 20:00"), value: toLocalInput(atTime(now, 20)) });
  const tomorrow = atTime(addDays(now, 1), 9);
  out.push({ label: t("notes.remind.tomorrow9", "Zítra 9:00"), value: toLocalInput(tomorrow) });
  const toMonday = ((8 - now.getDay()) % 7) || 7;
  const monday = atTime(addDays(now, toMonday), 9);
  if (!sameDay(monday, tomorrow)) out.push({ label: t("notes.remind.monday9", "V pondělí 9:00"), value: toLocalInput(monday) });
  return out;
}

/** 42 dní (6 týdnů) od pondělí týdne, ve kterém začíná měsíc. */
export function monthGrid(year: number, month: number): Date[] {
  const first = new Date(year, month, 1);
  const start = addDays(first, -((first.getDay() + 6) % 7));
  return Array.from({ length: 42 }, (_, i) => addDays(start, i));
}

/** Časy po 15 minutách přes celý den. */
export const TIME_SLOTS = Array.from({ length: 96 }, (_, i) => `${pad2(Math.floor(i / 4))}:${pad2((i % 4) * 15)}`);

/** Popisek výběru: „dnes 17:00“, „zítra 9:00“, „pondělí 20. 10. 9:00“. */
export function remindText(v: string, now: Date): string {
  const d = parseLocal(v);
  if (!d) return t("notes.remind.none", "Bez připomínky");
  const time = `${d.getHours()}:${pad2(d.getMinutes())}`;
  if (sameDay(d, now)) return t("notes.remind.today", "dnes {time}", { time });
  if (sameDay(d, addDays(now, 1))) return t("notes.remind.tomorrow", "zítra {time}", { time });
  const year = d.getFullYear() !== now.getFullYear() ? `${d.getFullYear()} ` : "";
  return t("notes.remind.date", "{dow} {d}. {m}. {year}{time}", { dow: dayNames()[d.getDay()], d: d.getDate(), m: d.getMonth() + 1, year, time });
}

/** Den z kalendáře se stávajícím časem (bez času: 9:00, dnes po deváté nejbližší celá hodina). */
export function withDay(v: string, day: Date, now: Date): string {
  const cur = parseLocal(v);
  if (cur) return toLocalInput(atTime(day, cur.getHours(), cur.getMinutes()));
  if (sameDay(day, now) && now.getHours() >= 9) return toLocalInput(atTime(day, now.getHours() + 1 > 23 ? 23 : now.getHours() + 1));
  return toLocalInput(atTime(day, 9));
}

/** Čas se stávajícím dnem (bez dne: dnes, a když už ten čas dnes byl, zítra). */
export function withTime(v: string, h: number, m: number, now: Date): string {
  const cur = parseLocal(v);
  if (cur) return toLocalInput(atTime(cur, h, m));
  const today = atTime(now, h, m);
  return toLocalInput(today.getTime() > now.getTime() ? today : addDays(today, 1));
}

/** ISO s offsetem z Rustu → lokální `YYYY-MM-DDTHH:MM` pro výběr. */
export const fromMs = (ms: number | null) => (ms == null ? "" : toLocalInput(new Date(ms)));
