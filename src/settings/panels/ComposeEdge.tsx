// Živá hrana vlastního zavřeného Ledge v editoru Kostky (kolo 28, `strip.compose.deco`): efekt hrany, barevná
// sada (bez duhy) nebo vlastní 2–3 barvy, délka, tloušťka, síla, rychlost, „hýbat se i v klidu“ (stojí CPU)
// a sekce „Podle stavu Clauda“: pro každý stav efekt nebo jen stálá hrana + „Vyzkoušet“ (proužek na
// `TRY_MS` přes `edge_try` a stejný stav v náhledu tady). Úpravy přes `commit` editoru.
import { For, Show, createSignal, onCleanup } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { sayError } from "../../ui/toast";
import type { StripCompose } from "../../strip/compose";
import { BASE_EDGES, COMPOSE_EDGES, EDGE_SETS, MOVING, SET_COLORS, TRY_MS, type ComposeEdge, type EdgeLen, type EdgeSet, type LiveState } from "../../strip/edgefx";
import ComposePreview from "../previews/ComposePreview";
import { Seg, Slider, lit } from "./apUi";

export const EDGE_NAME: Record<ComposeEdge, () => string> = {
  none: () => t("appearance.compose.edge.none", "Žádný"),
  underline: () => t("appearance.compose.edge.underline", "Linka dole"),
  outline: () => t("appearance.compose.edge.outline", "Obrys"),
  glow: () => t("appearance.compose.edge.glow", "Záře"),
  gradient: () => t("appearance.compose.edge.gradient", "Přechod"),
  flow: () => t("appearance.compose.edge.flow", "Přelévání"),
  dashes: () => t("appearance.compose.edge.dashes", "Proužky"),
  comet: () => t("appearance.compose.edge.comet", "Kometa"),
  light: () => t("appearance.compose.edge.light", "Rozsvícení"),
  breath: () => t("appearance.compose.edge.breath", "Dech"),
  outlineGradient: () => t("appearance.compose.edge.outlineGradient", "Obrys s přechodem"),
  sweep: () => t("appearance.compose.edge.sweep", "Naplnění"),
};
const SET_NAME: Record<EdgeSet, () => string> = {
  aurora: () => t("appearance.compose.set.aurora", "Polární záře"),
  sunset: () => t("appearance.compose.set.sunset", "Západ slunce"),
  ocean: () => t("appearance.compose.set.ocean", "Oceán"),
  embers: () => t("appearance.compose.set.embers", "Žhavé uhlíky"),
  clay: () => t("appearance.compose.set.clay", "Claude (oranžová)"),
};
const LEN_NAME: Record<EdgeLen, () => string> = {
  center: () => t("appearance.compose.len.center", "Střed 40 %"),
  wide: () => t("appearance.compose.len.wide", "Široká 80 %"),
  full: () => t("appearance.compose.len.full", "Celá šířka"),
  custom: () => t("appearance.compose.len.custom", "Vlastní"),
};
const STATE_NAME: Record<Exclude<LiveState, "idle">, () => string> = {
  work: () => t("appearance.compose.live.work", "Pracuje"),
  wait: () => t("appearance.compose.live.wait", "Čeká na tebe"),
  done: () => t("appearance.compose.live.done", "Hotovo"),
  limit: () => t("appearance.compose.live.limit", "Limit od 80 %"),
};
const LIVE_ROWS = ["work", "wait", "done", "limit"] as const;
/** Svit pod tělem (síla dává smysl). */
const HALO: readonly ComposeEdge[] = ["glow", "light", "breath", "flow", "comet", "sweep"];

type Q = { c: StripCompose; commit: (c: StripCompose, key?: string) => void };

/** Rozbalovací výběr. */
function Pick<T extends string>(q: { label: string; value: T; options: [T, string][]; set: (v: T) => void }) {
  return (
    <select aria-label={q.label} value={q.value} onChange={(e) => q.set(e.currentTarget.value as T)}>
      <For each={q.options}>{([v, l]) => <option value={v}>{l}</option>}</For>
    </select>
  );
}

/** Akcent hrany: efekt, barvy, délka, síla, rychlost a živá hrana podle stavu Clauda. */
export function EdgeBlock(q: Q) {
  const d = () => q.c.deco;
  const set = (patch: Partial<StripCompose["deco"]>, key = "") => q.commit({ ...q.c, deco: { ...d(), ...patch } }, key);
  const setLive = (patch: Partial<StripCompose["deco"]["live"]>) => set({ live: { ...d().live, ...patch } });
  const [tried, setTried] = createSignal<LiveState | undefined>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(timer));
  const tryIt = (s: LiveState) => {
    setTried(s);
    clearTimeout(timer);
    timer = setTimeout(() => setTried(undefined), TRY_MS);
    api.edgeTry(s, d()).catch(sayError);
  };
  const isHex = () => d().color.startsWith("#");
  const colorMode = () => (isHex() ? "#" : d().color);
  const used = () => [d().edge, ...(d().live.on ? LIVE_ROWS.map((s) => d().live[s]) : [])];
  const any = (list: readonly ComposeEdge[]) => used().some((e) => list.includes(e));
  const custom = t("appearance.compose.colorCustom", "Vlastní");
  return (
    <>
      <div class="opt"><span>{t("appearance.compose.edge", "Akcent hrany")}<small>{t("appearance.compose.edgeHint", "Římsa pod proužkem: linka, záře nebo živý efekt.")}</small></span>
        <Pick label={t("appearance.compose.edge", "Akcent hrany")} value={d().edge} options={BASE_EDGES.map((v) => [v, EDGE_NAME[v]()])} set={(v) => set({ edge: v })} /></div>
      <Show when={d().edge !== "none" || d().live.on}>
        <div class="opt"><span>{t("appearance.compose.edgeColor", "Barva")}<small>{t("appearance.compose.edgeColorHint", "Sada barev teče podél hrany. Podle Clauda = barva stavu.")}</small></span>
          <span class="cx-col">
            <Pick label={t("appearance.compose.edgeColor", "Barva")} value={colorMode()}
              options={[["accent", t("appearance.compose.colorAccent", "Akcent")], ["claude", t("appearance.compose.colorClaude", "Podle Clauda")], ...EDGE_SETS.map((s): [string, string] => [s, SET_NAME[s]()]), ["#", custom]]}
              set={(v) => set({ color: v === "#" ? (isHex() ? d().color : "#ff8800") : v }, "edgeColor")} />
            <Show when={SET_COLORS[d().color as EdgeSet]}>{(c) => <i class="cx-sw" style={{ background: `linear-gradient(90deg, ${c().join(", ")})` }} aria-hidden="true" />}</Show>
            <Show when={isHex()}>
              <input type="color" value={d().color} aria-label={custom} onInput={(e) => set({ color: e.currentTarget.value }, "edgeColor")} />
              <input type="color" value={d().color2 || "#ffffff"} aria-label={t("appearance.compose.edgeColor2", "Druhá barva")} onInput={(e) => set({ color2: e.currentTarget.value }, "edgeColor2")} />
              <input type="color" value={d().color3 || "#ffffff"} aria-label={t("appearance.compose.edgeColor3", "Třetí barva")} onInput={(e) => set({ color3: e.currentTarget.value }, "edgeColor3")} />
              <Show when={d().color3}><button type="button" class="mini" onClick={() => set({ color3: "" })}>{t("appearance.compose.edgeColor3Off", "Jen dvě")}</button></Show>
            </Show>
          </span></div>
        <div class="opt"><span>{t("appearance.compose.len", "Délka linky")}</span>
          <Seg label={t("appearance.compose.len", "Délka linky")} value={d().len} options={(["center", "wide", "full", "custom"] as const).map((v): [EdgeLen, string] => [v, LEN_NAME[v]()])} set={(v) => set({ len: v })} /></div>
        <Show when={d().len === "custom"}>
          <Slider label={t("appearance.compose.pct", "Vlastní délka")} min={10} max={100} step={5} value={d().pct} fmt={(v) => `${v} %`} set={(v) => set({ pct: v }, "edgePct")} />
        </Show>
        <Slider label={t("appearance.compose.edgeWidth", "Tloušťka linky")} min={1} max={3} step={1} value={d().width} fmt={(v) => `${v} px`} set={(v) => set({ width: v }, "edgeWidth")} />
        <Show when={any(HALO)}>
          <Slider label={t("appearance.compose.power", "Síla svitu")} min={1} max={3} step={1} value={d().power} fmt={(v) => [t("appearance.compose.power.1", "jemná"), t("appearance.compose.power.2", "střední"), t("appearance.compose.power.3", "silná")][v - 1]} set={(v) => set({ power: v }, "edgePower")} />
        </Show>
        <Show when={any(MOVING)}>
          <Slider label={t("appearance.compose.speed", "Rychlost")} min={1} max={5} step={1} value={d().speed} fmt={(v) => `${v}/5`} set={(v) => set({ speed: v }, "edgeSpeed")} />
        </Show>
        <Show when={MOVING.includes(d().edge)}>
          <label class="opt"><span>{t("appearance.compose.always", "Hýbat se i v klidu")}<small>{t("appearance.compose.alwaysHint", "Jinak se efekt hýbe jen, když Claude pracuje nebo čeká. Stále = trochu CPU navíc (asi jako přehrávání animace).")}</small></span>
            <input type="checkbox" checked={d().always} onChange={(e) => set({ always: e.currentTarget.checked })} /></label>
        </Show>
      </Show>

      <div class="cx-live" id="f-strip.compose.deco.live" classList={{ hit: lit("strip.compose.deco.live") }}>
        <label class="opt"><span>{t("appearance.compose.live", "Podle stavu Clauda")}<small>{t("appearance.compose.liveHint", "Hrana ukáže, co Claude dělá. Jen ze skutečných stavů; ve hře, při Nerušit a bez animací nic nehraje.")}</small></span>
          <input type="checkbox" checked={d().live.on} onChange={(e) => setLive({ on: e.currentTarget.checked })} /></label>
        <For each={LIVE_ROWS}>
          {(s) => (
            <div class="opt" classList={{ "cx-off": !d().live.on }}><span>{STATE_NAME[s]()}</span>
              <span class="cx-col">
                <Pick label={STATE_NAME[s]()} value={d().live[s]} options={COMPOSE_EDGES.map((v) => [v, v === "none" ? t("appearance.compose.live.base", "Jen stálá hrana") : EDGE_NAME[v]()])} set={(v) => setLive({ [s]: v })} />
                <button type="button" class="mini" onClick={() => tryIt(s)}>{t("appearance.compose.live.try", "Vyzkoušet")}</button>
              </span></div>
          )}
        </For>
        <Show when={tried()}>
          {(s) => (
            <div class="cx-try" aria-live="polite">
              <ComposePreview compose={q.c} state="idle" opacity={1} fx={s()} />
              <small>{t("appearance.compose.live.trying", "Ukázka: {state}", { state: s() === "idle" ? "" : STATE_NAME[s() as Exclude<LiveState, "idle">]() })}</small>
            </div>
          )}
        </Show>
      </div>
    </>
  );
}
