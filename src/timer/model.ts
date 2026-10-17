// Časovač v UI: zrcadlo stavu `timer.rs` (časy v ms epochy) a čistá logika karty — zbývá / naměřeno,
// průběh, formát se sekundami, vlastní minuty. Sekundy počítá jen otevřený šuplík z `now` (tik skořápky);
// proužek se překresluje po minutách v Rustu. Bez importů za běhu (test `model.test.mjs`).

export type Countdown =
  | { state: "running"; end: number; total: number }
  | { state: "paused"; left: number; total: number }
  | { state: "done"; at: number; total: number };
export type Stopwatch = { state: "running"; start: number } | { state: "paused"; elapsed: number };
export interface TimerState {
  countdown: Countdown | null;
  stopwatch: Stopwatch | null;
}
/** `Snapshot.timer` (Rust `timer::TimerView`) */
export interface TimerView extends TimerState {
  presets: number[];
}
/** akce příkazu `timer` (Rust `timer::Op`) */
export type TimerOp =
  | { op: "start"; minutes: number }
  | { op: "add"; minutes: number }
  | { op: "pause" | "resume" | "cancel" | "swStart" | "swPause" | "swReset" };

/** Strop odpočtu v minutách (Rust `timer::MAX_MIN`). */
export const MAX_MIN = 600;
export const DEFAULT_PRESETS = [5, 15, 25, 50];

/** Zbývá z odpočtu (ms, nikdy záporné); doběhlý = 0. */
export function leftMs(c: Countdown, now: number): number {
  if (c.state === "running") return Math.max(0, c.end - now);
  if (c.state === "paused") return c.left;
  return 0;
}

/** Naměřeno stopkami (ms). */
export function elapsedMs(w: Stopwatch, now: number): number {
  return w.state === "running" ? Math.max(0, now - w.start) : w.elapsed;
}

/** Průběh odpočtu 0–1 (kolik už uběhlo). */
export function progress(c: Countdown, now: number): number {
  if (c.state === "done" || c.total <= 0) return 1;
  return Math.min(1, Math.max(0, 1 - leftMs(c, now) / c.total));
}

const p2 = (n: number) => String(n).padStart(2, "0");

/** „m:ss“ / „h:mm:ss“. `up` = zaokrouhlit sekundy nahoru (odpočet: 0,4 s zbývá = 0:01, ne 0:00). */
export function fmtDuration(ms: number, up = false): string {
  const total = Math.max(0, up ? Math.ceil(ms / 1000) : Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${h}:${p2(m)}:${p2(s)}` : `${m}:${p2(s)}`;
}

/** Vlastní minuty z pole (čárka i tečka, desetiny se zaokrouhlí); mimo 1–600 nebo nesmysl = null. */
export function parseMinutes(raw: string): number | null {
  const s = raw.trim().replace(",", ".");
  if (!/^\d+(\.\d+)?$/.test(s)) return null;
  const n = Math.round(Number(s));
  return n >= 1 && n <= MAX_MIN ? n : null;
}

/** Předvolby ze snapshotu (starý snapshot bez `timer` = výchozí). */
export const presetsOf = (v: TimerView | null | undefined): number[] => (v?.presets?.length ? v.presets : DEFAULT_PRESETS);

/** Předvolby z pole Nastavení („5, 15, 25“): čísla 1–600 bez duplicit, nejvýš 6, v pořadí zápisu. */
export function parsePresets(raw: string): number[] {
  const out: number[] = [];
  for (const part of raw.split(/[\s,;]+/)) {
    const n = parseMinutes(part);
    if (n != null && !out.includes(n) && out.length < 6) out.push(n);
  }
  return out;
}
