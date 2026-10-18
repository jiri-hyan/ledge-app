// Čistá logika karet Systém a GitHub (bez importů za běhu — test `format.test.mjs`): velikosti a rychlosti
// sítě v čitelných jednotkách, body sparklinu, úroveň vytížení, výběr disků a stav CI/revize u PR.
// Čísla se formátují předanou funkcí `f(value, digits)` (UI dává `num` podle jazyka, test výchozí tečku).

export type NetUnit = "bytes" | "bits";
export type Fmt = (v: number, digits: number) => string;
const plain: Fmt = (v, d) => v.toFixed(d);

/** Číslo se zlomkem jen u malých hodnot: 8,2 · 12 · 476. */
const digitsFor = (v: number): number => (v < 10 ? 1 : 0);

const SIZE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/** Velikost na disku / v paměti (po 1024): „476 GB“, „11,2 GB“, „0 B“. Záporné a nečíselné = 0. */
export function fmtSize(bytes: number, f: Fmt = plain): string {
  let v = Number.isFinite(bytes) && bytes > 0 ? bytes : 0;
  let i = 0;
  while (v >= 1024 && i < SIZE_UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  // 1023,96 KB se po zaokrouhlení nesmí ukázat jako „1024 KB“
  if (i < SIZE_UNITS.length - 1 && Number(v.toFixed(digitsFor(v))) >= 1024) {
    v /= 1024;
    i++;
  }
  return `${f(v, i === 0 ? 0 : digitsFor(v))} ${SIZE_UNITS[i]}`;
}

const BYTE_RATES = ["B/s", "KB/s", "MB/s", "GB/s"] as const;
const BIT_RATES = ["bit/s", "kbit/s", "Mbit/s", "Gbit/s"] as const;

/** Rychlost sítě z bajtů za sekundu (po 1000 jako u přípojek): „1,2 MB/s“ nebo „9,6 Mbit/s“.
 *  `null` = zatím nenaměřeno → „–“. */
export function fmtRate(bytesPerSec: number | null | undefined, unit: NetUnit, f: Fmt = plain): string {
  if (bytesPerSec == null || !Number.isFinite(bytesPerSec)) return "–";
  const units = unit === "bits" ? BIT_RATES : BYTE_RATES;
  let v = Math.max(0, bytesPerSec) * (unit === "bits" ? 8 : 1);
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  if (i < units.length - 1 && Number(v.toFixed(digitsFor(v))) >= 1000) {
    v /= 1000;
    i++;
  }
  return `${f(v, i === 0 ? 0 : digitsFor(v))} ${units[i]}`;
}

/** Procenta na celé číslo: „23 %“; `null` = čeká se na druhý vzorek → „–“. */
export const fmtPct = (v: number | null | undefined, f: Fmt = plain): string => (v == null || !Number.isFinite(v) ? "–" : `${f(Math.round(v), 0)} %`);

/** Počet vzorků sparklinu (Rust `sysmon::calc::HISTORY`). */
export const HISTORY = 60;

/** Body `polyline` pro sparkline v poli `w × h`: nejnovější vzorek vpravo, stará data se zleva nedoplňují
 *  (graf roste zprava). `max` = strop osy (u procent 100); bez něj největší hodnota, nejméně `floor`.
 *  Méně než 2 vzorky = prázdné (nic se nekreslí). Svislý okraj 1 px, ať čára nezmizí na hraně. */
export function sparkPoints(values: readonly number[], w: number, h: number, max?: number, floor = 1): string {
  const v = values.slice(-HISTORY).filter((x) => Number.isFinite(x));
  if (v.length < 2) return "";
  const top = max ?? Math.max(floor, ...v);
  const step = w / (HISTORY - 1);
  const x0 = w - (v.length - 1) * step;
  return v
    .map((y, i) => {
      const frac = Math.min(1, Math.max(0, y / top));
      return `${round2(x0 + i * step)},${round2(1 + (h - 2) * (1 - frac))}`;
    })
    .join(" ");
}
const round2 = (n: number) => Math.round(n * 100) / 100;

export type Load = "ok" | "warm" | "hot";
/** Úroveň vytížení (CPU, paměť) podle procent: od 75 % žlutá, od 90 % červená (shodně s proužkem). */
export const loadOf = (pct: number | null | undefined): Load => (pct == null ? "ok" : pct >= 90 ? "hot" : pct >= 75 ? "warm" : "ok");

/** Úroveň zaplnění disku: červená, když zbývá míň než 10 % (nebo 10 GB), žlutá pod 20 %. */
export function diskLoad(free: number, total: number): Load {
  if (total <= 0) return "ok";
  const f = free / total;
  return f < 0.1 || free < 10 * 1024 ** 3 ? "hot" : f < 0.2 ? "warm" : "ok";
}

/** Zaplněná část disku 0–1. */
export const diskUsed = (free: number, total: number): number => (total > 0 ? Math.min(1, Math.max(0, 1 - free / total)) : 0);

/** Přepne disk ve výběru karty. `selected` prázdné = „všechny“ (config `system.disks`); `all` = písmena, která
 *  ve stroji jsou. Výsledek „všechny vybrané“ i „žádný“ se zapíše jako prázdný seznam. */
export function toggleDisk(selected: readonly string[], letter: string, all: readonly string[]): string[] {
  const L = letter.toUpperCase();
  const cur = selected.length ? selected.map((s) => s.toUpperCase()) : [...all];
  const next = cur.includes(L) ? cur.filter((x) => x !== L) : [...cur, L];
  const sorted = [...new Set(next)].sort();
  return sorted.length === 0 || all.every((a) => sorted.includes(a)) ? [] : sorted;
}

/** Je disk `letter` ve výběru? (prázdný výběr = ano) */
export const diskOn = (selected: readonly string[], letter: string): boolean => selected.length === 0 || selected.some((s) => s.toUpperCase() === letter.toUpperCase());

export type Ci = "none" | "pending" | "success" | "failure";
/** Barva a klíč popisku CI u PR. */
export const CI_TONE: Record<Ci, "none" | "amber" | "green" | "red"> = { none: "none", pending: "amber", success: "green", failure: "red" };

/** Kdy minulo `iso` (ISO čas z GitHubu) vzhledem k `now`; neplatný čas = `null`. */
export function isoMs(iso: string | undefined): number | null {
  const ms = iso ? Date.parse(iso) : NaN;
  return Number.isFinite(ms) ? ms : null;
}

/** Počet s „+“, když je to jen spodní odhad: „20+“. */
export const countText = (n: number, more: boolean): string => `${n}${more ? "+" : ""}`;
