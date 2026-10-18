// Společné ovládací prvky panelu Vzhled (Appearance*.tsx): přepínač `Seg` (nejvýš 4 volby — víc = galerie,
// hlídá `settings/segments.test.mjs`), posuvník s hodnotou `Slider`, rozbalovací „Pokročilé“ `Advanced`,
// přepínač ukázkového dění `StateSwitch` (náhledy kroků Zavřený a Po najetí), zvýraznění kotvy hledání `lit`
// a formáty hodnot. Jen vzhled — stav drží panel.
import { For, Show, createSignal, type JSX } from "solid-js";
import { lang, t } from "../../i18n";
import { PREVIEW_STATES, type PreviewState } from "../appearanceModel";
import type { Appearance } from "../../types";
import type { PanelProps } from "../types";
import { hit } from "../ctx";

/** Stav panelu Vzhled pro jeho části: rozpracovaný vzhled, změna pole, rozmazání (`drawer.glass`). */
export type Ap = {
  p: PanelProps;
  a: () => Appearance;
  set: <K extends keyof Appearance>(k: K, v: Appearance[K]) => void;
  /** celý vzhled naráz (výběr stylu) */
  setAll: (a: Appearance) => void;
  blur: () => boolean;
  setBlur: (on: boolean) => void;
};

/** Kotva hledání je `id="f-<klíč>"`; tohle = krátké zvýraznění po skoku z hledání (jako `Row`). */
export const lit = (id: string) => hit() === id;

export function Seg<T extends string>(q: { label: string; value: T; options: [T, string][]; set: (v: T) => void }) {
  return (
    <div class="seg" role="radiogroup" aria-label={q.label}>
      <For each={q.options}>
        {([v, l]) => (
          <button classList={{ on: q.value === v }} role="radio" aria-checked={q.value === v} onClick={() => q.set(v)}>{l}</button>
        )}
      </For>
    </div>
  );
}

export function Slider(q: { label: string; hint?: string; anchor?: string; min: number; max: number; step: number; value: number; fmt: (v: number) => string; set: (v: number) => void }) {
  return (
    <div class="opt" id={q.anchor ? `f-${q.anchor}` : undefined} classList={{ hit: !!q.anchor && lit(q.anchor) }}>
      <span>{q.label}<Show when={q.hint}><small>{q.hint}</small></Show></span>
      <span class="ap-range">
        <input type="range" min={q.min} max={q.max} step={q.step} value={q.value} aria-label={q.label} onInput={(e) => q.set(+e.currentTarget.value)} />
        <output>{q.fmt(q.value)}</output>
      </span>
    </div>
  );
}

/** Rozbalovací „Pokročilé“ kroku: otevřené drží panel (skok z hledání na volbu uvnitř ho rozbalí). */
export function Advanced(q: { id?: string; open: boolean; onToggle: (open: boolean) => void; hint: string; children: JSX.Element }) {
  return (
    <details class="card ap-details ap-top" id={q.id ? `f-${q.id}` : undefined} classList={{ hit: !!q.id && lit(q.id) }} open={q.open}
      onToggle={(e) => e.currentTarget.open !== q.open && q.onToggle(e.currentTarget.open)}>
      <summary><h4>{t("appearance.adv.title", "Pokročilé")}</h4><small>{q.hint}</small></summary>
      {q.children}
    </details>
  );
}

/** Co se v náhledech zrovna „děje“ — sdílí kroky Zavřený a Po najetí (přepnutí kroku ho nezahodí). */
const [previewState, setPreviewState] = createSignal<PreviewState>("work");
export { previewState };

const STATE_LABEL: Record<PreviewState, () => string> = {
  idle: () => t("appearance.state.idle", "Klid"),
  work: () => t("appearance.state.work", "Claude pracuje"),
  wait: () => t("appearance.state.wait", "Čeká na tebe"),
  music: () => t("appearance.state.music", "Hraje hudba"),
  timer: () => t("appearance.state.timer", "Časovač"),
  done: () => t("appearance.state.done", "Hotovo"),
};

/** Čipy „Co se děje“ nad náhledy (ukázková data, nic skutečného). */
export function StateSwitch() {
  return (
    <div class="ap-states" role="radiogroup" aria-label={t("appearance.state.title", "Co se zrovna děje (ukázka)")}>
      <span class="ap-states-l">{t("appearance.state.title", "Co se zrovna děje (ukázka)")}</span>
      <For each={PREVIEW_STATES}>
        {(s) => (
          <button class="mini" classList={{ "ap-chip-on": previewState() === s }} role="radio" aria-checked={previewState() === s} onClick={() => setPreviewState(s)}>
            {STATE_LABEL[s]()}
          </button>
        )}
      </For>
    </div>
  );
}

export const pct = (v: number) => (lang() === "cs" ? `${Math.round(v * 100)} %` : `${Math.round(v * 100)}%`);
export const times = (v: number) => (lang() === "cs" ? `${v.toFixed(1).replace(".", ",")}×` : `${v.toFixed(1)}×`);
