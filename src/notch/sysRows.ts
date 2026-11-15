// Řádky sekce Systém v liště notche (čistá logika, test `sysRows.test.mjs`): hodnota, historie a úroveň zátěže
// jednotně s kartou Systém — CPU / RAM / GPU přes `loadOf` (75 / 90, jedna tabulka `charts/thresholds.ts`), disk přes
// `diskLoad` (zbývá < 10 % nebo < 10 GB = červená, < 20 % = jantar), síť bez úrovně. Bez DOM a bez `t()`.
import type { SystemView } from "../integrations/api";
import { diskLoad, fmtPct, fmtRate, loadOf, type Load } from "../integrations/format.ts";
import { paintOf, type Paint } from "./look.ts";
import type { Metric } from "./bar.ts";

export type Row = { key: Metric; name: string; pct: number | null; hist?: number[]; max?: number; text: string; load: Load };

/** Barva „podle zátěže“ pro dnešní vzhled bez volby barvy: ok = akcent, warm = jantar, hot = červená. */
export const LOAD_CSS: Readonly<Record<Load, string | null>> = { ok: null, warm: "var(--amber)", hot: "var(--red)" };

/** Řádky sekce podle zvolených metrik; co nemá data, se vynechá (žádné vymyšlené hodnoty). `names` = přeložené popisky disku a sítě. */
export function rowsOf(v: SystemView, want: readonly Metric[], names: { disk: string; net: string }): Row[] {
  const disk = v.disks.find((x) => x.system) ?? v.disks[0];
  const diskPct = disk && disk.total > 0 ? ((disk.total - disk.free) / disk.total) * 100 : null;
  const all: Record<Metric, Row | null> = {
    cpu: v.cpu != null ? { key: "cpu", name: "CPU", pct: v.cpu, hist: v.cpuHist, max: 100, text: fmtPct(v.cpu), load: loadOf(v.cpu) } : null,
    ram: v.memTotal > 0 ? { key: "ram", name: "RAM", pct: v.memPct, hist: v.memHist, max: 100, text: fmtPct(v.memPct), load: loadOf(v.memPct) } : null,
    gpu: v.gpu != null ? { key: "gpu", name: "GPU", pct: v.gpu, hist: v.gpuHist, max: 100, text: fmtPct(v.gpu), load: loadOf(v.gpu) } : null,
    disk: diskPct != null && disk ? { key: "disk", name: names.disk, pct: diskPct, text: fmtPct(diskPct), load: diskLoad(disk.free, disk.total) } : null,
    net:
      v.rx != null || v.tx != null
        ? { key: "net", name: names.net, pct: null, hist: v.rxHist, text: `↓ ${fmtRate(v.rx, v.netUnit)}  ↑ ${fmtRate(v.tx, v.netUnit)}`, load: "ok" }
        : null,
  };
  return want.map((m) => all[m]).filter((r): r is Row => !!r);
}

/** Výplň řádku: zvolená barva sekce vyhrává; bez volby akcent a při zátěži jantar / červená podle `r.load`. */
export function paintRow(r: Row, color: string | undefined): Paint {
  if (color === "load") return { kind: "solid", css: LOAD_CSS[r.load] ?? "var(--green)" };
  const css = LOAD_CSS[r.load];
  return !color && css ? { kind: "solid", css } : paintOf(color, r.pct);
}

/** Barva čísla / textu: jen se zvolenou barvou a řádkem s procentem (výchozí = barva textu); přechod = akcent. */
export function toneRow(r: Row, color: string | undefined): string | undefined {
  if (!color || r.pct == null) return undefined;
  const p = paintRow(r, color);
  return p.kind === "solid" ? p.css : "var(--accent)";
}
