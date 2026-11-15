// Rozbalení náhledu po najetí do celého šuplíku: klik na prázdné místo hned, setrvání myši po `expandMs`
// (`drawer.hoverExpandMs`). Odpočet běží jen s otevřeným náhledem (komponenta žije jen tehdy) a stojí,
// dokud je myš nad tlačítkem, mimo okno nebo čeká schválení (`expandAfter`). Fokus nebere nic z toho. Kam se rozbalí, určuje jádro
// (dok → střední panel, jinak celý šuplík); `toFull` = rovnou celý (schválení v docku přebírá ostrůvek).
import { createEffect, createSignal, onCleanup, type Accessor } from "solid-js";
import { api } from "../api";
import { EXPAND_IGNORE, expandAfter } from "./model";

const onControl = (e: Event) => !!(e.target as HTMLElement | null)?.closest?.(EXPAND_IGNORE);

/** Posluchače na kořen náhledu (`<div {...useExpand(…)}>`); `pinned` = upevněný dok se nikdy nerozbalí. */
export function useExpand(ms: Accessor<number>, waiting: Accessor<boolean>, pinned: Accessor<boolean> = () => false, toFull: Accessor<boolean> = () => false) {
  // myš nad tlačítkem / mimo okno → odpočet stojí; klik na tlačítko ho začne znovu
  const [hold, setHold] = createSignal(false);
  const [round, setRound] = createSignal(0);
  createEffect(() => {
    round();
    const after = expandAfter(ms(), waiting(), hold(), pinned());
    if (after == null) return;
    const h = window.setTimeout(() => void api.expandDrawer(toFull()), after);
    onCleanup(() => clearTimeout(h));
  });
  return {
    onPointerOver: (e: PointerEvent) => setHold(onControl(e)),
    onPointerLeave: () => setHold(true),
    onClick: (e: MouseEvent) => {
      if (onControl(e)) setRound((r) => r + 1);
      else void api.expandDrawer(toFull());
    },
  };
}
