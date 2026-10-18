// Karta Systém: CPU, paměť, volné místo na discích a rychlost sítě s malými grafy za posledních 60 vzorků.
// Čte Rust (`sysmon/`, příkaz `system_view`) a NIC nepočítá sama: každé číslo je skutečný vzorek z WinAPI.
// Vzorkuje se po 2 s JEN když je karta vidět a šuplík otevřený (zavřený = žádný časovač, žádné čtení).
// Kliknutí na CPU / paměť otevře Správce úloh.
import { createEffect, createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import { num } from "../format";
import { t } from "../i18n";
import { drawerShown } from "../media/shown";
import { sayError } from "../ui/toast";
import { intApi, type SystemView } from "./api";
import { diskLoad, diskUsed, fmtPct, fmtRate, fmtSize, loadOf, sparkPoints, type Load } from "./format";
import "./integrations.css";

/** Odstup vzorků karty (Rust nevezme dva vzorky blíž než 1,5 s). */
const EVERY_MS = 2000;
const W = 118;
const H = 26;
/** Nejmenší strop osy grafu sítě: šum pod 50 KB/s nevypadá jako provoz. */
const NET_FLOOR = 50_000;

/** Poslední vzorek přežije přemontování karty (přepnutí záložky) — jinak by blikla prázdná. */
const [view, setView] = createSignal<SystemView | null>(null);

function Spark(p: { values: number[]; max?: number; floor?: number; load?: Load }) {
  return (
    <svg class="sys-spark" classList={{ "sys-warm": p.load === "warm", "sys-hot": p.load === "hot" }} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden="true">
      <polyline points={sparkPoints(p.values, W, H, p.max, p.floor)} />
    </svg>
  );
}

async function openTaskManager() {
  try {
    await intApi.systemTaskmgr();
  } catch (e) {
    sayError(e);
  }
}

export default function SystemCard(p: { menu?: JSX.Element }) {
  createEffect(() => {
    if (!drawerShown()) return;
    let alive = true;
    let timer: number | undefined;
    const tick = async () => {
      try {
        const v = await intApi.systemView();
        if (alive && v) setView(v);
      } catch {
        /* jeden vzorek navíc nic neřeší — příště */
      }
      if (alive) timer = window.setTimeout(tick, EVERY_MS);
    };
    void tick();
    onCleanup(() => {
      alive = false;
      if (timer !== undefined) window.clearTimeout(timer);
    });
  });
  const cpuLoad = () => loadOf(view()?.cpu);
  const memLoad = () => loadOf(view()?.memPct);
  const unit = () => view()?.netUnit ?? "bytes";
  const netMax = () => {
    const v = view();
    return v ? Math.max(NET_FLOOR, ...v.rxHist, ...v.txHist) : NET_FLOOR;
  };
  const size = (n: number) => fmtSize(n, num);
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--teal)" />{t("cards.system.title", "Systém")}{p.menu}</h4>
      <Show when={view()} fallback={<div class="empty">{t("cards.system.loading", "Měřím…")}</div>}>
        {(v) => (
          <>
            <button class="sys-row sys-click" onClick={openTaskManager} title={t("cards.system.taskmgr", "Otevřít Správce úloh")}>
              <span class="sys-l">{t("cards.system.cpu", "CPU")}</span>
              <b class="sys-v" classList={{ "sys-warm": cpuLoad() === "warm", "sys-hot": cpuLoad() === "hot" }}>{fmtPct(v().cpu, num)}</b>
              <Spark values={v().cpuHist} max={100} load={cpuLoad()} />
            </button>
            <button class="sys-row sys-click" onClick={openTaskManager} title={t("cards.system.taskmgr", "Otevřít Správce úloh")}>
              <span class="sys-l">{t("cards.system.mem", "Paměť")}</span>
              <b class="sys-v" classList={{ "sys-warm": memLoad() === "warm", "sys-hot": memLoad() === "hot" }}>{fmtPct(v().memPct, num)}</b>
              <span class="sys-sub">{t("cards.system.memOf", "{used} z {total}", { used: size(v().memUsed), total: size(v().memTotal) })}</span>
              <Spark values={v().memHist} max={100} load={memLoad()} />
            </button>
            <For each={v().disks}>
              {(d) => (
                <div class="sys-row" title={t("cards.system.diskTitle", "Disk {letter}: {total} celkem", { letter: d.letter, total: size(d.total) })}>
                  <span class="sys-l">{d.letter}:</span>
                  <span class="sys-bar" role="img" aria-label={t("cards.system.diskFree", "{free} volné", { free: size(d.free) })}>
                    <i classList={{ "sys-warm": diskLoad(d.free, d.total) === "warm", "sys-hot": diskLoad(d.free, d.total) === "hot" }} style={{ width: `${Math.round(diskUsed(d.free, d.total) * 100)}%` }} />
                  </span>
                  <span class="sys-sub sys-free">{t("cards.system.diskFree", "{free} volné", { free: size(d.free) })}</span>
                </div>
              )}
            </For>
            <div class="sys-row">
              <span class="sys-l">{t("cards.system.net", "Síť")}</span>
              <span class="sys-net">
                <span class="sys-down" title={t("cards.system.down", "Příjem")}>↓ {fmtRate(v().rx, unit(), num)}</span>
                <span class="sys-up" title={t("cards.system.up", "Odesílání")}>↑ {fmtRate(v().tx, unit(), num)}</span>
              </span>
              <svg class="sys-spark" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden="true">
                <polyline class="sys-line-rx" points={sparkPoints(v().rxHist, W, H, netMax())} />
                <polyline class="sys-line-tx" points={sparkPoints(v().txHist, W, H, netMax())} />
              </svg>
            </div>
          </>
        )}
      </Show>
    </div>
  );
}
