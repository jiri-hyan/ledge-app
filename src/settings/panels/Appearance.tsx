// Panel Nastavení „Vzhled“ (vlna 7): nahoře 4 kroky jako záložky — Zavřený (`AppearanceClosed.tsx`: galerie
// vzhledů zavřeného Ledge, pozice a velikost proužku; v Pokročilé tvar, linka, krytí a celé „Co proužek
// ukazuje“), Po najetí (`AppearanceHover.tsx`: 3 dlaždice `drawer.hover`), Otevřený (`AppearanceOpen.tsx`:
// galerie stylů, monitor ve skutečném poměru, šířka a hustota; v Pokročilé barvy, rozložení, vlastní šířka,
// písmo, rohy, pozadí) a Pohyb (`AppearanceMotion.tsx`: 7 animací s ukázkou + rychlost).
// Každý krok je hlavně galerie; žádný přepínač nemá víc než 4 volby. Skok z hledání (`hit`) přepne krok
// a rozbalí jeho Pokročilé (`appearanceModel.ts` `stepOf`/`isAdvanced`).
// Každá změna se hned náhledově aplikuje na celý šuplík (`applyAppearance`); odchod bez uložení vrátí
// uložený vzhled. Ukládá JEN společná lišta Nastavení (Uložit / Zahodit / Vrátit) → Rust pošle `appearance`
// a překreslí proužek. Pozadí (`drawer.glass`), šířka (`drawer.width`), animace a celý proužek se nastavují
// JEN tady — panel Chování má jen chování. Kotvy hledání `f-<id>` = klíče z `labels.ts`.
// Jazyk rozhraní (`ui.lang`) tu už není — je v panelu Začínáme úplně nahoře (přepne se po Uložit, `ctx.loadBase` → `setLang`).
import { For, Match, Switch, createEffect, createSignal, on, onCleanup } from "solid-js";
import type { PanelProps } from "../types";
import type { Appearance } from "../../types";
import { t } from "../../i18n";
import { DEFAULT_APPEARANCE, normalizeAppearance } from "../../theme/appearance";
import { applyAppearance, savedAppearance } from "../../theme/apply";
import { hit } from "../ctx";
import { STEPS, isAdvanced, stepOf, type StepId } from "../appearanceModel";
import AppearanceClosed from "./AppearanceClosed";
import AppearanceHover from "./AppearanceHover";
import AppearanceOpen from "./AppearanceOpen";
import AppearanceMotion from "./AppearanceMotion";
import { peek, withThemeDraft, type Ap } from "./apUi";
import "./appearance.css";

const STEP_LABEL: Record<StepId, () => string> = {
  closed: () => t("appearance.step.closed", "Zavřený"),
  hover: () => t("appearance.step.hover", "Po najetí"),
  open: () => t("appearance.step.open", "Otevřený"),
  motion: () => t("appearance.step.motion", "Pohyb"),
};
const STEP_HINT: Record<StepId, () => string> = {
  closed: () => t("appearance.step.closedHint", "Co je vidět u horní hrany"),
  hover: () => t("appearance.step.hoverHint", "Když najedeš myší"),
  open: () => t("appearance.step.openHint", "Celý šuplík"),
  motion: () => t("appearance.step.motionHint", "Jak se otevírá"),
};

/** Poslední krok a rozbalená Pokročilé přežijí přepnutí panelu (modul). */
const [lastStep, setLastStep] = createSignal<StepId>("closed");
const [advOpen, setAdvOpen] = createSignal<Partial<Record<StepId, boolean>>>({});

export default function AppearancePanel(p: PanelProps) {
  const a = (): Appearance => normalizeAppearance(p.cfg.appearance);
  /** Změna jednoho pole vzhledu (chybí-li sekce ve starém configu, založí ji s výchozími hodnotami). */
  function set<K extends keyof Appearance>(k: K, v: Appearance[K]) {
    if (!p.cfg.appearance) p.setCfg("appearance", structuredClone(DEFAULT_APPEARANCE));
    p.setCfg("appearance", k, structuredClone(v));
  }

  // náhled na celém šuplíku hned (najetí na dlaždici = dočasný náhled `peek`, rozpracovaný vlastní motiv z editoru);
  // odchod bez uložení → zpět uložený vzhled
  createEffect(() => applyAppearance(peek() ?? withThemeDraft(a())));
  onCleanup(() => applyAppearance(savedAppearance()));

  // skok z hledání / odjinud: přepnout krok a rozbalit jeho Pokročilé
  const follow = (h: string | null) => {
    const s = stepOf(h);
    if (!s) return;
    setLastStep(s);
    if (isAdvanced(h)) setAdvOpen((o) => ({ ...o, [s]: true }));
  };
  follow(hit());
  createEffect(on(hit, follow, { defer: true }));

  const dirty = () => JSON.stringify(a()) !== JSON.stringify(savedAppearance());
  const reduced = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const blur = () => p.cfg.drawer?.glass === "acrylic";
  const ap: Ap = {
    p,
    a,
    set,
    setAll: (next) => p.setCfg("appearance", structuredClone(next)),
    blur,
    // jen skutečná změna (výběr stylu bez rozmazání nezapíše `drawer.glass`, který v configu chybí)
    setBlur: (on) => {
      if (blur() !== on) p.setCfg("drawer", (d: any) => ({ ...(d ?? {}), glass: on ? "acrylic" : "solid" }));
    },
  };
  const adv = (s: StepId) => !!advOpen()[s];
  const setAdv = (s: StepId) => (v: boolean) => setAdvOpen((o) => ({ ...o, [s]: v }));
  const key = (e: KeyboardEvent, i: number) => {
    const d = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const n = (i + d + STEPS.length) % STEPS.length;
    setLastStep(STEPS[n]);
    ((e.currentTarget as HTMLElement).parentElement?.children[n] as HTMLElement | undefined)?.focus();
  };

  return (
    <div class="ap">
      <div class="ap-steps" role="tablist" aria-label={t("appearance.steps", "Vzhled po krocích")}>
        <For each={STEPS}>
          {(s, i) => (
            <button class="ap-step" classList={{ on: lastStep() === s }} role="tab" aria-selected={lastStep() === s} tabIndex={lastStep() === s ? 0 : -1}
              onClick={() => setLastStep(s)} onKeyDown={(e) => key(e, i())}>
              <b>{STEP_LABEL[s]()}</b>
              <small>{STEP_HINT[s]()}</small>
            </button>
          )}
        </For>
      </div>

      <Switch>
        <Match when={lastStep() === "closed"}><AppearanceClosed ap={ap} adv={adv("closed")} setAdv={setAdv("closed")} /></Match>
        <Match when={lastStep() === "hover"}><AppearanceHover ap={ap} /></Match>
        <Match when={lastStep() === "open"}><AppearanceOpen ap={ap} dirty={dirty()} adv={adv("open")} setAdv={setAdv("open")} /></Match>
        <Match when={lastStep() === "motion"}><AppearanceMotion ap={ap} reduced={reduced} /></Match>
      </Switch>
    </div>
  );
}
