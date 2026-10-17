// Rozbalovací panely výběrů (projekt, připomínka) uvnitř okna šuplíku — žádná nativní okna.
// Panel je v toku stránky (CSS `.pick-pop` zabere celý řádek lišty), takže šuplík jen naroste
// (ResizeObserver v App) a nic se neořízne. Otevřený je vždy nejvýš jeden; klik mimo ho zavře.

import { createSignal, onCleanup, createEffect } from "solid-js";

const [openId, setOpenId] = createSignal<string | null>(null);
let seq = 0;

export function usePopup() {
  const id = `pop${++seq}`;
  const open = () => openId() === id;
  const close = () => open() && setOpenId(null);
  createEffect(() => {
    if (!open()) return;
    const down = (e: PointerEvent) => {
      if (!(e.target as HTMLElement).closest?.(`[data-pop="${id}"]`)) close();
    };
    document.addEventListener("pointerdown", down, true);
    onCleanup(() => document.removeEventListener("pointerdown", down, true));
  });
  onCleanup(close);
  return { id, open, close, toggle: () => setOpenId(open() ? null : id) };
}

/** Panel výběru se zavřel — Esc / Enter v něm nesmí dojít k obsluze stránky (zahodit poznámku). */
export const swallow = (e: KeyboardEvent) => e.stopPropagation();

/** Posune jen `box` (ne stránku — `scrollIntoView` by posunul i obsah šuplíku). */
export function scrollWithin(box: Element | null | undefined, item: Element | null | undefined, center = false) {
  if (!(box instanceof HTMLElement) || !(item instanceof HTMLElement)) return;
  const top = item.offsetParent === box ? item.offsetTop : item.offsetTop - box.offsetTop;
  if (center) box.scrollTop = top - box.clientHeight / 2 + item.offsetHeight / 2;
  else if (top < box.scrollTop) box.scrollTop = top;
  else if (top + item.offsetHeight > box.scrollTop + box.clientHeight) box.scrollTop = top + item.offsetHeight - box.clientHeight;
}
