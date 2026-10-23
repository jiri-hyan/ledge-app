// Části karet (sekce) bez DOM (test `parts.test.mjs`): klávesy na úchytu části a klíč sbalení.
// Pořadí a skrývání částí je v `model.ts` (`partsOf`, `movePart`, `setPartHidden`), gesto tahu v `partDrag.ts`,
// vykreslení v `PartSlot.tsx`.
import type { KeyIn } from "./drag.ts";

export type PartKeyAct = { move: 1 | -1 } | { hide: true } | null;
/** Klávesa na úchytu části: šipky = o místo výš / níž (← ↑ dřív, → ↓ později), Delete / Backspace = skrýt. */
export function partKeyAction(e: KeyIn): PartKeyAct {
  if (e.ctrlKey || e.metaKey) return null;
  switch (e.key) {
    case "ArrowLeft":
    case "ArrowUp":
      return { move: -1 };
    case "ArrowRight":
    case "ArrowDown":
      return { move: 1 };
    case "Delete":
    case "Backspace":
      return { hide: true };
    default:
      return null;
  }
}

/** Klíč sbalení části v paměti (karta + část). */
export const foldKey = (card: string, part: string): string => `${card}:${part}`;
/** Přepnutí sbalení: nový záznam, vstup zůstane netknutý; rozbalená část v záznamu není. */
export function toggleFold(folded: Readonly<Record<string, boolean>>, key: string): Record<string, boolean> {
  const out = { ...folded };
  if (out[key]) delete out[key];
  else out[key] = true;
  return out;
}
