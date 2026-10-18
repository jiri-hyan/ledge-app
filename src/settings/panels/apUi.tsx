// Společné ovládací prvky panelu Vzhled (Appearance*.tsx): přepínač `Seg`, posuvník s hodnotou
// `Slider`, zvýraznění kotvy hledání `lit` a formáty hodnot. Jen vzhled — stav drží panel.
import { For, Show } from "solid-js";
import { lang } from "../../i18n";
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

export const pct = (v: number) => (lang() === "cs" ? `${Math.round(v * 100)} %` : `${Math.round(v * 100)}%`);
export const times = (v: number) => (lang() === "cs" ? `${v.toFixed(1).replace(".", ",")}×` : `${v.toFixed(1)}×`);
