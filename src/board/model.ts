// Nové widgety kola 22 (karty Claude dnes, Týden, Velké hodiny, Rychlé přepínače) — čistá logika bez DOM a bez
// `t()` (test `model.test.mjs`). Každé číslo má zdroj, který Ledge už čte: útrata a časová osa z indexu transcriptů
// (`costslive` → `Snapshot.costs`, ODHAD v cenách API = „odvozené“), kalendář z cache (`calendar::live`), hlasitost
// z Core Audio (`volume.rs`, „oficiální“ = hlásí ho systém). Co zdroj nemá, je prázdné — nic se nedopočítává.
import type { CostDay, CostHourRow, CostsView } from "../costs/model.ts";
import type { CalendarItem, CalendarView } from "../types/calendar.ts";

/** Původ čísla (vzor Codenotch): hlásí ho přímo zdroj (systém, Claude Code), nebo ho Ledge odvodil / odhadl. */
export type Origin = "official" | "derived";

/** Místní datum `YYYY-MM-DD` z ms. */
export function localDate(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

// ---------- Claude dnes ----------

export type HourRow = CostHourRow;

/** Posledních `n` dní ze sloupců útraty (nejstarší první); chybí-li data, prázdné pole. */
export function lastDays(days: CostDay[] | undefined, n: number): CostDay[] {
  return (days ?? []).slice(-n);
}

/** Součet dní: tokeny vždy, cena jen když ji aspoň jeden den má (jinak null = „—“, ne 0 $). */
export function sumDays(days: CostDay[]): { tokens: number; cost: number | null } {
  const priced = days.filter((d) => d.cost != null);
  return { tokens: days.reduce((a, d) => a + d.tokens, 0), cost: priced.length ? priced.reduce((a, d) => a + (d.cost ?? 0), 0) : null };
}

/** Je přehled ze dneška? (poslední den sloupců = dnešní místní datum). Starý = časová osa patří jinému dni. */
export function isToday(v: { summary?: { days: readonly { date: string }[] } | null } | null | undefined, now: number): boolean {
  const days = v?.summary?.days ?? [];
  const last = days[days.length - 1]?.date;
  return !!last && last === localDate(now);
}

/** Řádky časové osy z přehledu (starý snapshot bez pole = žádné). */
export function hourRows(v: CostsView | null | undefined): HourRow[] {
  const h = v?.summary?.hours;
  return Array.isArray(h) ? h.filter((r) => Array.isArray(r.tokens) && r.tokens.length === 24) : [];
}

/** Úroveň buňky časové osy 0–4: 0 = v té hodině žádná odpověď, jinak podle podílu na nejaktivnější hodině. */
export function heat(v: number, max: number): number {
  if (!(v > 0) || !(max > 0)) return 0;
  return Math.min(4, 1 + Math.floor((3 * v) / max));
}

/** Nejvyšší hodinová hodnota přes všechny řádky (společná škála). */
export const hoursMax = (rows: HourRow[]) => Math.max(0, ...rows.flatMap((r) => r.tokens));

/** Rozsah hodin, který má smysl ukázat: od první aktivní (nejpozději 8) do aktuální / poslední aktivní. */
export function hourSpan(rows: HourRow[], nowHour: number): [number, number] {
  let first = 24;
  let last = -1;
  for (const r of rows)
    r.tokens.forEach((t, i) => {
      if (t > 0) {
        first = Math.min(first, i);
        last = Math.max(last, i);
      }
    });
  const from = Math.min(first, 8, nowHour);
  const to = Math.max(last, nowHour);
  return [Math.max(0, from), Math.min(23, to)];
}

/** Výšky sloupců 0–1 podle ceny (bez ceny podle tokenů) — stejné pravidlo jako sekce v kartě Claude. */
export function barHeights(days: CostDay[]): number[] {
  const priced = days.some((d) => d.cost != null && d.cost > 0);
  const val = (d: CostDay) => (priced ? d.cost ?? 0 : d.tokens);
  const max = Math.max(0, ...days.map(val));
  return days.map((d) => (max > 0 ? val(d) / max : 0));
}

/** Stav karty Claude dnes podle přehledu: vypnuto, počítá se poprvé, chyba, hotovo. */
export function usageState(v: CostsView | null | undefined): "off" | "first" | "error" | "ok" {
  if (!v || v.state === "off") return "off";
  if (v.summary) return "ok";
  if (v.state === "error") return "error";
  return "first";
}

// ---------- Týden ----------

export interface WeekDay {
  date: string;
  /** den v měsíci */
  day: number;
  /** 0 = neděle … 6 = sobota (`Date.getDay`) */
  dow: number;
  today: boolean;
  weekend: boolean;
  /** v ten den je v kalendáři aspoň jedna událost (jen dny, které kalendář posílá) */
  busy: boolean;
}

/** Týden od pondělí, který obsahuje `now`; `busy` z dnů kalendáře (dnes / zítra / později — víc Ledge nedrží). */
export function weekStrip(now: number, cal?: CalendarView | null): WeekDay[] {
  const d = new Date(now);
  const monday = new Date(d.getFullYear(), d.getMonth(), d.getDate() - ((d.getDay() + 6) % 7));
  const busy = new Set((cal?.days ?? []).filter((x) => x.items.length > 0).map((x) => x.date));
  const today = localDate(now);
  return Array.from({ length: 7 }, (_, i) => {
    const x = new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + i);
    const date = localDate(x.getTime());
    return { date, day: x.getDate(), dow: x.getDay(), today: date === today, weekend: x.getDay() === 0 || x.getDay() === 6, busy: busy.has(date) };
  });
}

/** Co pás řekne o kalendáři: žádný zdroj, načítá se, nedostupný, nebo data. */
export function calState(cal?: CalendarView | null): "none" | "loading" | "unavailable" | "ok" {
  if (!cal || cal.status === "off" || cal.status === "unset") return "none";
  if (cal.status === "unavailable") return "unavailable";
  if (cal.status === "loading" || cal.status === "idle") return cal.days.length ? "ok" : "loading";
  return "ok";
}

/** Dnešní události, které ještě neskončily (celodenní první), nejvýš `max`. */
export function todayLeft(cal: CalendarView | null | undefined, now: number, max = 3): CalendarItem[] {
  const today = cal?.days.find((x) => x.kind === "today");
  const items = (today?.items ?? []).filter((it) => it.allDay || it.end > now);
  return [...items].sort((a, b) => Number(b.allDay) - Number(a.allDay) || a.start - b.start).slice(0, max);
}

// ---------- Rychlé přepínače ----------

/** Krok hlasitosti tlačítky − / + (procenta, oříznuto 0–100). */
export function stepVolume(level: number, delta: number): number {
  const v = Math.round((Number.isFinite(level) ? level : 0) + delta);
  return Math.max(0, Math.min(100, v));
}
