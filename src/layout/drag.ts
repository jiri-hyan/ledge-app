// Tažení karet v režimu úprav bez DOM (test `drag.test.mjs`): co udělá puštění / zrušení, náhled pořadí
// během tahu (tažená karta je hned na místě dopadu, ostatní se posunou), posun mřížky u okraje a klávesy
// na úchytu ⠿. `Deck.tsx` jen měří a volá tyhle funkce — HTML5 drag&drop v okně Tauri nefunguje, tah je
// na pointer událostech.
import { moveItem } from "./model.ts";

export type DragState = { from: number; to: number };
/** Jak tah skončil: `up` = puštěno, `cancel` = systém vzal ukazatel (pointercancel), `escape` = Esc. */
export type DragEnd = "up" | "cancel" | "escape";
export type DropAction = { kind: "none" } | { kind: "move"; from: number; to: number } | { kind: "page"; tab: string };

/** Co udělat po konci tahu. Jen puštění něco mění; zrušení nikdy (karta zůstane, kde byla). */
export function dropAction(d: DragState | null, tab: string | null, how: DragEnd): DropAction {
  if (!d || how !== "up") return { kind: "none" };
  if (tab) return { kind: "page", tab };
  return d.to !== d.from ? { kind: "move", from: d.from, to: d.to } : { kind: "none" };
}

/** Pořadí karet během tahu (náhled): `from` přesunutá na `to`; mimo rozsah = beze změny. Stejná
 *  funkce jako puštění (`moveCard`) — náhled a výsledek se nerozejdou. */
export const previewOrder = moveItem;

/** Posun mřížky za snímek, když je ukazatel u horního / spodního okraje (px; záporný = nahoru). */
export function autoScrollStep(y: number, top: number, bottom: number, edge = 40, max = 16): number {
  if (bottom - top < edge * 2) return 0;
  if (y < top + edge) return -Math.round((Math.min(edge, top + edge - y) / edge) * max);
  if (y > bottom - edge) return Math.round((Math.min(edge, y - (bottom - edge)) / edge) * max);
  return 0;
}

export type KeyIn = { key: string; altKey?: boolean; ctrlKey?: boolean; metaKey?: boolean };
export type KeyAct = { move: 1 | -1 } | { size: 1 | -1 } | { height: 1 | -1 } | null;

/** Klávesa na úchytu karty: šipky (i s Alt) = přesun, + / − = šířka, PageUp / PageDown = výška. */
export function keyAction(e: KeyIn): KeyAct {
  if (e.ctrlKey || e.metaKey) return null;
  switch (e.key) {
    case "ArrowLeft":
    case "ArrowUp":
      return { move: -1 };
    case "ArrowRight":
    case "ArrowDown":
      return { move: 1 };
    case "+":
    case "=":
      return { size: 1 };
    case "-":
      return { size: -1 };
    case "PageDown":
      return { height: 1 };
    case "PageUp":
      return { height: -1 };
    default:
      return null;
  }
}
