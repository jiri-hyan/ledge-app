// Široký notch (rozložení `notch`, kolo 22; kolo 27 nastavitelný): lišta sekcí nahoře v šuplíku. Bez vlastního
// seznamu (`layout.pages[].notchBar` chybí) kreslí jako dřív sekce odvozené z karet záložky (`bar.ts` `barPlan`);
// s vlastním seznamem kreslí jeho sekce v jejich vzhledech (`designs.ts`) a karty nepřebírá. Sekce jsou v
// `Sec*.tsx`, data jen z existujících zdrojů; systém se vzorkuje jen s otevřeným šuplíkem. Fokus nekrade.
// Režim ✎: lišta je náhled k úpravám — klik na sekci = panel vzhledu (`BarPanel.tsx`), tah = pořadí (stejný motor
// jako záložky, `layout/tabDrag.ts` + `flip.ts`), „+“ = přidat sekci; ovládání uvnitř sekcí v úpravách nereaguje.
import { For, Match, Show, Switch, createMemo, onCleanup } from "solid-js";
import type { Snapshot } from "../api";
import { t } from "../i18n";
import type { LayoutPage } from "../layout/model";
import { createTabDrag } from "../layout/tabDrag";
import { flipEls } from "../layout/flip";
import { moveItem } from "../layout/model";
import { Player, Volume } from "./SecMedia";
import Days from "./SecTime";
import System from "./SecSystem";
import { Limits, Meeting, WeatherSec } from "./SecInfo";
import { barDesignOf, BAR_NAMES } from "./designs";
import { addable, type BarSection, type NotchBar as Bar } from "./bar";
import { barAdding, barTarget, barWithDraft, closeSection, editBar, moveBarSection, openSection, setBarAdding } from "./barState";
import { tx } from "../layout/text";
import "./notch.css";

import BarPanel from "./BarPanel";

function Section(p: { s: Snapshot; now: number; sec: BarSection }) {
  const design = () => barDesignOf(p.sec.id, p.sec.design);
  const cal = () => p.s.personal.calendar;
  return (
    <Switch>
      <Match when={p.sec.id === "player"}><Player s={p.s} sec={p.sec} design={design()} /></Match>
      <Match when={p.sec.id === "days"}><Days sec={p.sec} design={design()} now={p.now} cal={cal()} /></Match>
      <Match when={p.sec.id === "system"}><System sec={p.sec} design={design()} /></Match>
      <Match when={p.sec.id === "volume"}><Volume s={p.s} sec={p.sec} design={design()} /></Match>
      <Match when={p.sec.id === "limits"}><Limits s={p.s} now={p.now} sec={p.sec} design={design()} /></Match>
      <Match when={p.sec.id === "weather"}><WeatherSec s={p.s} sec={p.sec} design={design()} /></Match>
      <Match when={p.sec.id === "meeting"}><Meeting s={p.s} now={p.now} sec={p.sec} design={design()} /></Match>
    </Switch>
  );
}

/** Mimo úpravy: sekce plánu (vlastní nebo odvozené). */
export default function NotchBar(p: { s: Snapshot; now: number; sections: BarSection[] }) {
  return (
    <div class="nb" data-anim>
      <For each={p.sections}>{(sec) => <Section s={p.s} now={p.now} sec={sec} />}</For>
    </div>
  );
}

/** Režim ✎: lišta jako upravitelný náhled. */
export function NotchBarEdit(p: { s: Snapshot; now: number; page: LayoutPage }) {
  let box!: HTMLDivElement;
  const bar = createMemo<Bar>(() => barWithDraft(p.page.id, editBar(p.page)));
  const drag = createTabDrag({
    box: () => box,
    editing: () => true,
    begin: () => {},
    busy: () => !!barTarget(),
    flip: (update) => flipEls(Array.from(box.querySelectorAll<HTMLElement>(":scope > .nb-item")), update),
    commit: (from, to) => moveBarSection(p.page, from, to),
    sel: { item: "nb-item", attr: "data-sec", fly: (g) => g.classList.add("nb-fly") },
  });
  /** pořadí během tahu (CSS `order`), jinak podle seznamu */
  const order = (i: number) => {
    const d = drag.drag();
    if (!d) return i;
    return moveItem(bar().sections.map((_, j) => j), d.from, d.to).indexOf(i);
  };
  const adding = () => barAdding() === p.page.id;
  // konec úprav (Hotovo / Zrušit / jiná stránka): rozpracovaná sekce a nabídka „+“ zmizí (klik na Hotovo ji předtím uložil)
  onCleanup(() => (setBarAdding(null), closeSection(null, false)));
  return (
    <div class="nb-editwrap">
      <div class="nb nb-edit" ref={box} role="list" aria-label={t("cards.notch.editAria", "Lišta výřezu — klikni na sekci pro vzhled, táhni pro pořadí")}>
        <For each={bar().sections}>
          {(sec, i) => (
            <div
              class="nb-item"
              role="listitem"
              data-sec={sec.id}
              classList={{ on: barTarget()?.id === sec.id, dragging: drag.drag()?.id === sec.id }}
              style={{ order: order(i()) }}
              tabindex="0"
              title={tx(BAR_NAMES[sec.id])}
              onPointerDown={(e) => drag.down(e, sec.id, i())}
              onClick={() => openSection(p.page, sec.id)}
              onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), openSection(p.page, sec.id))}
            >
              <div class="nb-inert" inert>
                <Section s={p.s} now={p.now} sec={sec} />
              </div>
              <span class="nb-tag">{tx(BAR_NAMES[sec.id])}</span>
            </div>
          )}
        </For>
        <Show when={addable(bar()).length > 0}>
          <button class="nb-add" style={{ order: 99 }} aria-expanded={adding()} aria-label={t("cards.notch.add", "Přidat sekci do lišty")} title={t("cards.notch.add", "Přidat sekci do lišty")} onClick={() => (closeSection(p.page, true), setBarAdding(adding() ? null : p.page.id))}>
            +
          </button>
        </Show>
      </div>
      <Show when={adding() || barTarget()?.page === p.page.id}>
        <BarPanel page={p.page} bar={bar()} />
      </Show>
    </div>
  );
}
