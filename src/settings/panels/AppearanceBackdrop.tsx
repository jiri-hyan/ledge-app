// Studio → Vzhled šuplíku → Pozadí (kolo 25): galerie pozadí otevřeného šuplíku jako dlaždice rozdělené na
// Klidná / Gradienty / Živá / Vlastní. Živá pozadí mají živý mini náhled — uvnitř dlaždice jen jeden statický
// snímek, animace hraje JEN při najetí / fokusu na dlaždici (po odjetí smyčka stojí). Výběr mění rozpracovaný
// config (platí hned na šuplíku, vrací se přes historii Studia); „Žádné“ = dnešní vzhled. Pod galerií: barva
// plamene, síla a rychlost živého pozadí, editor vlastního gradientu (`GradientEditor.tsx`).
import { For, Show, onCleanup } from "solid-js";
import { t } from "../../i18n";
import type { Backdrop, CardsMode, FlameColor, LiveKind } from "../../types";
import { lbl } from "../../theme/label";
import { CARDS_MODES, DEFAULT_BACKDROP, FLAMES, MAX_CUSTOM_GRADIENTS, addGradient, gradientCss, gradientOf, normalizeBackdrop } from "../../theme/backdrop";
import { FLAME_NAMES, LIVE_PRESETS, PRESETS, asGradient, presetById, type Preset } from "../../theme/backdropPresets";
import { createLoop } from "../../theme/backdropLoop";
import { LIVE_FPS, STILL_WARMUP, createLive } from "../../theme/backdropLive";
import { Seg, Slider, type Ap } from "./apUi";
import GradientEditor from "./GradientEditor";
import "./backdropUi.css";

/** Mini náhled živého pozadí: při načtení jeden statický snímek, animace jen při najetí myší / fokusu. */
function LiveMini(q: { kind: LiveKind; bd: Backdrop }) {
  let cv!: HTMLCanvasElement;
  let stop = () => {};
  const unit = () => {
    const ctx = cv.getContext("2d")!;
    return createLive(q.kind, ctx, cv.width, cv.height, { intensity: Math.max(0.6, q.bd.intensity), speed: q.bd.speed, flame: q.bd.flame });
  };
  const still = () => {
    const u = unit();
    for (let i = 0; i <= STILL_WARMUP[q.kind]; i++) u.frame(5 + i / 30, 1 / 30);
  };
  const play = () => {
    stop();
    const u = unit();
    const loop = createLoop({ now: () => performance.now(), request: (cb) => requestAnimationFrame(cb), cancel: (h) => cancelAnimationFrame(h), frame: (tt, dt) => u.frame(tt, dt) }, Math.min(LIVE_FPS[q.kind], 24));
    loop.start();
    stop = () => (loop.stop(), (stop = () => {}));
  };
  const rest = () => (stop(), still());
  onCleanup(() => stop());
  const reduced = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  return (
    <canvas
      class="bd-mini" width={80} height={36} ref={(el) => { cv = el; queueMicrotask(still); }}
      onPointerEnter={() => !reduced && play()} onPointerLeave={rest}
      onFocusIn={() => !reduced && play()} onFocusOut={rest}
    />
  );
}

function Tile(q: { on: boolean; name: string; desc: string; css: string; live?: LiveKind; bd: Backdrop; onPick: () => void }) {
  return (
    <button class="ap-tile bd-tile" classList={{ on: q.on }} role="radio" aria-checked={q.on} title={q.desc} onClick={q.onPick}>
      <span class="ap-swatch bd-sw" style={{ "background-image": q.css }}>
        <Show when={q.live}>{(k) => <LiveMini kind={k()} bd={q.bd} />}</Show>
      </span>
      <span class="ap-name">{q.name}</span>
      <small class="bd-desc">{q.desc}</small>
    </button>
  );
}

const CARD_NAMES: Record<CardsMode, () => string> = {
  auto: () => t("appearance.bd.cards.auto", "Automaticky"),
  solid: () => t("appearance.bd.cards.solid", "Plné"),
  tint: () => t("appearance.bd.cards.tint", "Zabarvené"),
  glass: () => t("appearance.bd.cards.glass", "Sklo"),
};
const NONE_CSS = "linear-gradient(180deg, var(--tint-solid), var(--tint-solid))";
const cssOf = (p: Preset) => gradientCss(asGradient(p));

export default function AppearanceBackdrop(q: { ap: Ap }) {
  const bd = (): Backdrop => normalizeBackdrop(q.ap.a().backdrop);
  const put = (n: Backdrop) => q.ap.set("backdrop", n);
  const sel = () => bd();
  const g = () => gradientOf(sel());
  const customSel = () => sel().custom.find((c) => c.id === sel().id) ?? null;
  const pickGrad = (id: string) => put({ ...sel(), id, live: "" });
  const pickLive = (id: LiveKind, base: string) => put({ ...sel(), live: id, id: sel().live === id && sel().id ? sel().id : base });
  const none = () => put({ ...DEFAULT_BACKDROP, custom: sel().custom });
  const calm = PRESETS.filter((p) => p.group === "calm");
  const grads = PRESETS.filter((p) => p.group === "gradient");
  const full = () => sel().custom.length >= MAX_CUSTOM_GRADIENTS;
  const create = (from?: Preset) => {
    const r = addGradient(sel().custom, from ? t("appearance.bd.copyOf", "{name} (vlastní)", { name: lbl(from.name) }) : t("appearance.bd.newName", "Vlastní gradient {n}", { n: sel().custom.length + 1 }), from ? asGradient(from) : { kind: "linear", angle: 160, stops: [{ c: "#1b2a5a", p: 0 }, { c: "#0a0e1c", p: 100 }] });
    if (r) put({ ...sel(), custom: r.list, id: r.id, live: "" });
  };
  const noneOn = () => !sel().id && !sel().live;
  const name = () => {
    const s = sel();
    const live = LIVE_PRESETS.find((l) => l.id === s.live);
    if (live) return lbl(live.name);
    const c = s.custom.find((x) => x.id === s.id);
    if (c) return c.name;
    const p = presetById(s.id);
    return p ? lbl(p.name) : t("appearance.bd.none", "Žádné (výchozí)");
  };

  return (
    <div class="card ap-top">
      <h4>{t("appearance.bd.title", "Pozadí šuplíku")}<span class="n">{name()}</span></h4>
      <p class="ap-note">{t("appearance.bd.note", "Pozadí je za kartami. Text zůstává čitelný: pod kartami se podle barev přidá ztmavovací závoj. Živá pozadí se kreslí jen při otevřeném šuplíku.")}</p>

      <div class="opt">
        <span>{t("appearance.bd.cards", "Karty nad pozadím")}<small>{t("appearance.bd.cards.hint", "Automaticky: s pozadím jsou karty průsvitné, bez něj beze změny. Sklo navíc rozmaže pozadí pod kartou.")}</small></span>
        <Seg<CardsMode> label={t("appearance.bd.cards", "Karty nad pozadím")} value={sel().cards} set={(v) => put({ ...sel(), cards: v })} options={CARDS_MODES.map((m) => [m, CARD_NAMES[m]()])} />
      </div>

      <div class="bd-group">{t("appearance.bd.g.calm", "Klidná")}</div>
      <div class="ap-themes" role="radiogroup" aria-label={t("appearance.bd.g.calm", "Klidná")}>
        <Tile on={noneOn()} name={t("appearance.bd.none", "Žádné (výchozí)")} desc={t("appearance.bd.none.d", "Dnešní vzhled, beze změny.")} css={NONE_CSS} bd={sel()} onPick={none} />
        <For each={calm}>{(p) => <Tile on={sel().id === p.id && !sel().live} name={lbl(p.name)} desc={lbl(p.desc)} css={cssOf(p)} bd={sel()} onPick={() => pickGrad(p.id)} />}</For>
      </div>

      <div class="bd-group">{t("appearance.bd.g.grad", "Gradienty")}</div>
      <div class="ap-themes" role="radiogroup" aria-label={t("appearance.bd.g.grad", "Gradienty")}>
        <For each={grads}>{(p) => <Tile on={sel().id === p.id && !sel().live} name={lbl(p.name)} desc={lbl(p.desc)} css={cssOf(p)} bd={sel()} onPick={() => pickGrad(p.id)} />}</For>
      </div>

      <div class="bd-group">{t("appearance.bd.g.live", "Živá")}</div>
      <div class="ap-themes" role="radiogroup" aria-label={t("appearance.bd.g.live", "Živá")}>
        <For each={LIVE_PRESETS}>
          {(l) => <Tile on={sel().live === l.id} name={lbl(l.name)} desc={lbl(l.desc)} css={(() => { const p = presetById(l.base); return p ? cssOf(p) : NONE_CSS; })()} live={l.id} bd={sel()} onPick={() => pickLive(l.id, l.base)} />}
        </For>
      </div>
      <Show when={sel().live}>
        <div class="bd-live">
          <Show when={sel().live === "fire"}>
            <div class="opt">
              <span>{t("appearance.bd.flame", "Barva plamene")}<small>{t("appearance.bd.flame.hint", "Klasický oheň, modrý plyn, zelený nebo fialový.")}</small></span>
              <Seg<FlameColor> label={t("appearance.bd.flame", "Barva plamene")} value={sel().flame} set={(v) => put({ ...sel(), flame: v })} options={FLAMES.map((f) => [f, lbl(FLAME_NAMES[f])])} />
            </div>
          </Show>
          <Slider label={t("appearance.bd.intensity", "Síla")} min={0.1} max={1} step={0.05} value={sel().intensity} fmt={(v) => `${Math.round(v * 100)} %`} set={(v) => put({ ...sel(), intensity: v })} />
          <Slider label={t("appearance.bd.speed", "Rychlost")} min={0.3} max={2} step={0.1} value={sel().speed} fmt={(v) => `${v.toFixed(1)}×`} set={(v) => put({ ...sel(), speed: v })} />
          <p class="ap-note">{t("appearance.bd.reduced", "S nastavením Windows „omezit pohyb“ se místo animace ukáže jeden statický snímek.")}</p>
        </div>
      </Show>

      <div class="bd-group">{t("appearance.bd.g.custom", "Vlastní")}</div>
      <div class="ap-themes" role="radiogroup" aria-label={t("appearance.bd.g.custom", "Vlastní")}>
        <For each={sel().custom}>
          {(c) => <Tile on={sel().id === c.id && !sel().live} name={c.name} desc={t("appearance.bd.custom.d", "Tvůj gradient.")} css={gradientCss(c)} bd={sel()} onPick={() => pickGrad(c.id)} />}
        </For>
        <button class="ap-tile bd-tile bd-new" disabled={full()} onClick={() => create()} title={full() ? t("appearance.bd.full", "Je jich 20, víc nejde.") : undefined}>
          <span class="ap-swatch bd-sw bd-plus">+</span>
          <span class="ap-name">{t("appearance.bd.new", "Nový gradient")}</span>
          <small class="bd-desc">{t("appearance.bd.new.d", "2–5 barev, vlastní úhel a druh.")}</small>
        </button>
      </div>
      <Show when={!customSel() && g() && !sel().live}>
        <div class="opt">
          <span>{t("appearance.bd.edit", "Upravit tento gradient")}<small>{t("appearance.bd.edit.d", "Vytvoří vlastní kopii, kterou můžeš přebarvit.")}</small></span>
          <button class="btn" disabled={full()} onClick={() => create(presetById(sel().id))}>{t("appearance.bd.edit.btn", "Upravit jako vlastní")}</button>
        </div>
      </Show>
      <Show when={customSel()}>{(c) => <GradientEditor ap={q.ap} bd={sel()} g={c()} />}</Show>
    </div>
  );
}
