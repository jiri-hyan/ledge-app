// Vzhled po krocích (vlna 7) — od kola 24 (Ledge Studio) každý krok žije v jiné části Studia jako vlastní
// otázka (`studio/parts.ts`): Zavřený (`AppearanceClosed.tsx`: galerie vzhledů zavřeného Ledge, pozice, velikost,
// „Co proužek ukazuje“) → Proužek, Po najetí (`AppearanceHover.tsx`) → Otevírání, Otevřený (`AppearanceOpen.tsx`:
// styly, motiv, šířka, hustota; v Doladit barvy, písmo, rohy, pozadí) a Pohyb (`AppearanceMotion.tsx`) → Vzhled
// šuplíku. `AppearanceStep` vykreslí jeden krok; rozpracovaný vzhled na dokument aplikuje Studio (`Studio.tsx`),
// najetí na dlaždici (`apUi.tsx` `peek`) jen scéna Studia (`studio/Scene.tsx`). Skok z hledání (`hit`) rozbalí
// Doladit svého kroku (`appearanceModel.ts` `stepOf`/`isAdvanced`). Kotvy hledání `f-<id>` = klíče z `labels.ts`.
import { Match, Switch, createEffect, createSignal, on } from "solid-js";
import type { PanelProps } from "../types";
import type { Appearance } from "../../types";
import { DEFAULT_APPEARANCE, normalizeAppearance } from "../../theme/appearance";
import { savedAppearance } from "../../theme/apply";
import { hit } from "../ctx";
import { isAdvanced, stepOf, type StepId } from "../appearanceModel";
import AppearanceClosed from "./AppearanceClosed";
import AppearanceHover from "./AppearanceHover";
import AppearanceOpen from "./AppearanceOpen";
import AppearanceMotion from "./AppearanceMotion";
import type { Ap } from "./apUi";
import "./appearance.css";

/** Rozbalená Doladit každého kroku přežijí přepnutí části (modul). */
const [advOpen, setAdvOpen] = createSignal<Partial<Record<StepId, boolean>>>({});

/** Jeden krok Vzhledu jako otázka Studia. */
export function AppearanceStep(p: PanelProps & { step: StepId }) {
  const a = (): Appearance => normalizeAppearance(p.cfg.appearance);
  /** Změna jednoho pole vzhledu (chybí-li sekce ve starém configu, založí ji s výchozími hodnotami). */
  function set<K extends keyof Appearance>(k: K, v: Appearance[K]) {
    if (!p.cfg.appearance) p.setCfg("appearance", structuredClone(DEFAULT_APPEARANCE));
    p.setCfg("appearance", k, structuredClone(v));
  }
  // skok z hledání / odjinud na volbu v Doladit tohoto kroku → rozbalit
  const follow = (h: string | null) => {
    if (stepOf(h) === p.step && isAdvanced(h)) setAdvOpen((o) => ({ ...o, [p.step]: true }));
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
  const adv = () => !!advOpen()[p.step];
  const setAdv = (v: boolean) => setAdvOpen((o) => ({ ...o, [p.step]: v }));
  return (
    <div class="ap">
      <Switch>
        <Match when={p.step === "closed"}><AppearanceClosed ap={ap} adv={adv()} setAdv={setAdv} /></Match>
        <Match when={p.step === "hover"}><AppearanceHover ap={ap} adv={adv()} setAdv={setAdv} /></Match>
        <Match when={p.step === "open"}><AppearanceOpen ap={ap} dirty={dirty()} adv={adv()} setAdv={setAdv} /></Match>
        <Match when={p.step === "motion"}><AppearanceMotion ap={ap} reduced={reduced} /></Match>
      </Switch>
    </div>
  );
}

/** Celý Vzhled (katalog stránek): krok Otevřený — ostatní kroky vykreslí Studio ve svých částech. */
export default function AppearancePanel(p: PanelProps) {
  return <AppearanceStep {...p} step="open" />;
}
