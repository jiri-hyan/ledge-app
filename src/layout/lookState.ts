// Stav „Vzhled karty…“ (⋯ na kartě) bez UI: cíl, rozpracovaný vzhled, `lookFor` (živý náhled na kartě),
// `openCardLook` / `closeCardLook`. Malý eager modul — `CardMenu`, `Deck`, `MainPage` a `VariantPicker` ho potřebují
// hned, kdežto panel `CardLook.tsx` (jen když je otevřený) se načítá líně (`lazyViews.tsx`).
import { createSignal } from "solid-js";
import { change, editing, saveLayout } from "./edit";
import { normalizeCardLook, setCardLook, type CardLook } from "./look";

export type Target = { page: string; card: string; initial: CardLook | undefined; cfgError: string | null };
export const [target, setTarget] = createSignal<Target | null>(null);
export const [draft, setDraft] = createSignal<CardLook | undefined>(undefined);

/** Vzhled, který má karta kreslit: rozpracovaný (živý náhled), jinak uložený. */
export const lookFor = (page: string, card: string, saved: CardLook | undefined): CardLook | undefined => {
  const tg = target();
  return tg && tg.page === page && tg.card === card ? draft() : saved;
};

/** ⋯ → „Vzhled karty…“. */
export function openCardLook(page: string, card: string, initial: CardLook | undefined, cfgError: string | null) {
  const cur = target();
  if (cur) void closeCardLook(true);
  const init = normalizeCardLook(initial, card);
  setTarget({ page, card, initial: init, cfgError });
  setDraft(init);
}

export async function closeCardLook(save: boolean) {
  const tg = target();
  const l = draft();
  setTarget(null);
  setDraft(undefined);
  if (!save || !tg || JSON.stringify(l ?? null) === JSON.stringify(tg.initial ?? null)) return;
  if (editing()) change((x) => setCardLook(x, tg.page, tg.card, l));
  else await saveLayout((x) => setCardLook(x, tg.page, tg.card, l), tg.cfgError);
}
