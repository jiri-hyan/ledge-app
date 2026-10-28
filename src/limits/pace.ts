// Tempo čerpání limitů (volba `claude.limitsPace`): ryska plánu v pruhu 5h / týden, segment přes plán,
// odhad 5h okna z tempa. Čisté funkce bez DOM a bez `t()` (překlad se vstřikuje do `paceTip`).
// Plán = uplynulý podíl okna × 100 (týden 7 d, 5h okno 5 h). Nic se nevymýšlí: bez `resetsAt` nebo po
// resetu vrací `null` a volající nekreslí nic (pruh jako dnes).
const bar = (pct: number) => Math.max(0, Math.min(100, pct));

export const WEEK_MS = 7 * 86_400_000;
export const WIN5_MS = 5 * 3_600_000;
export const DAY_MS = 86_400_000;
/** Okno historie pro tempo 5h a minimum rozpětí, od kterého je historie dost dlouhá (jinak průměr okna). */
export const RATE_WINDOW_MS = 30 * 60_000;
export const RATE_MIN_MS = 10 * 60_000;

export type Lv = "ok" | "warn" | "bad";

/** Plán v % (0–100): uplynulý podíl okna délky `lenMs` končícího v `resetsAt`; null = nelze. */
export function planPct(resetsAt: number | null | undefined, lenMs: number, now: number): number | null {
  if (resetsAt == null || resetsAt <= now) return null;
  return Math.max(0, Math.min(100, ((now - (resetsAt - lenMs)) / lenMs) * 100));
}

/** Úroveň podle odchylky (b.): do +0,5 ok, do +5 pozor, nad +5 přes. */
export const paceLevel = (dev: number): Lv => (dev <= 0.5 ? "ok" : dev <= 5 ? "warn" : "bad");

/** Segment výplně přes plán (od rysky po konec výplně), jen když skutečné % > plán. */
export function overSeg(actual: number, plan: number): { from: number; to: number; lv: Lv } | null {
  return actual > plan ? { from: bar(plan), to: bar(actual), lv: paceLevel(actual - plan) } : null;
}

/** Týden: kolik % zbývá do plánu na konci aktuálního 24h bloku (záporné = dnes přečerpáno). */
export function weekToday(actual: number, resetsAt: number, now: number): number {
  const elapsed = now - (resetsAt - WEEK_MS);
  const block = Math.max(0, Math.min(6, Math.floor(elapsed / DAY_MS)));
  return ((block + 1) / 7) * 100 - actual;
}

export interface Rate {
  /** % za hodinu */
  perH: number;
  /** recent = posledních 30 min historie, avg = průměr od začátku okna */
  src: "recent" | "avg";
}

/** Tempo 5h okna: z historie `[at, pct]` za 30 min (aspoň 10 min rozpětí), jinak průměr okna. */
export function rate5h(hist: [number, number][] | undefined, used: number, resetsAt: number, now: number): Rate | null {
  const pts = (hist ?? []).filter(([at]) => at >= now - RATE_WINDOW_MS && at <= now);
  if (pts.length >= 2) {
    const [a, p0] = pts[0];
    const [b, p1] = pts[pts.length - 1];
    if (b - a >= RATE_MIN_MS) return { perH: ((p1 - p0) / (b - a)) * 3_600_000, src: "recent" };
  }
  const el = now - (resetsAt - WIN5_MS);
  return el > 0 ? { perH: (used / el) * 3_600_000, src: "avg" } : null;
}

export type Eta =
  | { kind: "idle" }
  | { kind: "lasts"; until: number }
  | { kind: "runs-out"; at: number; until: number; slowTo: number };

/** Odhad 5h okna: tempo ≤ 0 = idle; 100 % až po obnově = lasts; jinak čas dosažení 100 % a tempo, které stačí. */
export function eta5h(rate: Rate, used: number, resetsAt: number, now: number): Eta {
  if (!(rate.perH > 0)) return { kind: "idle" };
  const at = now + ((100 - used) / rate.perH) * 3_600_000;
  if (at >= resetsAt) return { kind: "lasts", until: resetsAt };
  return { kind: "runs-out", at, until: resetsAt, slowTo: (100 - used) / ((resetsAt - now) / 3_600_000) };
}

export interface PaceInfo {
  plan: number;
  dev: number;
  /** segment přes plán (null = skutečné ≤ plán) */
  over: { from: number; to: number; lv: Lv } | null;
  /** barva celé výplně navíc (5h: odhad říká „dojde před obnovou“) */
  fillLv: Lv | null;
  /** jen týden */
  today?: number;
  /** jen 5h */
  rate?: Rate | null;
  eta?: Eta | null;
}

/** Tempo pruhu; null = nic nekreslit (bez resetsAt / po resetu). */
export function paceOf(kind: "week" | "5h", used: number, resetsAt: number | null | undefined, hist: [number, number][] | undefined, now: number): PaceInfo | null {
  const plan = planPct(resetsAt, kind === "week" ? WEEK_MS : WIN5_MS, now);
  if (plan == null || resetsAt == null) return null;
  const base = { plan, dev: used - plan, over: overSeg(used, plan), fillLv: null as Lv | null };
  if (kind === "week") return { ...base, today: weekToday(used, resetsAt, now) };
  const rate = rate5h(hist, used, resetsAt, now);
  const eta = rate ? eta5h(rate, used, resetsAt, now) : null;
  return { ...base, rate, eta, fillLv: eta?.kind === "runs-out" ? "warn" : null };
}

export type Tr = (key: string, fallback: string, vars?: Record<string, string | number>) => string;
const n1 = (x: number) => String(Math.round(x * 10) / 10);

/** Text tooltipu (čísla); `clock` = čas „HH:MM“, `tr` = překlad. */
export function paceTip(kind: "week" | "5h", p: PaceInfo, clock: (ms: number) => string, t: Tr): string {
  if (kind === "week") {
    const dev = Math.round(p.dev * 10) / 10;
    const devTxt = dev > 0 ? t("sessions.limits.pace.over", "+{n} b. nad tempem", { n: n1(dev) }) : t("sessions.limits.pace.under", "{n} b. pod tempem", { n: n1(-dev) });
    const td = p.today ?? 0;
    const today = td >= 0 ? t("sessions.limits.pace.todayLeft", "dnes zbývá {n} %", { n: n1(td) }) : t("sessions.limits.pace.todayOver", "dnes přečerpáno o {n} %", { n: n1(-td) });
    return `${t("sessions.limits.pace.plan", "plán {n} %", { n: n1(p.plan) })} · ${devTxt} · ${today}`;
  }
  const r = p.rate;
  if (!r || !p.eta) return "";
  if (p.eta.kind === "idle") return t("sessions.limits.pace.idle", "bez čerpání");
  const rate = r.src === "recent" ? t("sessions.limits.pace.rateRecent", "tempo {n} %/h (posl. 30 min)", { n: n1(r.perH) }) : t("sessions.limits.pace.rateAvg", "tempo {n} %/h (průměr okna)", { n: n1(r.perH) });
  if (p.eta.kind === "lasts") return `${rate} · ${t("sessions.limits.pace.lasts", "vydrží do obnovy {t}", { t: clock(p.eta.until) })}`;
  return `${rate} · ${t("sessions.limits.pace.runsOut", "dojde {a}, obnova {b}", { a: clock(p.eta.at), b: clock(p.eta.until) })} · ${t("sessions.limits.pace.slow", "zpomal na ≤ {n} %/h", { n: n1(p.eta.slowTo) })}`;
}
