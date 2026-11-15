// Jedna společná priorita „co je teď důležité“ (H1, k34). Dřív tři nezávislé tabulky: ostrůvek (`model.ts`
// `islandEvent`), pruh návrhů (`suggest/model.ts` `pick`) a proužek (Rust `attention.rs` `Event::rank`).
// Tady je pořadí jednou; ostrůvek ho používá přímo (`topOf`), pruh a proužek ho nesou jako zrcadlo a hlídá je
// test `priority.test.mjs` (pořadí `pick` i `Event::rank` musí být podposloupnost téhle tabulky).
// Čistá logika bez DOM a bez `t()`; data jen ze snapshotu (žádný nový fetch).
import type { CalendarView } from "../types/calendar";
import type { LimitsView } from "../types/claude";
import { FULL_PCT } from "../charts/thresholds.ts";
import { WIN5_MS, eta5h, rate5h } from "../limits/pace.ts";
import type { SuggestKind } from "../suggest/model.ts";

/** Druhy dění od nejdůležitějšího: schválení > otázka > zadání čeká na mě (`brief`, chip `briefWaiting`) > schůzka (≤ N min / probíhá) >
 *  limit blízko vyčerpání > splatná připomínka > úkoly Dnes > poznámka ze schůzky > Claude od posledně (`since`, chip `claudeSince`; jen pruh) >
 *  ranní shrnutí (`morning`, jen pruh) > spadlé CI > PR k revizi > dev server > adresa ve schránce > hotovo > časovač > hraje hudba >
 *  Claude pracuje > pozastavená hudba > klid. Každý druh chipu pruhu (`SuggestKind`) má řádek tady (`KIND_PRIO`, typ vynutí tsc). */
export const PRIORITY = ["approval", "question", "brief", "meeting", "limit", "reminder", "today", "meetingNote", "since", "morning", "ci", "review", "dev", "clip", "done", "timer", "music", "working", "musicPaused", "idle"] as const;
export type Prio = (typeof PRIORITY)[number];

/** Řádek tabulky pro každý druh chipu, který umí `pick` vrátit (L175: dřív reminder / today / meetingNote / dev / clip chyběly a test je přeskočil). */
export const KIND_PRIO: Record<SuggestKind, Prio> = {
  briefWaiting: "brief",
  meeting: "meeting",
  reminder: "reminder",
  today: "today",
  meetingNote: "meetingNote",
  claudeSince: "since",
  morning: "morning",
  ghCi: "ci",
  ghReview: "review",
  dev: "dev",
  clip: "clip",
};

/** Pořadí druhu (nižší = důležitější). */
export const rankOf = (k: Prio): number => PRIORITY.indexOf(k);

/** Nejdůležitější z kandidátů (při shodě první); prázdný seznam = null. */
export function topOf<T extends { prio: Prio }>(c: readonly T[]): T | null {
  let best: T | null = null;
  for (const x of c) if (!best || rankOf(x.prio) < rankOf(best.prio)) best = x;
  return best;
}

/** Schůzka je „teď“ od tolika ms před začátkem do konce (stejně jako odkaz Připojit v pruhu návrhů). */
export const MEETING_LEAD_MS = 10 * 60_000;
/** Limit 5h je „blízko“: od tohoto procenta, nebo když tempo dojde nejpozději za tolik ms před obnovou. */
export const LIMIT_PCT = FULL_PCT;
export const LIMIT_ETA_MS = 30 * 60_000;

export interface MeetingNow {
  title: string;
  start: number;
  end: number;
  /** už začala */
  ongoing: boolean;
  join: { url: string; provider: string } | null;
}

/** Nejbližší dnešní schůzka (bez celodenních), která začíná za ≤ `MEETING_LEAD_MS` nebo právě běží. */
export function meetingNow(cal: Pick<CalendarView, "days"> | null | undefined, now: number): MeetingNow | null {
  let best: MeetingNow | null = null;
  for (const it of cal?.days.find((d) => d.kind === "today")?.items ?? []) {
    if (it.allDay) continue;
    const end = Math.max(it.end, it.start + 60_000);
    if (now < it.start - MEETING_LEAD_MS || now >= end) continue;
    if (!best || it.start < best.start) best = { title: it.title, start: it.start, end, ongoing: now >= it.start, join: it.join?.url ? { url: it.join.url, provider: it.join.provider } : null };
  }
  return best;
}

export interface LimitNear {
  pct: number;
  resetsAt: number | null;
  /** kdy podle tempa z posledních vzorků dojde 100 % (jen když je to před obnovou); jinak null */
  etaAt: number | null;
}

/** Minimální počet vzorků historie, od kterého se tempu věří (jinak průměr okna = divadlo). */
export const ETA_MIN_POINTS = 3;

/** Čas dosažení 100 % z tempa posledních 30 min (aspoň `ETA_MIN_POINTS` vzorků, `rate5h` „recent“); jinak null. */
export function etaAtOf(hist: [number, number][] | undefined, used: number, resetsAt: number | null | undefined, now: number): number | null {
  if (resetsAt == null || resetsAt <= now || resetsAt - now > WIN5_MS) return null;
  const inWin = (hist ?? []).filter(([at]) => at >= now - 30 * 60_000 && at <= now);
  if (inWin.length < ETA_MIN_POINTS) return null;
  const r = rate5h(hist, used, resetsAt, now);
  if (!r || r.src !== "recent") return null;
  const e = eta5h(r, used, resetsAt, now);
  return e.kind === "runs-out" ? e.at : null;
}

/** Limit 5h z čerstvého vzorku, který je blízko vyčerpání (≥ `LIMIT_PCT` %, nebo dojde do `LIMIT_ETA_MS`); jinak null. */
export function limitNear(l: Pick<LimitsView, "at" | "staleMs" | "fiveHour" | "history5h"> | undefined, now: number): LimitNear | null {
  const w = l?.fiveHour;
  if (!l || !w || l.at == null || now - l.at > l.staleMs) return null;
  if (w.resetsAt != null && w.resetsAt <= now) return null;
  const etaAt = etaAtOf(l.history5h, w.pct, w.resetsAt, now);
  const soon = etaAt != null && etaAt - now <= LIMIT_ETA_MS;
  return w.pct >= LIMIT_PCT || soon ? { pct: w.pct, resetsAt: w.resetsAt, etaAt } : null;
}
