// Vzhledy karty Claude (kolo 25, `CardLook.design`): čistá logika bez DOM a bez `t()` (test `claudeLooks.test.mjs`).
// Čte jen skutečná data snapshotu (limity, útrata po dnech, sezení); co chybí, vrací jako prázdný stav — nic se
// nevymýšlí. Texty skládá `ClaudeDesign.tsx` z těchto struktur.
import { level, winState, type LimitWindow } from "../limits/model.ts";
import { paceOf } from "../limits/pace.ts";
import { dayUnpriced, priceSeries } from "../costs/model.ts";
import { isToday } from "../board/model.ts";
import { orderSessions } from "../sessions/order.ts";
import type { CostDay } from "../costs/model.ts";

export type LimIn = { at: number | null; staleMs: number; fiveHour: LimitWindow | null; sevenDay: LimitWindow | null; history5h?: [number, number][] };
export type Tone = "ok" | "warn" | "bad";

export type WinInfo = { kind: "none" | "fresh" | "stale" | "reset"; pct: number | null; tone: Tone; resetsAt: number | null; ageMin: number };

/** Jedno okno limitu: procento jen z čerstvého / starého vzorku; po resetu a bez dat `pct` null. */
export function winInfo(w: LimitWindow | null | undefined, lv: Pick<LimIn, "at" | "staleMs"> | null | undefined, now: number): WinInfo {
  const st = lv ? winState(w, lv.at, now, lv.staleMs) : ({ kind: "none" } as const);
  const pct = st.kind === "fresh" || st.kind === "stale" ? st.pct : null;
  return { kind: st.kind, pct, tone: pct == null ? "ok" : level(pct), resetsAt: w?.resetsAt ?? null, ageMin: st.kind === "stale" ? st.ageMin : 0 };
}

export type PaceLine = {
  pct: number;
  /** kde bys měl být podle času (0–100) */
  plan: number;
  resetsAt: number;
  /** runs-out = při tomhle tempu dojde v `at`; lasts = vydrží do obnovy; idle = bez čerpání; calm = tempo neznáme */
  kind: "runs-out" | "lasts" | "idle" | "calm";
  at?: number;
  /** tempo %/h, které by stačilo do obnovy */
  slowTo?: number;
  perH?: number;
};

/** Tempomat 5h okna (existující výpočet `limits/pace.ts`); jen s čerstvým vzorkem a časem obnovy. */
export function paceLine(lv: LimIn | null | undefined, now: number): PaceLine | null {
  if (!lv) return null;
  const i = winInfo(lv.fiveHour, lv, now);
  if (i.kind !== "fresh" || i.pct == null || i.resetsAt == null) return null;
  const p = paceOf("5h", i.pct, i.resetsAt, lv.history5h, now);
  if (!p) return null;
  const base = { pct: i.pct, plan: p.plan, resetsAt: i.resetsAt, perH: p.rate?.perH };
  const e = p.eta;
  if (!e) return { ...base, kind: "calm" };
  if (e.kind === "runs-out") return { ...base, kind: "runs-out", at: e.at, slowTo: e.slowTo };
  return { ...base, kind: e.kind };
}

export type SpendModel = {
  state: "off" | "computing" | "none" | "ok";
  /** true = ceny v USD, false = jen tokeny (žádný den nemá cenu) */
  priced: boolean;
  /** hodnota dneška; null = dnešek není v přehledu, nebo má jen tokeny bez ceny (pak „—“, ne 0 $) */
  today: number | null;
  /** dnešek má i tokeny bez ceny → částka je dolní odhad („≥“) */
  todayLower: boolean;
  values: number[];
  /** den má tokeny, ale žádnou známou cenu (hodnota 0 v `values` není „0 $“) */
  unknown: boolean[];
  /** den má i nenacenné tokeny → částka dne je dolní odhad */
  lower: boolean[];
  dates: string[];
  /** průměr za dokončené dny okna (bez nedokončeného dneška a bez dnů s neznámou cenou, nulové dny ano) */
  avg: number | null;
  /** z kolika dní je `avg` */
  avgDays: number;
  avgLower: boolean;
  peak: number;
  peakLower: boolean;
};

type CostsIn = { state?: string; summary: { days: CostDay[] } | null } | null | undefined;

/** Útrata po dnech (posledních `n`): dnešek jen když poslední den přehledu je dnešní místní datum. Série = `costs/model.ts`
 *  `priceSeries` (jediná definice); průměr jen z dokončených dnů (N6), den s neznámou cenou není „0 $“ (N5). */
export function spendModel(costs: CostsIn, now: number, n = 14): SpendModel {
  const empty = (state: SpendModel["state"]): SpendModel => ({ state, priced: true, today: null, todayLower: false, values: [], unknown: [], lower: [], dates: [], avg: null, avgDays: 0, avgLower: false, peak: 0, peakLower: false });
  if (!costs || costs.state === "off") return empty("off");
  const days = costs.summary?.days ?? [];
  if (!costs.summary) return empty(costs.state === "computing" ? "computing" : "none");
  if (!days.length) return empty("none");
  const win = days.slice(-n);
  const { mode, values, unknown } = priceSeries(days, n);
  const last = values.length - 1;
  const fresh = isToday(costs as { summary: { days: { date: string }[] } }, now);
  const lower = win.map((d, i) => !unknown[i] && dayUnpriced(d) > 0);
  const done = values.map((_, i) => i < last + (fresh ? 0 : 1) && !unknown[i]);
  const avgDays = done.filter(Boolean).length;
  const known = values.filter((_, i) => !unknown[i]);
  return {
    state: "ok",
    priced: mode === "cost",
    today: fresh && !unknown[last] ? values[last] : null,
    todayLower: fresh && lower[last],
    values,
    unknown,
    lower,
    dates: win.map((d) => d.date),
    avg: avgDays ? values.reduce((a, v, i) => a + (done[i] ? v : 0), 0) / avgDays : null,
    avgDays,
    avgLower: lower.some((l, i) => l && done[i]),
    peak: Math.max(0, ...known),
    peakLower: lower.some(Boolean),
  };
}

type SessionLike = { id: string; status: "waiting" | "working" | "idle" | "ended"; updatedAt: number };

/** Dlaždice sezení: pořadí jako v kartě (pracující první), strop `max`, zbytek jako počet. */
export function sessionTiles<T extends SessionLike>(list: readonly T[], max: number, asked: (id: string) => boolean = () => false): { shown: T[]; more: number; working: number } {
  const all = orderSessions(list, asked);
  return { shown: all.slice(0, Math.max(0, max)), more: Math.max(0, all.length - max), working: all.filter((x) => x.status === "working").length };
}
