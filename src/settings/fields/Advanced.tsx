// Sdílená sbalitelná část „Pokročilé“ panelů Nastavení: technické volby (porty, repa, příkazy, klíče) sem,
// ať základ panelu zůstane srozumitelný. Jedna implementace pro všechny panely (Vzhled má vlastní kroky
// v `apUi.tsx`). Rozbalení si pamatuje úložiště prohlížeče po `id` (try/catch — bez něj je vše sbalené);
// skok z hledání nebo odjinud na volbu uvnitř (`hit`) oddíl rozbalí, ale do paměti nezapíše.
import { createEffect, createSignal, on, Show, type JSX } from "solid-js";
import { t } from "../../i18n";
import { hit } from "../ctx";
import { ADV_KEY, parseAdvOpen, withAdv } from "../advModel";

const read = () => {
  try {
    return parseAdvOpen(localStorage.getItem(ADV_KEY));
  } catch {
    return {};
  }
};
const [stored, setStored] = createSignal<Record<string, true>>(read());

function remember(id: string, open: boolean) {
  const next = withAdv(stored(), id, open);
  setStored(next);
  try {
    localStorage.setItem(ADV_KEY, JSON.stringify(next));
  } catch {
    // paměť je jen pohodlí
  }
}

/** `id` = klíč paměti (panel, případně `panel.oddíl`), `hint` = co je uvnitř (jedna věta pod nadpisem),
 *  `title` = jiný nadpis než „Pokročilé“ (panel Osobní: „Další sekce…“), `inline` = uvnitř jiné sekce (bez vlastní karty). */
export function Advanced(p: { id: string; title?: string; hint?: string; inline?: boolean; children: JSX.Element }) {
  let box!: HTMLDetailsElement;
  /** rozbaleno skokem na volbu uvnitř */
  const [forced, setForced] = createSignal(false);
  const open = () => !!stored()[p.id] || forced();
  createEffect(
    on(hit, (h) => {
      const id = (h ?? "").replace(/[^\w.-]/g, "");
      if (id && box?.querySelector(`[id="f-${id}"]`)) setForced(true);
    }),
  );
  return (
    <details
      ref={box}
      class="st-adv"
      classList={{ inline: p.inline }}
      open={open()}
      onToggle={(e) => {
        const now = e.currentTarget.open;
        if (now === open()) return;
        setForced(false);
        remember(p.id, now);
      }}
    >
      <summary>
        <b>{p.title ?? t("settings.adv.title", "Pokročilé")}</b>
        <Show when={p.hint}><small>{p.hint}</small></Show>
      </summary>
      <div class="st-adv-body">{p.children}</div>
    </details>
  );
}
