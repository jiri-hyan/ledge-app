// Vzhledy karty Systém (kolo 25, `look.design`, registr `layout/designs.ts`): prstence, budíky, histogram, terminál,
// čísla a síť. Řádkový vzhled `lines` zůstává v `SystemCard.tsx`. Čte stejný `SystemView` (historie vzorků po 2 s, disky,
// síť) — nic neměří navíc, vzorkování řídí `SystemCard` (jen když je karta vidět a šuplík otevřený). Logika `systemLooks.ts`.
// Šířka: `container-type` na `.sd` (`systemLooks.css`) — úzká = podstatné, široká = GB, průměr / špička, delší historie.
// Každý graf má tooltip s hodnotou a časem; animace jen transform (budík), v klidu nic neběží.
import { For, Match, Show, Switch, type JSX } from "solid-js";
import { num } from "../format";
import { t } from "../i18n";
import Gauge from "../charts/Gauge";
import Rings from "../charts/Rings";
import type { SystemView } from "./api";
import { diskLoad, fmtPct, fmtRate, fmtSize, loadOf, sparkPoints, type Load } from "./format";
import { fullestDisk, histStats, loadBars, termCells, windowSec } from "./systemLooks";
import "./systemLooks.css";

const W = 120;
const H = 32;
const NET_FLOOR = 50_000;
const tone = (l: Load): "ok" | "warn" | "bad" => (l === "hot" ? "bad" : l === "warm" ? "warn" : "ok");

type P = { v: SystemView; design: string; onOpen: () => void };

function Spark(p: { values: number[]; max?: number; floor?: number; load?: Load; title: string; second?: { values: number[] } }) {
  return (
    <svg class="sd-spark" data-load={p.load} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={p.title}>
      <title>{p.title}</title>
      <polyline points={sparkPoints(p.values, W, H, p.max, p.floor)} />
      <Show when={p.second}>{(s) => <polyline class="sd-spark-2" points={sparkPoints(s().values, W, H, p.max, p.floor)} />}</Show>
    </svg>
  );
}

export default function SystemDesign(p: P) {
  const v = () => p.v;
  const size = (n: number) => fmtSize(n, num);
  const rate = (r: number | null) => fmtRate(r, v().netUnit, num);
  const cpuLoad = () => loadOf(v().cpu);
  const memLoad = () => loadOf(v().memPct);
  const disk = () => fullestDisk(v().disks);
  const diskPct = () => (disk() ? disk()!.used * 100 : null);
  const diskTone = () => (disk() ? tone(diskLoad(disk()!.disk.free, disk()!.disk.total)) : "ok");
  const netMax = () => Math.max(NET_FLOOR, ...v().rxHist, ...v().txHist);
  /** tooltip historie: aktuální hodnota, průměr a špička za dobu vzorků */
  const histTip = (label: string, hist: number[], now: string, fmt: (x: number) => string) => {
    const s = histStats(hist);
    return s
      ? t("cards.sd.tip", "{label}: {now} · průměr {avg}, špička {peak} za posledních {sec} s", { label, now, avg: fmt(s.avg), peak: fmt(s.peak), sec: windowSec(s.n) })
      : t("cards.sd.tip.none", "{label}: {now} · historie se teprve plní", { label, now });
  };
  const pct = (x: number) => fmtPct(x, num);
  const cpuTip = () => histTip(t("cards.system.cpu", "CPU"), v().cpuHist, pct(v().cpu ?? NaN), pct);
  const memTip = () => histTip(t("cards.system.mem", "Paměť"), v().memHist, `${pct(v().memPct)} (${size(v().memUsed)} / ${size(v().memTotal)})`, pct);
  const diskTip = () => (disk() ? t("cards.sd.tip.disk", "Disk {letter}: zaplněno {pct}, volné {free} z {total}", { letter: disk()!.disk.letter, pct: pct(diskPct()!), free: size(disk()!.disk.free), total: size(disk()!.disk.total) }) : "");
  const netTip = () => `${t("cards.system.down", "Příjem")} ${rate(v().rx)} · ${t("cards.system.up", "Odesílání")} ${rate(v().tx)} · ${t("cards.sd.tip.peak", "špička {rx} / {tx} za posledních {sec} s", { rx: rate(Math.max(0, ...v().rxHist)), tx: rate(Math.max(0, ...v().txHist)), sec: windowSec(v().rxHist.length) })}`;
  const hist = () => loadBars(v().cpuHist, 60);
  const diskName = () => (disk() ? `${disk()!.disk.letter}:` : t("cards.sd.disk", "Disk"));

  const rows = () => [
    { k: "cpu", label: t("cards.system.cpu", "CPU"), val: pct(v().cpu ?? NaN), sub: (() => { const s = histStats(v().cpuHist); return s ? t("cards.sd.avgPeak", "ø {avg} · špička {peak}", { avg: pct(s.avg), peak: pct(s.peak) }) : ""; })(), tip: cpuTip(), open: true },
    { k: "mem", label: t("cards.system.mem", "Paměť"), val: pct(v().memPct), sub: t("cards.system.memOf", "{used} z {total}", { used: size(v().memUsed), total: size(v().memTotal) }), tip: memTip(), open: true },
    ...(disk() ? [{ k: "disk", label: diskName(), val: pct(diskPct()!), sub: t("cards.system.diskFree", "{free} volné", { free: size(disk()!.disk.free) }), tip: diskTip(), open: false }] : []),
  ];

  const Row = (r: ReturnType<typeof rows>[number]): JSX.Element => (
    <button class="sd-lrow" data-k={r.k} title={r.tip} onClick={r.open ? p.onOpen : undefined} disabled={!r.open}>
      <i class="sd-key" aria-hidden="true" />
      <span class="sd-l">{r.label}</span>
      <b class="sd-v">{r.val}</b>
      <small class="sd-sub sd-wide">{r.sub}</small>
    </button>
  );

  return (
    <div class="sysd" data-design={p.design}>
      <Switch>
        <Match when={p.design === "rings"}>
          <div class="sd-rings">
            <div class="sd-fig">
              <Rings
                label={t("cards.sd.rings.label", "CPU, paměť a disk")}
                rings={[
                  { frac: v().cpu != null ? v().cpu! / 100 : null, tone: tone(cpuLoad()), title: cpuTip() },
                  { frac: v().memPct / 100, tone: tone(memLoad()), title: memTip() },
                  { frac: diskPct() != null ? diskPct()! / 100 : null, tone: diskTone(), title: diskTip() },
                ]}
              >
                <b class="sd-big">{pct(v().cpu ?? NaN)}</b>
                <small class="sd-cap">{t("cards.system.cpu", "CPU")}</small>
              </Rings>
            </div>
            <div class="sd-legend">
              <For each={rows()}>{Row}</For>
            </div>
          </div>
        </Match>
        <Match when={p.design === "gauges"}>
          <div class="sd-gauges">
            <For each={[{ k: "cpu", pct: v().cpu, tone: tone(cpuLoad()), tip: cpuTip(), label: t("cards.system.cpu", "CPU"), sub: "", open: true }, { k: "mem", pct: v().memPct, tone: tone(memLoad()), tip: memTip(), label: t("cards.system.mem", "Paměť"), sub: `${size(v().memUsed)} / ${size(v().memTotal)}`, open: true }, ...(disk() ? [{ k: "disk", pct: diskPct(), tone: diskTone(), tip: diskTip(), label: diskName(), sub: t("cards.system.diskFree", "{free} volné", { free: size(disk()!.disk.free) }), open: false }] : [])]}>
              {(g) => (
                <button class="sd-gauge" title={g.tip} onClick={g.open ? p.onOpen : undefined} disabled={!g.open}>
                  <Gauge frac={g.pct != null ? g.pct / 100 : null} tone={g.tone} title={g.tip} label={g.tip}>
                    <b class="sd-big">{pct(g.pct ?? NaN)}</b>
                  </Gauge>
                  <span class="sd-l">{g.label}</span>
                  <Show when={g.sub}><small class="sd-sub sd-wide">{g.sub}</small></Show>
                </button>
              )}
            </For>
          </div>
        </Match>
        <Match when={p.design === "bars"}>
          <div class="sd-bars">
            <button class="sd-bars-top" onClick={p.onOpen} title={cpuTip()}>
              <b class="sd-big" data-load={cpuLoad()}>{pct(v().cpu ?? NaN)}</b>
              <span class="sd-l">{t("cards.system.cpu", "CPU")}</span>
              <Show when={histStats(v().cpuHist)}>{(s) => <small class="sd-sub sd-wide">{t("cards.sd.avgPeak", "ø {avg} · špička {peak}", { avg: pct(s().avg), peak: pct(s().peak) })}</small>}</Show>
            </button>
            <Show when={hist().length} fallback={<div class="sd-hist-empty">{t("cards.sd.filling", "Historie se teprve plní…")}</div>}>
              <div class="sd-hist" role="img" aria-label={t("cards.sd.bars.label", "Zátěž CPU v posledních vzorcích")}>
                <For each={hist()}>
                  {(b, i) => (
                    <i classList={{ "sd-old": hist().length - i() > 30 }} data-load={b.load} style={{ height: `${Math.max(4, Math.round(b.h * 100))}%` }} title={t("cards.sd.bars.tip", "CPU {pct} · před {sec} s", { pct: pct(b.pct), sec: b.age })} />
                  )}
                </For>
              </div>
            </Show>
            <div class="sd-foot">
              <span title={memTip()}>{t("cards.system.mem", "Paměť")} <b data-load={memLoad()}>{pct(v().memPct)}</b></span>
              <Show when={disk()}><span title={diskTip()}>{diskName()} <b data-load={diskLoad(disk()!.disk.free, disk()!.disk.total)}>{pct(diskPct()!)}</b></span></Show>
              <span class="sd-wide" title={netTip()}>↓ {rate(v().rx)} ↑ {rate(v().tx)}</span>
            </div>
          </div>
        </Match>
        <Match when={p.design === "terminal"}>
          <div class="sd-term" role="table">
            <For each={rows()}>
              {(r) => {
                const n = () => (r.k === "cpu" ? v().cpu : r.k === "mem" ? v().memPct : diskPct());
                return (
                  <button class="sd-trow" title={r.tip} onClick={r.open ? p.onOpen : undefined} disabled={!r.open}>
                    <span class="sd-tk">{r.k === "disk" ? diskName() : r.label}</span>
                    <span class="sd-cells" data-load={r.k === "disk" ? diskLoad(disk()!.disk.free, disk()!.disk.total) : loadOf(n())}>
                      <span class="sd-narrow">[{termCells(n(), 10)}]</span>
                      <span class="sd-wide">[{termCells(n(), 20)}]</span>
                    </span>
                    <b class="sd-v">{pct(n() ?? NaN)}</b>
                    <small class="sd-sub sd-wide">{r.sub}</small>
                  </button>
                );
              }}
            </For>
            <div class="sd-trow" title={netTip()}>
              <span class="sd-tk">{t("cards.system.net", "Síť")}</span>
              <span class="sd-cells">↓ {rate(v().rx)}</span>
              <b class="sd-v">↑ {rate(v().tx)}</b>
            </div>
          </div>
        </Match>
        <Match when={p.design === "tiles"}>
          <div class="sd-tiles">
            <button class="sd-tile" title={cpuTip()} onClick={p.onOpen} data-load={cpuLoad()}>
              <b class="sd-big">{pct(v().cpu ?? NaN)}</b>
              <span class="sd-l">{t("cards.system.cpu", "CPU")}</span>
              <Spark values={v().cpuHist} max={100} load={cpuLoad()} title={cpuTip()} />
              <Show when={histStats(v().cpuHist)}>{(s) => <small class="sd-sub sd-wide">{t("cards.sd.avgPeak", "ø {avg} · špička {peak}", { avg: pct(s().avg), peak: pct(s().peak) })}</small>}</Show>
            </button>
            <button class="sd-tile" title={memTip()} onClick={p.onOpen} data-load={memLoad()}>
              <b class="sd-big">{pct(v().memPct)}</b>
              <span class="sd-l">{t("cards.system.mem", "Paměť")}</span>
              <Spark values={v().memHist} max={100} load={memLoad()} title={memTip()} />
              <small class="sd-sub sd-wide">{size(v().memUsed)} / {size(v().memTotal)}</small>
            </button>
            <div class="sd-tile" title={netTip()}>
              <b class="sd-big">{rate(v().rx)}</b>
              <span class="sd-l">↓ {t("cards.system.down", "Příjem")}</span>
              <Spark values={v().rxHist} max={netMax()} title={netTip()} />
              <small class="sd-sub sd-wide">↑ {rate(v().tx)}</small>
            </div>
          </div>
        </Match>
        <Match when={p.design === "net"}>
          <div class="sd-net">
            <div class="sd-net-big" title={netTip()}>
              <span><small>↓ {t("cards.system.down", "Příjem")}</small><b class="sd-big">{rate(v().rx)}</b></span>
              <span class="sd-up"><small>↑ {t("cards.system.up", "Odesílání")}</small><b class="sd-big">{rate(v().tx)}</b></span>
            </div>
            <Spark values={v().rxHist} second={{ values: v().txHist }} max={netMax()} title={netTip()} />
            <div class="sd-foot">
              <button class="sd-foot-b" onClick={p.onOpen} title={cpuTip()}>{t("cards.system.cpu", "CPU")} <b data-load={cpuLoad()}>{pct(v().cpu ?? NaN)}</b></button>
              <button class="sd-foot-b" onClick={p.onOpen} title={memTip()}>{t("cards.system.mem", "Paměť")} <b data-load={memLoad()}>{pct(v().memPct)}</b></button>
              <Show when={disk()}><span title={diskTip()}>{diskName()} <b>{pct(diskPct()!)}</b></span></Show>
            </div>
          </div>
        </Match>
      </Switch>
    </div>
  );
}
