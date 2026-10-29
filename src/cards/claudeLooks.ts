// Vzhledy karty Claude (kolo 25, `CardLook.design`): čistá logika bez DOM a bez `t()` (test `claudeLooks.test.mjs`).
// Čte jen skutečná data snapshotu (limity, útrata po dnech, sezení); co chybí, vrací jako prázdný stav — nic se
// nevymýšlí. Texty skládá `ClaudeDesign.tsx` z těchto struktur.
import { level, winState, type LimitWindow } from "../limits/model.ts";
import { paceOf } from "../limits/pace.ts";
import { costSeries } from "../layout/hero.ts";
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
  today: number | null;
  values: number[];
  dates: string[];
  avg: number | null;
  peak: number;
};

type CostsIn = { state?: string; summary: { days: CostDay[] } | null } | null | undefined;

/** Útrata po dnech (posledních `n`): dnešek jen když poslední den přehledu je dnešní místní datum. */
export function spendModel(costs: CostsIn, now: number, n = 14): SpendModel {
  const empty = (state: SpendModel["state"]): SpendModel => ({ state, priced: true, today: null, values: [], dates: [], avg: null, peak: 0 });
  if (!costs || costs.state === "off") return empty("off");
  const days = costs.summary?.days ?? [];
  if (!costs.summary) return empty(costs.state === "computing" ? "computing" : "none");
  if (!days.length) return empty("none");
  const win = days.slice(-n);
  const values = costSeries(days, n);
  const priced = win.some((d) => d.cost != null && d.cost > 0);
  const today = isToday(costs as { summary: { days: { date: string }[] } }, now) ? values[values.length - 1] : null;
  const sum = values.reduce((a, b) => a + b, 0);
  return { state: "ok", priced, today, values, dates: win.map((d) => d.date), avg: values.length ? sum / values.length : null, peak: Math.max(0, ...values) };
}

type SessionLike = { id: string; status: "waiting" | "working" | "idle" | "ended"; updatedAt: number };

/** Dlaždice sezení: pořadí jako v kartě (pracující první), strop `max`, zbytek jako počet. */
export function sessionTiles<T extends SessionLike>(list: readonly T[], max: number, asked: (id: string) => boolean = () => false): { shown: T[]; more: number; working: number } {
  const all = orderSessions(list, asked);
  return { shown: all.slice(0, Math.max(0, max)), more: Math.max(0, all.length - max), working: all.filter((x) => x.status === "working").length };
}
