// Útrata Claude (odhad v cenách API, ne účet) — rozbalitelná sekce dole v kartě Claude. Data jen
// z `Snapshot.costs` (Rust `costslive.rs` nad lokálními transcripty Claude Code). Přepočet se žádá
// jen při rozbalení a při otevření šuplíku s rozbalenou sekcí (Rust ho pustí nejvýš 1× / 60 s).
// Model bez ceny v ceníku = „—“ (tokeny ano); během přepočtu bez dat „počítám…“, ne nuly.
import { createEffect, createSignal, For, on, Show } from "solid-js";
import { api, type Snapshot } from "../api";
import { ago, num } from "../format";
import { t } from "../i18n";
import { bars, compact, floorTrend, pickRange, progressPct, share, usdDigits, type CostLine, type FloorRow } from "./model";
import "./costs.css";

const OPEN_KEY = "ledge.costs.open";
const RANGE_KEY = "ledge.costs.range";

function load(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function store(key: string, v: string) {
  try {
    localStorage.setItem(key, v);
  } catch {
    /* jen pohodlí */
  }
}

/** „12,34 $“ / „$12.34“; bez ceny „—“. */
export function usd(v: number | null | undefined): string {
  if (v == null) return "—";
  return t("sessions.costs.usd", "{v} $", { v: num(v, usdDigits(v)) });
}

/** „3,4 M“ tokenů */
export function tok(n: number): string {
  const c = compact(n);
  return `${num(c.v, c.digits)}${c.unit ? ` ${c.unit}` : ""}`;
}

function Line(p: { l: CostLine; total: CostLine }) {
  const name = () => (p.l.other ? t("sessions.costs.other", "ostatní") : p.l.name || t("sessions.costs.noDir", "bez složky"));
  return (
    <div class="cost-line" title={p.l.unpriced ? t("sessions.costs.unpricedTip", "{n} tokenů bez ceny v ceníku", { n: tok(p.l.unpriced) }) : undefined}>
      <span class="cost-name">{name()}</span>
      <span class="cost-track"><span class="cost-fill" style={{ width: `${share(p.l, p.total) * 100}%` }} /></span>
      <span class="cost-tok">{tok(p.l.tokens)}</span>
      <span class="cost-usd">{usd(p.l.cost)}</span>
    </div>
  );
}

/** Řádek podlahy: název · medián kontextu 1. volání · počet souborů · trend proti předchozím 7 dnům. */
function FloorLine(p: { r: FloorRow }) {
  const tr = () => floorTrend(p.r);
  const trendText = () => {
    const x = tr();
    if (!x) return "";
    return x.dir === "flat" ? t("sessions.costs.floorFlat", "beze změny") : x.dir === "up" ? t("sessions.costs.floorUp", "▲ +{v}", { v: tok(x.abs) }) : t("sessions.costs.floorDown", "▼ −{v}", { v: tok(x.abs) });
  };
  const prev = () => (p.r.prevMedian == null ? t("sessions.costs.floorPrevNone", "žádné soubory") : t("sessions.costs.floorPrev", "medián {m} v {n} souborech", { m: tok(p.r.prevMedian), n: p.r.prevN }));
  return (
    <div class="cost-floor" title={t("sessions.costs.floorRowTip", "medián {m} tokenů v {n} souborech; předchozích 7 dní: {p}", { m: num(p.r.median), n: p.r.n, p: prev() })}>
      <span class="cost-name">{p.r.name || t("sessions.costs.noDir", "bez složky")}</span>
      <span class="cost-tok">{t("sessions.costs.floorCount", "{n}×", { n: p.r.n })}</span>
      <span class="cost-usd">{tok(p.r.median)}</span>
      <span class="cost-trend" classList={{ up: tr()?.dir === "up", down: tr()?.dir === "down" }}>{trendText()}</span>
    </div>
  );
}

export default function CostsSection(p: { s: Snapshot; now: number }) {
  const [open, setOpen] = createSignal(load(OPEN_KEY) === "1");
  const [days, setDays] = createSignal(Number(load(RANGE_KEY)) || 7);
  const cv = () => p.s.costs;
  // přepočet jen s rozbalenou sekcí v otevřeném šuplíku (Rust: 1× / 60 s, vlákno s nízkou prioritou);
  // závisí jen na rozbalení a pohledu — ne na každém snapshotu
  createEffect(on([open, () => p.s.view], ([o, v]) => {
    if (o && v === "full") void api.costsRefresh().catch(() => {});
  }));
  const toggle = () => {
    const o = !open();
    setOpen(o);
    store(OPEN_KEY, o ? "1" : "0");
  };
  const pick = (d: number) => {
    setDays(d);
    store(RANGE_KEY, String(d));
  };
  const range = () => pickRange(cv()?.summary?.ranges, days());
  const today = () => pickRange(cv()?.summary?.ranges, 1);
  const pct = () => progressPct(cv()?.progress);
  const heights = () => bars(cv()?.summary?.days ?? []);
  const RANGES = () => [
    [1, t("sessions.costs.today", "Dnes")],
    [7, t("sessions.costs.week", "7 dní")],
    [cv()?.windowDays ?? 30, t("sessions.costs.window", "{n} dní", { n: cv()?.windowDays ?? 30 })],
  ] as const;
  return (
    <Show when={cv() && cv()!.state !== "off"}>
      <div class="cost">
        <button class="cost-head" onClick={toggle} aria-expanded={open()}>
          <span class="cost-chev" classList={{ on: open() }}>›</span>
          <span class="cost-title" title={t("sessions.costs.tip", "Odhad podle ceníku API — při předplatném ji neplatíš.")}>{t("sessions.costs.title", "Hodnota v cenách API")}</span>
          <Show when={!open() && today()}>
            <span class="cost-peek">{t("sessions.costs.todayShort", "dnes {v}", { v: usd(today()!.total.cost) })}</span>
          </Show>
        </button>
        <Show when={open()}>
          <div class="cost-body">
            <div class="cost-tag" title={t("sessions.costs.tagTip", "Předplatné Pro/Max se po tokenech neúčtuje. Ceník API ověřen {date}: {src}", { date: cv()!.pricesVerified, src: cv()!.pricesSource })}>
              {t("sessions.costs.tag", "odhad v cenách API, ne účet")}
            </div>
            <Show when={cv()!.state === "computing"}>
              <div class="cost-note">{pct() != null ? t("sessions.costs.computingPct", "počítám… {n} %", { n: pct()! }) : t("sessions.costs.computing", "počítám…")}</div>
            </Show>
            <Show when={cv()!.state === "error"}>
              <div class="cost-note bad">{cv()!.error}</div>
            </Show>
            <Show when={range()} fallback={<Show when={cv()!.state === "idle"}><div class="cost-note">{t("sessions.costs.computing", "počítám…")}</div></Show>}>
              {(r) => (
                <>
                  <div class="cost-tabs">
                    <For each={RANGES()}>{([d, label]) => <button class="cost-tab" classList={{ on: r().days === d }} onClick={() => pick(d)}>{label}</button>}</For>
                  </div>
                  <div class="cost-sum">
                    <span class="cost-big">{usd(r().total.cost)}</span>
                    <span class="cost-sub">{t("sessions.costs.tokens", "{n} tokenů", { n: tok(r().total.tokens) })}</span>
                  </div>
                  <div class="cost-split">
                    {t("sessions.costs.split", "vstup {i} · výstup {o} · zápis cache {w} · čtení cache {r}", { i: tok(r().split.input), o: tok(r().split.output), w: tok(r().split.cacheWrite), r: tok(r().split.cacheRead) })}
                  </div>
                  <Show when={r().total.unpriced > 0}>
                    <div class="cost-note">{t("sessions.costs.unpriced", "{n} tokenů bez ceny (model chybí v ceníku)", { n: tok(r().total.unpriced) })}</div>
                  </Show>
                  <div class="cost-bars" role="img" aria-label={t("sessions.costs.barsLabel", "Útrata po dnech")}>
                    <For each={cv()!.summary!.days}>
                      {(d, i) => (
                        <span class="cost-col" title={`${d.date}: ${usd(d.cost)} · ${tok(d.tokens)}`}>
                          <span class="cost-bar" classList={{ dim: cv()!.summary!.days.length - i() > r().days }} style={{ height: `${Math.max(heights()[i()] ?? 0, 0) * 100}%` }} />
                        </span>
                      )}
                    </For>
                  </div>
                  <Show when={r().projects.length > 0} fallback={<div class="cost-note">{t("sessions.costs.empty", "Za tohle období žádné odpovědi Clauda.")}</div>}>
                    <div class="cost-h">{t("sessions.costs.projects", "Projekty")}</div>
                    <For each={r().projects}>{(l) => <Line l={l} total={r().total} />}</For>
                    <div class="cost-h">{t("sessions.costs.models", "Modely")}</div>
                    <For each={r().models}>{(l) => <Line l={l} total={r().total} />}</For>
                  </Show>
                  <Show when={cv()!.summary?.floor}>
                    {(fl) => (
                      <>
                        <div class="cost-h" title={t("sessions.costs.floorTip", "Medián kontextu prvního volání API každé session / agenta za posledních 7 dní; trend proti 7 dnům před tím (od 3 souborů v obou). Nafouklý CLAUDE.md, MCP nebo skills je vidět tady.")}>{t("sessions.costs.floorProjects", "Start nové relace (první volání)")}</div>
                        <For each={fl().projects}>{(r) => <FloorLine r={r} />}</For>
                        <Show when={fl().agents.length > 0}>
                          <div class="cost-h">{t("sessions.costs.floorAgents", "Start agenta podle typu (první volání)")}</div>
                          <For each={fl().agents}>{(r) => <FloorLine r={r} />}</For>
                        </Show>
                        <Show when={fl().projects.length === 0 && fl().agents.length === 0}>
                          <div class="cost-note">{t("sessions.costs.floorEmpty", "Za 7 dní žádná nová session ani agent.")}</div>
                        </Show>
                      </>
                    )}
                  </Show>
                  <Show when={cv()!.computedAt}>
                    <div class="cost-foot">{t("sessions.costs.computed", "spočítáno {when}", { when: ago(cv()!.computedAt!, p.now) })}</div>
                  </Show>
                </>
              )}
            </Show>
          </div>
        </Show>
      </div>
    </Show>
  );
}
