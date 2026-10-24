// Stav „návrh zástupce z přetaženého“ (kolo 20): jádro pošle událost `drop-shortcut`, skořápka (`App.tsx`)
// návrh uloží sem a `MainPage` ukáže panel `DropNew`. Zrušení / přidání ho zase zahodí. Eager a malé —
// těžký editor je líný chunk (`lazyViews.tsx`).
import { createSignal } from "solid-js";
import { readPayload, type DropPayload } from "./dropshortcut.ts";

const [dropped, setDropped] = createSignal<DropPayload | null>(null);
export { dropped };

/** Událost z jádra → návrh (neplatný se zahodí). Vrací, jestli něco přijal. */
export function offerDrop(raw: unknown): boolean {
  const p = readPayload(raw);
  setDropped(p);
  return !!p;
}

export const clearDrop = () => setDropped(null);
