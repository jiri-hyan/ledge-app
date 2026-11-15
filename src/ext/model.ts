// Čistá logika bloku Rozšíření v Nastavení (bez DOM, test `model.test.mjs`): stav řádku, formát změřené
// RAM a CPU, zapnutí / vypnutí v seznamu `extensions`. Čísla jsou JEN z měření (`ext/measure.rs`):
// bez měření „—“, nikdy odhad. RAM = private bytes všech procesů rozšíření (i WebView2), CPU = % celého stroje
// za dobu od minulého měření (5 s).
import { fmtSize, type Fmt } from "../integrations/format.ts";

/** Rust `ext::measure::Cost` */
export interface ExtCost {
  ramBytes: number;
  cpuPct: number | null;
  procs: number;
  outsideJob: number;
}

/** Rust `ext::ExtView` (`ext_status`): `ExtStatus` + změřená cena běžícího rozšíření */
export interface ExtRow {
  id: string;
  enabled: boolean;
  phase: "idle" | "starting" | "running" | "paused" | "backoff" | "failed" | "incompatible";
  onDemand: boolean;
  pid: number | null;
  error: string | null;
  tail: string[];
  pauseWhy: string | null;
  cost: ExtCost | null;
}

/** Stav řádku pro popisek a barvu tečky. */
export type ExtState = "off" | "waiting" | "starting" | "running" | "paused" | "backoff" | "failed" | "incompatible";

export function extState(r: Pick<ExtRow, "enabled" | "phase">): ExtState {
  switch (r.phase) {
    case "idle":
      return r.enabled ? "waiting" : "off";
    case "starting":
    case "running":
    case "paused":
    case "backoff":
    case "failed":
    case "incompatible":
      return r.phase;
  }
}

/** Tečka: zelená běží, žlutá přechod / pauza, červená selhání, šedá vypnuto. */
export function stateTone(s: ExtState): "ok" | "warn" | "bad" | "" {
  if (s === "running") return "ok";
  if (s === "failed" || s === "incompatible") return "bad";
  if (s === "starting" || s === "paused" || s === "backoff" || s === "waiting") return "warn";
  return "";
}

/** RAM celého rozšíření: „62 MB“; bez měření „—“. */
export function fmtRam(c: ExtCost | null | undefined, f?: Fmt): string {
  return c && Number.isFinite(c.ramBytes) ? fmtSize(c.ramBytes, f) : "—";
}

/** CPU celého rozšíření v % celého stroje: „0,3 %“, pod 0,05 „< 0,1 %“ (změřeno, jen skoro nula); bez dvou
 *  vzorků „—“. */
export function fmtCpu(c: ExtCost | null | undefined, f: Fmt = (v, d) => v.toFixed(d)): string {
  const p = c?.cpuPct;
  if (p == null || !Number.isFinite(p)) return "—";
  return p < 0.05 ? `< ${f(0.1, 1)} %` : `${f(p, p < 10 ? 1 : 0)} %`;
}

/** Seznam `extensions` po zapnutí / vypnutí jednoho id (bez duplicit, cizí id zůstanou — z novější verze). */
export function withExt(list: readonly string[], id: string, on: boolean): string[] {
  const rest = list.filter((x) => x !== id);
  return on ? [...rest, id] : rest;
}
