// Úpravy lišty notche v režimu ✎ (kolo 27): rozpracovaná sekce (živý náhled v liště, Vrátit / Hotovo jako panel
// „Vzhled karty…“ — `layout/lookState.ts`), přidat / odebrat / přeskládat. Všechno jde do draftu rozložení přes
// `change` (jeden krok Zpět); odvozená lišta se při první změně převede na vlastní (`bar.ts` `materialize`).
// Sekce se cílí podle id (na liště je každá jednou), ne indexem — odvozená lišta bez dat má jiné indexy.
import { createSignal } from "solid-js";
import { change } from "../layout/edit";
import type { LayoutPage } from "../layout/model";
import { addSection, materialize, moveSection, patchSection, removeSection, withBar, type BarId, type BarSection, type NotchBar } from "./bar";

export type BarTarget = { page: string; id: BarId; initial: BarSection };
export const [barTarget, setBarTarget] = createSignal<BarTarget | null>(null);
export const [barDraft, setBarDraft] = createSignal<BarSection | null>(null);
/** otevřená nabídka „+“ (id záložky) */
export const [barAdding, setBarAdding] = createSignal<string | null>(null);

/** Lišta záložky v úpravách: vlastní, nebo ta, která by z karet vznikla (jako by data byla). */
export const editBar = (page: LayoutPage): NotchBar => materialize(page.notchBar, page.cards.map((c) => c.id)).bar;

/** Lišta k vykreslení s rozpracovanou sekcí (živý náhled). */
export function barWithDraft(pageId: string, bar: NotchBar): NotchBar {
  const tg = barTarget();
  const d = barDraft();
  if (!tg || tg.page !== pageId || !d) return bar;
  return { sections: bar.sections.map((s) => (s.id === tg.id ? d : s)) };
}

/** Jedna změna lišty v draftu (s převodem odvozené lišty na vlastní; karty zůstávají v configu). */
function edit(page: LayoutPage, f: (b: NotchBar) => NotchBar) {
  const m = materialize(page.notchBar, page.cards.map((c) => c.id));
  change((l) => withBar(l, page.id, f(m.bar)));
}

const indexOf = (b: NotchBar, id: string) => b.sections.findIndex((s) => s.id === id);

export function openSection(page: LayoutPage, id: BarId) {
  if (barTarget()) closeSection(page, true);
  const s = editBar(page).sections.find((x) => x.id === id);
  if (!s) return;
  setBarAdding(null);
  setBarTarget({ page: page.id, id, initial: s });
  setBarDraft(s);
}

/** Změna voleb rozpracované sekce (normalizovaná — výchozí se nezapisuje). */
export function patchDraft(patch: Partial<Record<keyof BarSection, unknown>>) {
  const d = barDraft();
  if (d) setBarDraft(patchSection({ sections: [d] }, 0, patch).sections[0]);
}

/** Hotovo / klik mimo = do draftu rozložení; Esc = zahodit. */
export function closeSection(page: LayoutPage | null, save: boolean) {
  const tg = barTarget();
  const d = barDraft();
  setBarTarget(null);
  setBarDraft(null);
  if (!save || !tg || !d || !page || page.id !== tg.page || JSON.stringify(d) === JSON.stringify(tg.initial)) return;
  edit(page, (b) => ({ sections: b.sections.map((s) => (s.id === tg.id ? d : s)) }));
}

export function removeBarSection(page: LayoutPage, id: BarId) {
  setBarTarget(null);
  setBarDraft(null);
  edit(page, (b) => removeSection(b, indexOf(b, id)));
}

export function addBarSection(page: LayoutPage, id: BarId) {
  setBarAdding(null);
  edit(page, (b) => addSection(b, id));
}

export function moveBarSection(page: LayoutPage, from: number, to: number) {
  edit(page, (b) => moveSection(b, from, to));
}

export function applyBarPreset(page: LayoutPage, preset: NotchBar) {
  setBarTarget(null);
  setBarDraft(null);
  setBarAdding(null);
  change((l) => withBar(l, page.id, preset));
}

export function resetBarToAuto(page: LayoutPage) {
  setBarTarget(null);
  setBarDraft(null);
  setBarAdding(null);
  change((l) => withBar(l, page.id, undefined));
}
