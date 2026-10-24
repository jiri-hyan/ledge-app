// „Číslo jako hrdina“ (kolo 22, `look.hero`): které skutečné číslo karta ukáže velké, s popiskem, druhým řádkem
// a řadou pro sparkline. Čistá logika bez DOM a bez `t()` (test `hero.test.mjs`); texty jsou páry [klíč, čeština]
// s parametry, překlad až v `HeroCard.tsx`. Nic se nevymýšlí: bez čísla z reálného zdroje = `null` a karta se
// vykreslí jako dřív. Karta Claude s čekajícím schválením / otázkou = `null` (tlačítka schválení musí být vidět).
import { left, pctText, winState, type LimitWindow } from "../limits/model.ts";
import { usdDigits, type CostDay } from "../costs/model.ts";
import { isToday } from "../board/model.ts";

export type Text = readonly [string, string];
export type HeroFacts = {
  /** hlavní číslo jako čísla (formát podle jazyka dělá UI) */
  value: number;
  /** počet desetin při formátování */
  digits: number;
  unit: string;
  label: Text;
  /** druhý řádek (klíč, čeština, parametry) */
  sub?: { text: Text; vars?: Record<string, string | number> };
  series?: number[];
  min?: number;
  max?: number;
};

/** Výřez snapshotu, který hrdina čte (snadno se sestaví v testu). */
export type HeroInput = {
  pending?: readonly unknown[];
  questions?: readonly unknown[];
  limits?: { at: number | null; staleMs: number; fiveHour: LimitWindow | null } | null;
  costs?: { summary: { days: CostDay[] } | null } | null;
  personal?: { weather?: { status: string; label: string; data: { temp: number; min: number | null; max: number | null } | null } | null } | null;
};

/** Dny útraty pro sparkline: cena, když ji má aspoň jeden den, jinak tokeny (poslední `n` dní). */
export function costSeries(days: readonly CostDay[] | undefined, n = 14): number[] {
  const d = (days ?? []).slice(-n);
  const priced = d.some((x) => x.cost != null && x.cost > 0);
  return d.map((x) => (priced ? x.cost ?? 0 : x.tokens));
}

function claude(s: HeroInput, now: number): HeroFacts | null {
  if ((s.pending?.length ?? 0) + (s.questions?.length ?? 0) > 0) return null;
  const series = costSeries(s.costs?.summary?.days);
  const l = s.limits;
  const st = l ? winState(l.fiveHour, l.at, now, l.staleMs) : { kind: "none" as const };
  if (st.kind === "fresh" || st.kind === "stale") {
    const rest = left(l?.fiveHour?.resetsAt, now);
    const sub: HeroFacts["sub"] =
      st.kind === "stale"
        ? { text: ["cards.hero.stale", "údaj z doby před {n} min"] as Text, vars: { n: st.ageMin } }
        : rest
          ? { text: ["cards.hero.reset", "reset za {h} h {m} min"] as Text, vars: { h: rest.d * 24 + rest.h, m: rest.m } }
          : undefined;
    return { value: Number(pctText(st.pct)), digits: 0, unit: "%", label: ["cards.hero.limit5h", "Limit Claude 5 h"], sub, series: series.length >= 2 ? series : undefined };
  }
  // „dnes“ jen když poslední den přehledu je dnešní místní datum (jinak by starý přehled lhal o dnešku)
  const days = s.costs?.summary?.days;
  const today = isToday(s.costs, now) ? days?.[days.length - 1] : undefined;
  if (today?.cost != null) {
    return { value: today.cost, digits: usdDigits(today.cost), unit: "$", label: ["cards.hero.costToday", "Útrata dnes (odhad v cenách API)"], series: series.length >= 2 ? series : undefined };
  }
  return null;
}

function weather(s: HeroInput): HeroFacts | null {
  const w = s.personal?.weather;
  const d = w?.data;
  if (!w || !d || !Number.isFinite(d.temp) || (w.status !== "ok" && w.status !== "stale")) return null;
  const range = d.min != null && d.max != null ? { text: ["cards.hero.minmax", "dnes {min}° až {max}°"] as Text, vars: { min: Math.round(d.min), max: Math.round(d.max) } } : undefined;
  return { value: Math.round(d.temp), digits: 0, unit: "°C", label: ["cards.hero.weather", "Venku"], sub: range };
}

/** Hrdina karty `cardId` ze snapshotu; `null` = karta bez hrdiny / bez skutečného čísla (kreslí se jako dřív). */
export function heroFacts(cardId: string, s: HeroInput, now: number): HeroFacts | null {
  switch (cardId) {
    case "claude":
      return claude(s, now);
    case "weather":
      return weather(s);
    default:
      return null;
  }
}
