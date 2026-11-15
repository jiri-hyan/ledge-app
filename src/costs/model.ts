// Útrata Claude (odhad v cenách API, ne účet) — čistá logika sekce v kartě Claude (bez DOM a bez `t()`):
// zkrácení počtu tokenů, počet desetin u USD, výšky sloupců po dnech, podíl řádku, průběh přepočtu.
// Data posílá jádro (`costslive::view` → `Snapshot.costs`); cena `null` = model bez ceny v ceníku → „—“.

export interface CostLine {
  name: string;
  tokens: number;
  /** null = žádný token řádku nemá cenu v ceníku */
  cost: number | null;
  /** tokeny bez ceny (neznámý model) */
  unpriced: number;
  /** součet zbytku za top N */
  other: boolean;
}
export interface CostRange {
  days: number;
  split: { input: number; output: number; cacheWrite: number; cacheRead: number };
  total: CostLine;
  requests: number;
  projects: CostLine[];
  models: CostLine[];
}
export interface CostDay {
  /** YYYY-MM-DD, místní den */
  date: string;
  tokens: number;
  /** null = žádný token dne nemá cenu; jinak součet jen NACENĚNÝCH tokenů (při `unpriced` > 0 dolní odhad) */
  cost: number | null;
  /** tokeny dne bez ceny v ceníku (starý snapshot bez pole = 0) */
  unpriced?: number;
}
/** Řádek časové osy (Rust `usage::view::HourRow`): projekt a tokeny po místních hodinách dneška (24 čísel). */
export interface CostHourRow {
  name: string;
  tokens: number[];
  requests: number;
  other: boolean;
}
/** Řádek podlahy nové relace / agenta (Rust `usage::floor::FloorRow`): medián kontextu 1. volání za 7 dní. */
export interface FloorRow {
  /** projekt nebo typ agenta */
  name: string;
  median: number;
  /** souborů (session / agentů) za 7 dní */
  n: number;
  /** předchozích 7 dní; null = žádný soubor */
  prevMedian: number | null;
  prevN: number;
  /** median − prevMedian; jen když je v obou obdobích aspoň 3 soubory, jinak null */
  delta: number | null;
}
export interface CostFloor {
  projects: FloorRow[];
  agents: FloorRow[];
}
export interface CostsView {
  state: "off" | "idle" | "computing" | "ok" | "error";
  /** 0–1, jen při přepočtu */
  progress: number | null;
  computedAt: number | null;
  error: string | null;
  windowDays: number;
  /** `hours` = dnešek po hodinách a projektech (kolo 22, karta Claude dnes); starý snapshot bez pole = nic */
  summary: { ranges: CostRange[]; days: CostDay[]; hours?: CostHourRow[]; floor?: CostFloor; sessions?: import("../sessions/cost").SessCost[] } | null;
  pricesVerified: string;
  pricesSource: string;
}

/** Zkrácené číslo tokenů: hodnota + jednotka (k / M / G) + počet desetin pro `num`. */
export function compact(n: number): { v: number; unit: "" | "k" | "M" | "G"; digits: number } {
  const a = Math.abs(n);
  if (a >= 1e9) return { v: n / 1e9, unit: "G", digits: a >= 1e11 ? 0 : 1 };
  if (a >= 1e6) return { v: n / 1e6, unit: "M", digits: a >= 1e8 ? 0 : 1 };
  if (a >= 1e3) return { v: n / 1e3, unit: "k", digits: a >= 1e5 ? 0 : 1 };
  return { v: n, unit: "", digits: 0 };
}

/** Desetiny u USD: pod 100 $ dvě (0,37 $), jinak celé. */
export const usdDigits = (v: number) => (Math.abs(v) >= 100 ? 0 : 2);

/** Zkrácený počet tokenů („3,4 M“): JEDINÝ formátovač (karta Claude, hrdina, kontext session). `f` = formát čísla
 *  (UI dává `num` podle jazyka, test výchozí tečku); bez `f` `toFixed`. */
export function tokText(n: number, f: (v: number, digits: number) => string = (v, d) => v.toFixed(d)): string {
  const c = compact(n);
  return `${f(c.v, c.digits)}${c.unit ? ` ${c.unit}` : ""}`;
}

/** Série útraty po dnech pro sloupce i sparkline — JEDINÁ definice (N5 / N7, kolo 34). Režim `cost`, když má cenu
 *  aspoň jeden den okna (jinak `tokens`). `unknown[i]` = den má tokeny, ale žádná jejich cena není známá (jen neznámé
 *  modely): v grafu je výška 0, ale není to „0 $“ — kdo hodnotu ukazuje, musí napsat „—“. Poslední `n` dní (bez `n` všechny). */
export interface PriceSeries {
  mode: "cost" | "tokens";
  values: number[];
  unknown: boolean[];
}
export function priceSeries(days: readonly CostDay[] | undefined, n?: number): PriceSeries {
  const d = n == null ? [...(days ?? [])] : (days ?? []).slice(-n);
  const mode = d.some((x) => x.cost != null && x.cost > 0) ? "cost" : "tokens";
  const unknown = d.map((x) => mode === "cost" && x.cost == null && x.tokens > 0);
  return { mode, values: d.map((x) => (mode === "cost" ? x.cost ?? 0 : x.tokens)), unknown };
}

/** Hodnoty série → výšky sloupců 0–1 (podle maxima; prázdná / nulová série = samé nuly, ne NaN). */
export function normalize(values: readonly number[]): number[] {
  const max = Math.max(0, ...values);
  return values.map((v) => (max > 0 ? v / max : 0));
}

/** Výšky sloupců 0–1 podle ceny (bez ceny podle tokenů); prázdné okno = samé nuly. */
export const bars = (days: CostDay[]): number[] => normalize(priceSeries(days).values);

/** Text částky: bez ceny „—“; při tokenech bez ceny „≥ X“ (dolní odhad, nikdy vymyšlená cena). `fmt` formátuje číslo. */
export function costText(cost: number | null | undefined, unpriced: number | undefined, fmt: (v: number) => string): string {
  if (cost == null) return "—";
  return `${Math.max(0, unpriced ?? 0) > 0 ? "≥ " : ""}${fmt(cost)}`;
}

/** Tokeny dne bez ceny (chybějící pole = 0). */
export const dayUnpriced = (d: { unpriced?: number }) => Math.max(0, d.unpriced ?? 0);

/** Součet nenaceněných tokenů přes dny. */
export const sumUnpriced = (days: readonly { unpriced?: number }[]) => days.reduce((a, d) => a + dayUnpriced(d), 0);

/** Je cena dne jen dolní odhad? (aspoň jeden token bez ceny → částka „≥“, nikdy ne vymyšlená cena). */
export const isLowerBound = (d: { unpriced?: number }) => dayUnpriced(d) > 0;

/** Podíl řádku na celku 0–1 (podle ceny, když ji mají oba; jinak podle tokenů). */
export function share(line: CostLine, total: CostLine): number {
  if (line.cost != null && total.cost != null && total.cost > 0) return Math.min(1, line.cost / total.cost);
  return total.tokens > 0 ? Math.min(1, line.tokens / total.tokens) : 0;
}

/** Průběh přepočtu v celých procentech (dolů, 100 jen hotovo); bez údaje null. */
export const progressPct = (p: number | null | undefined) => (p == null ? null : Math.max(0, Math.min(100, Math.floor(p * 100))));

/** Rozsah podle počtu dní (1 / 7 / okno); chybí-li, první dostupný. */
export function pickRange(ranges: CostRange[] | undefined, days: number): CostRange | null {
  if (!ranges?.length) return null;
  return ranges.find((r) => r.days === days) ?? ranges[0];
}

/** Trend podlahy: bez `delta` (málo souborů) nic; změna pod 5 % předchozího mediánu (nejméně 1 000 tokenů) = beze změny. */
export function floorTrend(r: FloorRow): { dir: "up" | "down" | "flat"; abs: number } | null {
  if (r.delta == null) return null;
  const abs = Math.abs(r.delta);
  if (abs < Math.max(1000, (r.prevMedian ?? 0) * 0.05)) return { dir: "flat", abs };
  return { dir: r.delta > 0 ? "up" : "down", abs };
}

/** Po kolika dnech od `VERIFIED` ceníku (Rust `usage/price.rs`) se ukáže varování, že odhad USD může být zastaralý (L235). */
export const PRICE_STALE_DAYS = 60;

/** Stáří ceníku ve dnech (`verified` = `YYYY-MM-DD`); neplatné datum = null (nic se nehlásí, nevymýšlí). */
export function priceAgeDays(verified: string | null | undefined, nowMs: number): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(verified ?? "");
  if (!m) return null;
  const t0 = Date.UTC(+m[1], +m[2] - 1, +m[3]);
  return Number.isFinite(t0) ? Math.max(0, Math.floor((nowMs - t0) / 86_400_000)) : null;
}

/** Stáří ceníku, když přesáhlo `PRICE_STALE_DAYS`, jinak null (varování se neukáže). */
export const priceStaleDays = (verified: string | null | undefined, nowMs: number): number | null => {
  const d = priceAgeDays(verified, nowMs);
  return d != null && d > PRICE_STALE_DAYS ? d : null;
};
