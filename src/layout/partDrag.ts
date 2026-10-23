// Gesto přetažení části karty (řadič) — stejný motor jako u karet a záložek (`dnd.ts` čisté funkce, `flip.ts`
// FLIP a přistání ducha, `drag.ts` `dropAction`). V režimu úprav chytíš část za její lištu `.lp-edit` (tah od
// `GRAB_PX`) a přetáhneš ji v rámci karty: duch (kopie části pod kurzorem), na místě dopadu mezera, ostatní části
// se plynule odsunou (CSS `order` + FLIP, DOM se nepřeskládá). Dole zóna „Skrýt“ (sdílená s kartami): puštění
// na ni část skryje (`hiddenParts`, jde vrátit). Hlavní obsah karty (`body`) se skrýt nedá. Esc / ztráta
// ukazatele = nic se nezmění. rAF jen během tahu (posun mřížky u okraje).
import { batch, createSignal } from "solid-js";
import { GRAB_PX, contentPoint, flyPos, grabOffset, inBox, offsetRect } from "./dnd";
import { autoScrollStep, dropAction, previewOrder, type DragEnd, type DropAction } from "./drag";
import { flipEls, landGhost } from "./flip";
import { BODY, gridDropIndex, type CardId, type Rect } from "./model";

/** Rozdělaný tah části: která karta, která část, odkud kam v úplném pořadí částí karty. */
export type PartDrag = { card: CardId; key: string; from: number; to: number; order: string[] };

export type PartDragEnv = {
  /** posuvná mřížka (`.deck`) — posun u okraje */
  box: () => HTMLElement;
  zone: () => HTMLElement | undefined;
  /** úplné uložené pořadí částí karty (i skrytých) */
  orderOf: (card: CardId) => string[];
  commit: (a: DropAction, d: PartDrag) => void;
};

const same = (a: PartDrag | null, b: PartDrag | null) => a === b || (!!a && !!b && a.card === b.card && a.key === b.key && a.from === b.from && a.to === b.to);

export function createPartDrag(env: PartDragEnv) {
  const [drag, setDrag] = createSignal<PartDrag | null>(null, { equals: same });
  const [ptr, setPtr] = createSignal({ x: 0, y: 0 });
  const [zoneOn, setZoneOn] = createSignal(false);
  let abort: (() => void) | null = null;

  /** pořadí částí karty během tahu (náhled); mimo tah nebo nad zónou Skrýt `null` = uložené */
  const preview = (card: CardId): string[] | null => {
    const d = drag();
    return d && d.card === card && !zoneOn() ? previewOrder(d.order, d.from, d.to) : null;
  };
  /** tahá se část, kterou jde skrýt → Deck ukáže zónu Skrýt */
  const hideable = () => {
    const d = drag();
    return !!d && d.key !== BODY;
  };

  /** pointerdown na liště části (jen v úpravách; Deck ji napojí přes `PartsCtl.down`) */
  function down(e: PointerEvent, card: CardId, key: string) {
    if (e.button !== 0 || abort) return;
    const bar = e.currentTarget as HTMLElement;
    const slot = bar.closest<HTMLElement>(".lp-slot");
    if (!slot) return;
    const x0 = e.clientX;
    const y0 = e.clientY;
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      window.removeEventListener("blur", stop);
      abort = null;
    };
    const move = (ev: PointerEvent) => {
      const at = { x: ev.clientX, y: ev.clientY };
      if (Math.hypot(at.x - x0, at.y - y0) < GRAB_PX) return;
      stop();
      window.getSelection()?.removeAllRanges();
      session(card, key, slot, at);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
    window.addEventListener("blur", stop);
    abort = stop;
  }

  function session(card: CardId, key: string, slot: HTMLElement, pt: { x: number; y: number }) {
    const lpbox = slot.closest<HTMLElement>(".lp-box");
    const order = env.orderOf(card);
    const from = order.indexOf(key);
    if (!lpbox || from < 0) return;
    const deck = env.box();
    const slots = () => Array.from(lpbox.querySelectorAll<HTMLElement>(":scope > .lp-slot"));
    const slotOf = (k: string) => slots().find((el) => el.dataset.part === k);
    const hostBox = () => {
      const hr = lpbox.getBoundingClientRect();
      return { left: hr.left, top: hr.top, clientLeft: lpbox.clientLeft, clientTop: lpbox.clientTop, scrollLeft: 0, scrollTop: 0 };
    };
    const r0 = slot.getBoundingClientRect();
    const grab = grabOffset(r0, pt.x, pt.y);
    // rámečky částí v pořadí karty, v souřadnicích lišty `.lp-box` (`offset*`: bez posunu, bez rozběhlých animací)
    const rects: Rect[] = order.map((k) => {
      const el = slotOf(k);
      return el ? { x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight } : { x: 0, y: 0, w: 0, h: 0 };
    });
    // duch: kopie části v obalu, který nese styly karty (potomci `.card …`), pevně pod kurzorem
    const ghost = document.createElement("div");
    ghost.className = "card lp-fly";
    ghost.setAttribute("aria-hidden", "true");
    ghost.inert = true;
    ghost.appendChild(slot.cloneNode(true));
    Object.assign(ghost.style, { width: `${r0.width}px`, minHeight: `${r0.height}px`, transform: `translate(${r0.left}px, ${r0.top}px)` });
    document.body.appendChild(ghost);
    let last = pt;
    let raf = 0;

    const place = () => {
      const p = flyPos(grab, last.x, last.y);
      ghost.style.transform = `translate(${p.x}px, ${p.y}px)`;
      setPtr(last);
    };
    const retarget = () => {
      const d = drag();
      if (!d) return;
      const z = key === BODY ? undefined : env.zone()?.getBoundingClientRect();
      const inZone = !!z && inBox(z, last.x, last.y, 4);
      let to = d.to;
      if (!inZone) {
        const c = contentPoint(hostBox(), last.x, last.y);
        to = gridDropIndex(rects, d.from, c.x, c.y);
      }
      if (inZone !== zoneOn() || to !== d.to) {
        flipEls(slots(), () =>
          batch(() => {
            setZoneOn(inZone);
            if (to !== d.to) setDrag({ ...d, to });
          }),
        );
      }
    };
    const tick = () => {
      const r = deck.getBoundingClientRect();
      const zr = env.zone()?.getBoundingClientRect();
      const step = autoScrollStep(last.y, r.top, zr ? Math.min(r.bottom, zr.top) : r.bottom);
      if (step) deck.scrollTop += step;
      raf = requestAnimationFrame(tick);
    };
    const move = (ev: PointerEvent) => {
      // tlačítko bylo puštěno mimo okno (pointerup nedorazil): tah se zruší, ať nezůstane viset
      if (ev.pointerType === "mouse" && ev.buttons === 0) return cancel();
      last = { x: ev.clientX, y: ev.clientY };
      place();
      retarget();
    };
    const off = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", onKey, true);
      cancelAnimationFrame(raf);
      abort = null;
    };
    const finish = (how: DragEnd) => {
      off();
      const d = drag();
      const a = dropAction(d, null, how, { zone: zoneOn() });
      const apply = () =>
        batch(() => {
          if (d) env.commit(a, d);
          setDrag(null);
          setZoneOn(false);
        });
      // zrušeno / skryto: části se plynule vrátí; puštění na místo: náhled už je výsledek
      if (a.kind === "none" || a.kind === "hide") flipEls(slots(), apply);
      else apply();
      const el = a.kind === "hide" ? null : slotOf(key);
      landGhost(ghost, flyPos(grab, last.x, last.y), el ? offsetRect(hostBox(), el) : null);
    };
    const up = () => finish("up");
    const cancel = () => finish("cancel");
    // Esc zruší jen tah, ne celé úpravy
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      ev.preventDefault();
      ev.stopPropagation();
      finish("escape");
    };

    setPtr(pt);
    setDrag({ card, key, from, to: from, order });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", onKey, true);
    abort = cancel;
    place();
    raf = requestAnimationFrame(tick);
  }

  return { drag, ptr, zoneOn, preview, hideable, down, abort: () => abort?.() };
}
