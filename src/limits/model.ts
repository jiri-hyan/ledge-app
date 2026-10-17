// Limity Claude 5h / týden — čistá logika pro kartu a peek (bez DOM a bez `t()`): stav okna podle
// stáří vzorku a času resetu, odpočet, šířka pruhu, úroveň barvy. Stejná pravidla jako Rust
// `limits.rs` (`state`); hranici stáří posílá jádro (`LimitsView.staleMs`), nic se nevymýšlí.

export interface LimitWindow {
  /** spotřeba 0–100 (může být desetinná) */
  pct: number;
  /** konec okna, ms epochy; null = nevíme */
  resetsAt: number | null;
}

export type WinState =
  | { kind: "none" }
  | { kind: "fresh"; pct: number }
  | { kind: "stale"; pct: number; ageMin: number }
  /** reset už proběhl → nové procento neznáme */
  | { kind: "reset" };

export function winState(w: LimitWindow | null | undefined, at: number | null | undefined, now: number, staleMs: number): WinState {
  if (!w || at == null) return { kind: "none" };
  if (w.resetsAt != null && w.resetsAt <= now) return { kind: "reset" };
  const age = now - at;
  if (age > staleMs) return { kind: "stale", pct: w.pct, ageMin: Math.floor(age / 60000) };
  return { kind: "fresh", pct: w.pct };
}

/** Zbývá do resetu (zaokrouhleno nahoru na minutu); v minulosti / neznámo = null. */
export function left(resetsAt: number | null | undefined, now: number): { d: number; h: number; m: number } | null {
  if (resetsAt == null) return null;
  const ms = resetsAt - now;
  if (ms <= 0) return null;
  const min = Math.ceil(ms / 60000);
  return { d: Math.floor(min / 1440), h: Math.floor((min % 1440) / 60), m: min % 60 };
}

/** Šířka pruhu v % (0–100). */
export const bar = (pct: number) => Math.max(0, Math.min(100, pct));

/** Barva pruhu: < 70 % ok, < 90 % pozor, jinak plno. */
export const level = (pct: number): "ok" | "warn" | "bad" => (pct >= 90 ? "bad" : pct >= 70 ? "warn" : "ok");

/** Celé procento k zobrazení („42 %“) — dolů, ať 99,6 % nevypadá jako plných 100. */
export const pctText = (pct: number) => String(Math.floor(pct));

/** Je čas resetu jiný den než teď? (pak se k času ukáže i datum) */
export function otherDay(ms: number, now: number): boolean {
  const a = new Date(ms);
  const b = new Date(now);
  return a.getFullYear() !== b.getFullYear() || a.getMonth() !== b.getMonth() || a.getDate() !== b.getDate();
}
