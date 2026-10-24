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
  cost: number | null;
}
/** Řádek časové osy (Rust `usage::view::HourRow`): projekt a tokeny po místních hodinách dneška (24 čísel). */
export interface CostHourRow {
  name: string;
  tokens: number[];
  requests: number;
  other: boolean;
}
export interface CostsView {
  state: "off" | "idle" | "computing" | "ok" | "error";
  /** 0–1, jen při přepočtu */
  progress: number | null;
  computedAt: number | null;
  error: string | null;
  windowDays: number;
  /** `hours` = dnešek po hodinách a projektech (kolo 22, karta Claude dnes); starý snapshot bez pole = nic */
  summary: { ranges: CostRange[]; days: CostDay[]; hours?: CostHourRow[] } | null;
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

/** Výšky sloupců 0–1 podle ceny (bez ceny podle tokenů); prázdné okno = samé nuly. */
export function bars(days: CostDay[]): number[] {
  const priced = days.some((d) => d.cost != null && d.cost > 0);
  const val = (d: CostDay) => (priced ? d.cost ?? 0 : d.tokens);
  const max = Math.max(0, ...days.map(val));
  return days.map((d) => (max > 0 ? val(d) / max : 0));
}

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
