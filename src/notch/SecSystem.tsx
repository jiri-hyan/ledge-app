// Sekce lišty notche „Systém“ (kolo 27) ve vzhledech Linky (dnešní) · Plochy · Sloupky · Prstence · Velká čísla ·
// Pruhy jako htop. Volby: barva (akcent / podle zátěže / přechod / hex), tloušťka a vyhlazení čáry, metriky
// CPU / RAM / Disk / Síť. Data jen z `intApi.systemView` vzorkovaného po 2 s JEN s otevřeným šuplíkem (zavřený =
// žádný časovač); graf až od dvou skutečných vzorků, bez dat prázdný stav. Tooltip (title) nese hodnotu.
import { For, Match, Show, Switch, createEffect, createSignal, createUniqueId, onCleanup } from "solid-js";
import { t } from "../i18n";
import { drawerShown } from "../media/shown";
import { intApi, type SystemView } from "../integrations/api";
import { fmtPct, fmtRate, loadOf } from "../integrations/format";
import { ring } from "../charts/geom";
import { chartPaths, htopBar, lastBars, paintOf, solidOf, weightPx, GRADIENT_CSS, type Paint } from "./look";
import { metricsOf, type BarSection, type Metric } from "./bar";

/** Odstup vzorků systému (stejně jako karta Systém). */
const EVERY_MS = 2000;

/** Vzorky systému jen s otevřeným šuplíkem. */
function useSystemView() {
  const [view, setView] = createSignal<SystemView | null>(null);
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
  return view;
}

type Row = { key: Metric; name: string; pct: number | null; hist?: number[]; max?: number; text: string };

function rowsOf(v: SystemView, want: readonly Metric[]): Row[] {
  const disk = v.disks.find((x) => x.system) ?? v.disks[0];
  const diskPct = disk && disk.total > 0 ? ((disk.total - disk.free) / disk.total) * 100 : null;
  const all: Record<Metric, Row | null> = {
    cpu: v.cpu != null ? { key: "cpu", name: "CPU", pct: v.cpu, hist: v.cpuHist, max: 100, text: fmtPct(v.cpu) } : null,
    ram: v.memTotal > 0 ? { key: "ram", name: "RAM", pct: v.memPct, hist: v.memHist, max: 100, text: fmtPct(v.memPct) } : null,
    disk: diskPct != null ? { key: "disk", name: t("cards.notch.disk", "Disk"), pct: diskPct, text: fmtPct(diskPct) } : null,
    net:
      v.rx != null || v.tx != null
        ? { key: "net", name: t("cards.notch.net", "Síť"), pct: null, hist: v.rxHist, text: `↓ ${fmtRate(v.rx, v.netUnit)}  ↑ ${fmtRate(v.tx, v.netUnit)}` }
        : null,
  };
  return want.map((m) => all[m]).filter((r): r is Row => !!r);
}

/** Mini graf z reálných vzorků (čára, volitelně plocha); přechod = SVG gradient. */
function Chart(p: { values: number[]; max?: number; smooth: boolean; weight?: string; paint: Paint; area: boolean }) {
  const id = createUniqueId();
  const g = () => chartPaths(p.values, 100, 24, p.smooth, p.max);
  const stroke = () => (p.paint.kind === "gradient" ? `url(#nbg${id})` : p.paint.css);
  return (
    <Show when={g()} fallback={<span class="nb-spark" />}>
      {(x) => (
        <svg class="nb-spark" viewBox="0 0 100 24" preserveAspectRatio="none" aria-hidden="true">
          <Show when={p.paint.kind === "gradient"}>
            <defs>
              <linearGradient id={`nbg${id}`} x1="0" x2="1" y1="0" y2="0">
                <stop offset="0" style={{ "stop-color": "var(--accent)" }} />
                <stop offset="1" style={{ "stop-color": "var(--accent-2)" }} />
              </linearGradient>
            </defs>
          </Show>
          <Show when={p.area}><path d={x().area} style={{ fill: stroke(), opacity: 0.22 }} /></Show>
          <path d={x().line} fill="none" style={{ stroke: stroke() }} stroke-width={weightPx(p.weight)} stroke-linecap="round" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
        </svg>
      )}
    </Show>
  );
}

/** Výplň pruhu / sloupku jako CSS pozadí. */
const fill = (p: Paint) => (p.kind === "gradient" ? GRADIENT_CSS : p.css);

export default function System(p: { sec: BarSection; design: string }) {
  const view = useSystemView();
  const rows = () => (view() ? rowsOf(view()!, metricsOf(p.sec)) : []);
  /** Dnešní vzhled bez volby barvy: akcent, přetížení jantar (jako dřív). */
  const paint = (r: Row): Paint => (!p.sec.color && r.pct != null && loadOf(r.pct) !== "ok" ? { kind: "solid", css: "var(--amber)" } : paintOf(p.sec.color, r.pct));
  const tip = (r: Row) => `${r.name} ${r.text}`;
  /** Barva čísla / textu: jen se zvolenou barvou (výchozí = barva textu); přechod = akcent. */
  const tone = (r: Row) => (p.sec.color && r.pct != null ? solidOf(p.sec.color, r.pct) : undefined);
  return (
    <section class="nb-sec nb-sys" data-design={p.design} aria-label={t("cards.notch.system", "Systém")}>
      <Show when={rows().length} fallback={<small class="nb-empty">{t("cards.notch.sysWait", "Čekám na první vzorek systému…")}</small>}>
        <Switch>
          <Match when={p.design === "rings"}>
            <div class="nb-tiles">
              <For each={rows()}>
                {(r) => (
                  <div class="nb-tile" title={tip(r)}>
                    <Show when={r.pct != null} fallback={<b class="nb-net">{r.text}</b>}>
                      <RingOf pct={r.pct!} paint={paint(r)} weight={p.sec.weight} />
                    </Show>
                    <small>{r.name}</small>
                  </div>
                )}
              </For>
            </div>
          </Match>
          <Match when={p.design === "big"}>
            <div class="nb-tiles">
              <For each={rows()}>
                {(r) => (
                  <div class="nb-tile nb-bignum" title={tip(r)}>
                    <b style={{ color: tone(r) }} classList={{ "nb-net": r.pct == null }}>{r.pct != null ? Math.round(r.pct) : r.text}<Show when={r.pct != null}><i>%</i></Show></b>
                    <small>{r.name}</small>
                  </div>
                )}
              </For>
            </div>
          </Match>
          <Match when={p.design === "htop"}>
            <pre class="nb-htop">
              <For each={rows()}>
                {(r) => (
                  <span title={tip(r)} style={{ color: tone(r) }}>
                    {`${r.name.padEnd(4).slice(0, 4)} ${r.pct != null ? `${htopBar(r.pct, 12)} ${r.text}` : r.text}\n`}
                  </span>
                )}
              </For>
            </pre>
          </Match>
          <Match when={true}>
            <For each={rows()}>
              {(r) => (
                <div class="nb-row" data-load={loadOf(r.pct)} title={tip(r)}>
                  <span class="nb-rn">{r.name}</span>
                  <Show
                    when={r.hist && r.hist.length >= 2}
                    fallback={<span class="nb-bar"><i style={{ width: `${Math.min(100, r.pct ?? 0)}%`, background: fill(paint(r)) }} /></span>}
                  >
                    <Show
                      when={p.design === "bars"}
                      fallback={<Chart values={r.hist!} max={r.max} smooth={!!p.sec.smooth} weight={p.sec.weight} paint={paint(r)} area={p.design === "area"} />}
                    >
                      <span class="nb-cols" aria-hidden="true">
                        <For each={lastBars(r.hist!, 24, r.max ?? Math.max(1, ...r.hist!))}>{(h) => <i style={{ height: `${Math.max(6, h * 100)}%`, background: fill(paint(r)) }} />}</For>
                      </span>
                    </Show>
                  </Show>
                  <b class="nb-rv">{r.key === "net" ? `↓ ${fmtRate(view()?.rx, view()!.netUnit)}` : r.text}</b>
                </div>
              )}
            </For>
          </Match>
        </Switch>
      </Show>
    </section>
  );
}

function RingOf(p: { pct: number; paint: Paint; weight?: string }) {
  const id = createUniqueId();
  const sw = () => (p.weight === "thin" ? 3 : p.weight === "bold" ? 7 : 5);
  const g = () => ring(p.pct / 100, 52, sw());
  const stroke = () => (p.paint.kind === "gradient" ? `url(#nbr${id})` : p.paint.css);
  return (
    <svg class="nb-ring" viewBox="0 0 52 52" aria-hidden="true">
      <Show when={p.paint.kind === "gradient"}>
        <defs>
          <linearGradient id={`nbr${id}`} x1="0" x2="1" y1="0" y2="1">
            <stop offset="0" style={{ "stop-color": "var(--accent)" }} />
            <stop offset="1" style={{ "stop-color": "var(--accent-2)" }} />
          </linearGradient>
        </defs>
      </Show>
      <Show when={g()}>
        {(r) => (
          <>
            <circle cx="26" cy="26" r={r().r} fill="none" class="nb-track" stroke-width={sw()} />
            <circle cx="26" cy="26" r={r().r} fill="none" style={{ stroke: stroke() }} stroke-width={sw()} stroke-dasharray={r().dash} stroke-linecap="round" transform="rotate(-90 26 26)" />
          </>
        )}
      </Show>
      <text x="26" y="30" text-anchor="middle" class="nb-ringv">{Math.round(p.pct)}</text>
    </svg>
  );
}
