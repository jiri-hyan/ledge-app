// H5 ranní shrnutí: jeden chip při prvním otevření šuplíku po dlouhé nečinnosti (spánek, noc). Čistá logika bez DOM a `t()`
// (test `morning.test.mjs`). Jen složení dat, která Ledge už má ve snapshotu — kalendář dnes, úkoly a připomínky z poznámek,
// zadání čekající na mě, PR / CI z GitHubu, týdenní limit Claude. Co chybí nebo je nula, se nevypíše; žádná část = žádný chip.
// Žádný časovač: počítá se v `pick` při změně snapshotu a na hranicích `nextChange` (konec schůzky, konec ranního okna).

/** Nečinnost, po které je otevření „první po probuzení“: šuplík se naposledy schoval před aspoň tolika ms (nebo od startu nikdy). */
export const MORNING_GAP_MS = 6 * 3600_000;
/** Ranní okno v místním čase [od, do) hodin; mimo něj se shrnutí nenabízí (po poledni už není „ráno“). */
export const MORNING_FROM_H = 5;
export const MORNING_TO_H = 13;
/** Týdenní limit Claude se zmíní od tohoto procenta (pod ním není co hlásit). */
export const MORNING_WEEK_PCT = 50;

export interface MorningInput {
  /** `Snapshot.drawerHiddenAt`: kdy se šuplík naposledy schoval; null = od startu Ledge nikdy */
  hiddenAt: number | null;
  /** dnešní schůzky bez celodenních (`start` / `end` v ms) */
  agenda: { start: number; end: number }[];
  /** `Snapshot.limits` (jen to, co čteme); chybí = Claude vypnutý */
  limits?: { at: number | null; staleMs: number; sevenDay: { pct: number; resetsAt: number | null } | null } | null;
}

/** Počty z ostatních zdrojů `SuggestInput` (už ověřené: GitHub jen čerstvý a zapnutý). */
export interface MorningCounts {
  tasks: { overdue: number; today: number };
  reminders: number;
  briefs: number;
  review: number;
  failing: number;
}

export type MorningPart =
  | { k: "meetings"; n: number; first: number }
  | { k: "overdue" | "tasks" | "reminders" | "briefs" | "review" | "ci" | "week"; n: number };

const pad = (n: number): string => String(n).padStart(2, "0");

/** Klíč zavření: jednou denně (místní datum). */
export function morningKey(now: number): string {
  const d = new Date(now);
  return `morning:${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Konec ranního okna dne, do kterého patří `now` (místní `MORNING_TO_H`); je-li už po něm, příštího dne. */
export function morningWindowEnd(now: number): number {
  const d = new Date(now);
  const end = new Date(d.getFullYear(), d.getMonth(), d.getDate(), MORNING_TO_H).getTime();
  return end > now ? end : new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1, MORNING_TO_H).getTime();
}

/** Je teď první otevření po dlouhé nečinnosti v ranním okně? */
export function isMorningOpen(hiddenAt: number | null, now: number): boolean {
  const h = new Date(now).getHours();
  if (h < MORNING_FROM_H || h >= MORNING_TO_H) return false;
  return hiddenAt === null || now - hiddenAt >= MORNING_GAP_MS;
}

/** Části shrnutí v pořadí: schůzky · po termínu · úkoly na dnes · připomínky · zadání · PR · CI · týdenní limit. Prázdné = bez chipu. */
export function morningParts(mi: MorningInput, c: MorningCounts, now: number): MorningPart[] {
  if (!isMorningOpen(mi.hiddenAt, now)) return [];
  const out: MorningPart[] = [];
  const left = mi.agenda.filter((a) => a.end > now);
  if (left.length) out.push({ k: "meetings", n: left.length, first: Math.min(...left.map((a) => a.start)) });
  const add = (k: Exclude<MorningPart["k"], "meetings">, n: number) => {
    if (n > 0) out.push({ k, n });
  };
  add("overdue", c.tasks.overdue);
  add("tasks", c.tasks.today);
  add("reminders", c.reminders);
  add("briefs", c.briefs);
  add("review", c.review);
  add("ci", c.failing);
  const l = mi.limits;
  const w = l?.sevenDay;
  const fresh = !!l && !!w && l.at !== null && now - l.at <= l.staleMs && (w.resetsAt === null || w.resetsAt > now);
  if (fresh && w && w.pct >= MORNING_WEEK_PCT) add("week", Math.round(w.pct));
  return out;
}

/** Podpis částí pro `sameSuggestions` (bez zbytečného překreslení). */
export const morningSig = (p: readonly MorningPart[] | undefined): string => (p ?? []).map((x) => `${x.k}:${x.n}${x.k === "meetings" ? `@${x.first}` : ""}`).join("|");
