// „Číslo jako hrdina“ (kolo 22, `look.hero`): které skutečné číslo karta ukáže velké, s popiskem, druhým řádkem
// a řadou pro sparkline. Čistá logika bez DOM a bez `t()` (test `hero.test.mjs`); texty jsou páry [klíč, čeština]
// s parametry, překlad až v `HeroCard.tsx`. Nic se nevymýšlí: bez čísla z reálného zdroje = `null` a karta se
// vykreslí jako dřív. Karta Claude s čekajícím schválením / otázkou = `null` (tlačítka schválení musí být vidět).
// Kolo 25: karta Claude ukáže VŽDY to, co si šéf zvolil (limit 5 h / týden / útrata); chybí-li zdroj, vrátí prázdný
// stav `empty` (nadpis + proč) — dřív tiše spadla na útratu a nešlo poznat, že limity chybí.
import { left, pctText, winState, type LimitWindow } from "../limits/model.ts";
import { usdDigits, type CostDay } from "../costs/model.ts";
import { isToday } from "../board/model.ts";
import { tempIn, tempUnit } from "../cards/temp.ts";

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
  /** prázdný stav místo čísla (zvolené číslo není z reálného zdroje): nadpis a jedna věta proč / co udělat */
  empty?: { title: Text; why: Text };
  min?: number;
  max?: number;
};

/** Výřez snapshotu, který hrdina čte (snadno se sestaví v testu). */
export type HeroInput = {
  pending?: readonly unknown[];
  questions?: readonly unknown[];
  limits?: { at: number | null; staleMs: number; fiveHour: LimitWindow | null; sevenDay?: LimitWindow | null; statusLine?: string; history5h?: [number, number][]; desktop?: string; fetch?: string } | null;
  costs?: { summary: { days: CostDay[] } | null } | null;
  personal?: { weather?: { status: string; label: string; fahrenheit?: boolean; data: { temp: number; min: number | null; max: number | null } | null } | null } | null;
};

/** Dny útraty pro sparkline: cena, když ji má aspoň jeden den, jinak tokeny (poslední `n` dní). */
export function costSeries(days: readonly CostDay[] | undefined, n = 14): number[] {
  const d = (days ?? []).slice(-n);
  const priced = d.some((x) => x.cost != null && x.cost > 0);
  return d.map((x) => (priced ? x.cost ?? 0 : x.tokens));
}

/** Co je hrdina karty Claude: limit 5 h (výchozí), týdenní limit, útrata dnes. */
export type ClaudePick = "5h" | "week" | "spend";

type LimW = LimitWindow | null | undefined;
const emptyFacts = (label: Text, title: Text, why: Text): HeroFacts => ({ value: 0, digits: 0, unit: "", label, empty: { title, why } });

/** Proč limity chybí (jedna věta; nic nehádá víc, než ví stav zapojení stavového řádku). Zapnuté další zdroje
 *  (`claude.limitsFetch` / `claude.limitsDesktop`, Rust `limitsfetch.rs` / `limitsrc.rs`) mají přednost — jejich
 *  stav je konkrétnější. */
export function limitsWhy(l: HeroInput["limits"]): Text {
  switch (l?.fetch) {
    case "expired":
      return ["cards.hero.why.fetchExpired", "Přihlášení Claude Code vypršelo — spusť v terminálu claude (případně /login); Ledge ho neobnovuje."];
    case "noLogin":
      return ["cards.hero.why.fetchNoLogin", "Ledge nenašel přihlášení Claude Code (~/.claude/.credentials.json) — přihlas se v terminálu příkazem claude."];
    case "error":
      return ["cards.hero.why.fetchError", "Anthropic teď na dotaz na limity neodpověděl; Ledge to zkusí znovu za pár minut."];
    case "pending":
      return ["cards.hero.why.fetchPending", "Ledge se ptá Anthropicu na limity…"];
  }
  if (l?.desktop === "none") return ["cards.hero.why.desktopNone", "Aplikace Claude limity na tomhle PC zatím neuložila (ukládá je, jen když běží)."];
  switch (l?.statusLine) {
    case "none":
    case "foreign":
      return ["cards.hero.why.connect", "Zapoj stavový řádek v Nastavení → Claude, odtud limity přicházejí."];
    case "error":
      return ["cards.hero.why.error", "Stavový řádek hlásí chybu — podrobnosti v Nastavení → Claude."];
    case "full":
      return ["cards.hero.why.waiting", "Stavový řádek je zapojený; limity přijdou po první odpovědi Clauda (jen předplatné Pro / Max)."];
    default:
      return ["cards.hero.why.none", "Zdroj limitů zatím nic nedodal; zapojení najdeš v Nastavení → Claude."];
  }
}

function limitFacts(s: HeroInput, now: number, w: LimW, label: Text, series: number[]): HeroFacts {
  const l = s.limits;
  const st = l ? winState(w, l.at, now, l.staleMs) : { kind: "none" as const };
  if (st.kind === "fresh" || st.kind === "stale") {
    const rest = left(w?.resetsAt, now);
    const sub: HeroFacts["sub"] =
      st.kind === "stale"
        ? { text: ["cards.hero.stale", "údaj z doby před {n} min"] as Text, vars: { n: st.ageMin } }
        : rest
          ? { text: ["cards.hero.reset", "reset za {h} h {m} min"] as Text, vars: { h: rest.d * 24 + rest.h, m: rest.m } }
          : undefined;
    return { value: Number(pctText(st.pct)), digits: 0, unit: "%", label, sub, series: series.length >= 2 ? series : undefined };
  }
  const title: Text = ["cards.hero.noLimits", "Limity zatím nejsou"];
  if (st.kind === "reset") return emptyFacts(label, title, ["cards.hero.why.reset", "Okno se obnovilo; nové číslo přijde s další odpovědí Clauda."]);
  return emptyFacts(label, title, limitsWhy(l));
}

function spendFacts(s: HeroInput, now: number, series: number[]): HeroFacts {
  const label: Text = ["cards.hero.costToday", "Útrata dnes (odhad v cenách API)"];
  // „dnes“ jen když poslední den přehledu je dnešní místní datum (jinak by starý přehled lhal o dnešku)
  const days = s.costs?.summary?.days;
  const today = isToday(s.costs, now) ? days?.[days.length - 1] : undefined;
  if (today?.cost != null) return { value: today.cost, digits: usdDigits(today.cost), unit: "$", label, series: series.length >= 2 ? series : undefined };
  return emptyFacts(label, ["cards.hero.noSpend", "Útrata dnes zatím není"], ["cards.hero.why.spend", "Přehled útraty ještě nemá dnešek s cenou — zapni a nech dopočítat v Nastavení → Claude → Útrata."]);
}

/** Hrdina karty Claude podle volby; čekající schválení / otázka = null (tlačítka musí být vidět). Bez dat NIKDY jiné
 *  číslo, než zvolené — ukáže prázdný stav s důvodem (kolo 25, oprava tichého přepnutí na útratu). */
function claude(s: HeroInput, now: number, pick: ClaudePick): HeroFacts | null {
  if ((s.pending?.length ?? 0) + (s.questions?.length ?? 0) > 0) return null;
  const series = costSeries(s.costs?.summary?.days);
  if (pick === "spend") return spendFacts(s, now, series);
  if (pick === "week") return limitFacts(s, now, s.limits?.sevenDay, ["cards.hero.limitWeek", "Limit Claude týden"], []);
  return limitFacts(s, now, s.limits?.fiveHour, ["cards.hero.limit5h", "Limit Claude 5 h"], (s.limits?.history5h ?? []).map((p) => p[1]));
}

function weather(s: HeroInput): HeroFacts | null {
  const w = s.personal?.weather;
  const d = w?.data;
  if (!w || !d || !Number.isFinite(d.temp) || (w.status !== "ok" && w.status !== "stale")) return null;
  const range = d.min != null && d.max != null ? { text: ["cards.hero.minmax", "dnes {min}° až {max}°"] as Text, vars: { min: Math.round(tempIn(d.min, w.fahrenheit)), max: Math.round(tempIn(d.max, w.fahrenheit)) } } : undefined;
  return { value: Math.round(tempIn(d.temp, w.fahrenheit)), digits: 0, unit: tempUnit(w.fahrenheit), label: ["cards.hero.weather", "Venku"], sub: range };
}

/** Hrdina karty `cardId` ze snapshotu; `null` = karta bez hrdiny / bez skutečného čísla (kreslí se jako dřív). */
export function heroFacts(cardId: string, s: HeroInput, now: number, pick: ClaudePick = "5h"): HeroFacts | null {
  switch (cardId) {
    case "claude":
      return claude(s, now, pick);
    case "weather":
      return weather(s);
    default:
      return null;
  }
}
