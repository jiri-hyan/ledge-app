// Gesto přetažení karty (řadič) — přirozený drag & drop v mřížce šuplíku. Čistá logika je v `dnd.ts` / `drag.ts`
// / `model.ts` (testy), tady jen DOM: pointer události na `window` (HTML5 drag&drop v okně Tauri nefunguje),
// měření mřížky, „duch“ karty pod kurzorem, rAF smyčka JEN během tahu (posun mřížky u okraje, prodleva nad
// ouškem záložky), úklid. Vstupy:
//  - v úpravách chytneš kartu kdekoli na jejím prázdném místě nebo za ⠿ (od `GRAB_PX`); ovládací prvky ne,
//  - mimo úpravy podržení (`HOLD_MS`) na hlavičce karty zapne úpravy a rovnou začne tah (klik a výběr textu
//    zůstanou: posun nad `HOLD_SLOP` před uplynutím času podržení zruší),
//  - tah: poloprůhledný duch pod kurzorem, na místě dopadu mezera velikosti karty (CSS `order`, DOM se
//    nepřeskládá), ostatní se plynule odsunou (FLIP z `Deck`), zóna „Skrýt kartu“ dole, ouško záložky = přesun
//    na její konec, prodleva `DWELL_MS` nad ouškem záložku přepne a tah pokračuje v její mřížce,
//  - Esc / ztráta ukazatele = nic se nezmění; puštění → `env.commit`.
import { batch, createSignal } from "solid-js";
import { dur } from "../motion";
import { dropTab, noteHoldDrag, setDropTab, setEditPage } from "./edit";
import { dropAction, autoScrollStep, type DragEnd, type DropAction } from "./drag";
import { CONTROL_SELECTOR, DWELL_MS, EDIT_CONTROL_SELECTOR, GRAB_PX, HEADER_SELECTOR, HOLD_MS, NO_DWELL, canHold, contentPoint, dwellStep, flyPos, grabOffset, holdStep, inBox, insertIndex, type ScrollBox } from "./dnd";
import { gridDropIndex, type CardHeight, type CardId, type CardSize, type LayoutCard, type LayoutPage, type Rect } from "./model";

/** Stav rozdělaného tahu (mění se jen při změně cíle — poloha ukazatele je zvlášť). */
export type Drag = { id: CardId; home: string; from: number; to: number; size: CardSize; h: CardHeight | undefined; px: number };

export type DragEnv = {
  box: () => HTMLElement;
  zone: () => HTMLElement | undefined;
  editing: () => boolean;
  page: () => LayoutPage;
  /** karty zobrazené záložky v pořadí mřížky (v úpravách včetně těch, které teď nemají co ukázat) */
  ids: () => CardId[];
  cardOf: (id: CardId) => LayoutCard | undefined;
  /** zapnout režim úprav (podržení mimo úpravy); s chybou configu se nezapnou */
  begin: () => void;
  /** karta už na záložce `page` je (na jedné záložce smí být jen jednou) */
  blocked: (d: Drag, page: string) => boolean;
  /** změna s plynulým odsunem karet (FLIP) */
  flip: (update: () => void) => void;
  commit: (a: DropAction, d: Drag) => void;
};

const sameDrag = (a: Drag | null, b: Drag | null) => a === b || (!!a && !!b && a.id === b.id && a.home === b.home && a.from === b.from && a.to === b.to);

export function createCardDrag(env: DragEnv) {
  const [drag, setDrag] = createSignal<Drag | null>(null, { equals: sameDrag });
  const [ptr, setPtr] = createSignal({ x: 0, y: 0 });
  const [zoneOn, setZoneOn] = createSignal(false);
  /** zruší rozdělané gesto (čekání na tah i tah) — pro odmontování mřížky */
  let abort: (() => void) | null = null;

  const items = () => Array.from(env.box().querySelectorAll<HTMLElement>(":scope > .deck-item:not(.lay-fly)"));
  const scrollBox = (): ScrollBox => {
    const b = env.box();
    const r = b.getBoundingClientRect();
    return { left: r.left, top: r.top, clientLeft: b.clientLeft, clientTop: b.clientTop, scrollLeft: b.scrollLeft, scrollTop: b.scrollTop };
  };
  /** rámečky karet v souřadnicích obsahu mřížky (`offset*`: bez posunu, bez rozběhlých animací) */
  const measure = (skip?: CardId): Rect[] =>
    items()
      .filter((el) => el.dataset.card !== skip)
      .map((el) => ({ x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight }));

  /** pointerdown na `.deck-item`: v úpravách chytit prázdné místo karty / ⠿, jinak podržení hlavičky */
  function down(e: PointerEvent, id: CardId) {
    if (e.button !== 0 || abort) return;
    const t = e.target as Element;
    const item = e.currentTarget as HTMLElement;
    if (env.editing()) {
      // obsah karty v úpravách nemá pointer-events (až na vlastní úpravy dlaždic / odkazů) → cíl je sama položka
      if (t.closest(EDIT_CONTROL_SELECTOR) || !(t === item || t.closest(".lay-grip"))) return;
      pend(e, id, false);
      return;
    }
    const head = t.closest(HEADER_SELECTOR);
    const ctl = t.closest(CONTROL_SELECTOR);
    const inHead = !!head && item.contains(head);
    if (!canHold({ button: e.button, editing: false, header: inHead, control: !!ctl && !!head && head.contains(ctl) })) return;
    pend(e, id, true);
  }

  /** čekání na tah: v úpravách na posun o `GRAB_PX`, mimo ně na uplynutí `HOLD_MS` bez pohybu */
  function pend(e: PointerEvent, id: CardId, hold: boolean) {
    const x0 = e.clientX;
    const y0 = e.clientY;
    const t0 = performance.now();
    // kde na kartě ji uživatel chytil — ještě před zapnutím úprav (lišta nad mřížkou kartu posune dolů)
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const grab0 = { dx: x0 - r.left, dy: y0 - r.top };
    let last = { x: x0, y: y0 };
    let timer = 0;
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      window.removeEventListener("blur", stop);
      clearTimeout(timer);
      abort = null;
    };
    const go = () => {
      stop();
      if (hold) {
        env.begin();
        if (!env.editing()) return;
        window.getSelection()?.removeAllRanges();
        noteHoldDrag();
      }
      session(id, last, hold, hold ? grab0 : undefined);
    };
    const move = (ev: PointerEvent) => {
      last = { x: ev.clientX, y: ev.clientY };
      const dist = Math.hypot(last.x - x0, last.y - y0);
      if (!hold) return void (dist >= GRAB_PX && go());
      const s = holdStep(dist, performance.now() - t0);
      if (s === "cancel") stop();
      else if (s === "start") go();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
    window.addEventListener("blur", stop);
    if (hold) timer = window.setTimeout(go, HOLD_MS);
    abort = stop;
  }

  /** „duch“: kopie karty (i s jejím vzhledem — leží přímo v `.deck`), pevně pod kurzorem, bez ovládání */
  function makeGhost(el: HTMLElement, r: DOMRect): HTMLElement {
    const g = el.cloneNode(true) as HTMLElement;
    g.querySelectorAll(".lay-tools, .lay-rs, .lay-target").forEach((n) => n.remove());
    g.classList.remove("lifted");
    g.classList.add("lay-fly");
    g.removeAttribute("data-card");
    g.setAttribute("aria-hidden", "true");
    g.inert = true;
    Object.assign(g.style, { width: `${r.width}px`, height: `${r.height}px`, order: "", transform: `translate(${r.left}px, ${r.top}px)` });
    env.box().appendChild(g);
    return g;
  }

  function session(id: CardId, pt: { x: number; y: number }, fromHold: boolean, grab0?: { dx: number; dy: number }) {
    const box = env.box();
    const home = env.page().id;
    const from = env.ids().indexOf(id);
    const el = items()[from];
    if (from < 0 || !el) return;
    const r0 = el.getBoundingClientRect();
    const grab = grab0 ? grabOffset({ left: 0, top: 0, width: r0.width, height: r0.height }, grab0.dx, grab0.dy) : grabOffset(r0, pt.x, pt.y);
    const ghost = makeGhost(el, r0);
    const card = env.cardOf(id);
    let viewId = home;
    let last = pt;
    let rects = measure();
    let tab: string | null = null;
    let dwell = NO_DWELL;
    let raf = 0;

    const place = () => {
      const p = flyPos(grab, last.x, last.y);
      ghost.style.transform = `translate(${p.x}px, ${p.y}px)`;
      setPtr(last);
    };
    const tabAt = (x: number, y: number) => {
      const t = (document.elementFromPoint(x, y) as HTMLElement | null)?.closest<HTMLElement>("[data-tab]")?.dataset.tab ?? null;
      return t && t !== viewId ? t : null;
    };
    const retarget = () => {
      const d = drag();
      if (!d) return;
      const z = env.zone()?.getBoundingClientRect();
      const inZone = !!z && inBox(z, last.x, last.y, 4);
      const tb = inZone ? null : tabAt(last.x, last.y);
      tab = tb;
      let to = d.to;
      if (!inZone && !tb) {
        const c = contentPoint(scrollBox(), last.x, last.y);
        if (viewId !== d.home) to = env.blocked(d, viewId) ? d.to : insertIndex(rects, d.to, c.x, c.y);
        else to = gridDropIndex(rects, d.from, c.x, c.y);
      }
      if (inZone !== zoneOn() || tb !== dropTab() || to !== d.to) {
        env.flip(() =>
          batch(() => {
            setZoneOn(inZone);
            setDropTab(tb);
            if (to !== d.to) setDrag({ ...d, to });
          }),
        );
      }
    };
    /** prodleva nad ouškem uběhla: záložka se přepne, tah pokračuje v její mřížce (karta se vloží na konec) */
    const switchTo = (tabId: string) => {
      const d = drag();
      if (!d) return;
      setEditPage(tabId);
      viewId = tabId;
      setDropTab(null);
      setZoneOn(false);
      setDrag({ ...d, to: tabId === d.home ? d.from : env.ids().length });
      rects = measure(tabId === d.home || env.blocked(d, tabId) ? undefined : d.id);
      tab = null;
      retarget();
    };
    const tick = (now: number) => {
      const r = box.getBoundingClientRect();
      const zr = env.zone()?.getBoundingClientRect();
      const step = autoScrollStep(last.y, r.top, zr ? Math.min(r.bottom, zr.top) : r.bottom);
      if (step) {
        box.scrollTop += step;
        retarget();
      }
      const w = dwellStep(dwell, tab, now, DWELL_MS);
      dwell = w.dwell;
      if (w.fire) switchTo(w.fire);
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
      window.removeEventListener("keydown", key, true);
      cancelAnimationFrame(raf);
      abort = null;
    };
    const finish = (how: DragEnd) => {
      off();
      const d = drag();
      const tb = dropTab();
      const foreign = d && viewId !== d.home ? { page: viewId, blocked: env.blocked(d, viewId) } : undefined;
      // pustit na ouško domovské záložky (tah byl přepnutý jinam) = vrátit kartu, kam patřila
      const a: DropAction = d && tb && tb === d.home ? { kind: "none" } : dropAction(d, tb, how, { zone: zoneOn(), foreign });
      const stay = a.kind === "insert" || a.kind === "page";
      const apply = () =>
        batch(() => {
          if (d && !stay && viewId !== d.home) setEditPage(d.home);
          if (d) env.commit(a, d);
          setDrag(null);
          setDropTab(null);
          setZoneOn(false);
        });
      // zrušeno / zahozeno: karty se plynule vrátí; puštění na místo: náhled už je výsledek
      if (a.kind === "none" || a.kind === "hide" || a.kind === "blocked") env.flip(apply);
      else apply();
      const ms = dur(120);
      if (ms > 0 && ghost.animate) {
        const f = ghost.animate([{ opacity: 0.78 }, { opacity: 0 }], { duration: ms, fill: "forwards" });
        f.onfinish = () => ghost.remove();
      } else ghost.remove();
      if (fromHold) {
        // po tahu z podržení nesmí z puštění vzniknout klik na hlavičku pod kurzorem
        const eat = (c: Event) => (c.stopPropagation(), c.preventDefault());
        window.addEventListener("click", eat, true);
        window.setTimeout(() => window.removeEventListener("click", eat, true), 80);
      }
    };
    const up = () => finish("up");
    const cancel = () => finish("cancel");
    // Esc zruší jen tah, ne celé úpravy (dřív Esc během tahu zavřel režim úprav)
    const key = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      ev.preventDefault();
      ev.stopPropagation();
      finish("escape");
    };

    setPtr(pt);
    setDrag({ id, home, from, to: from, size: card?.size ?? "m", h: card?.h, px: r0.height });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
    abort = cancel;
    place();
    raf = requestAnimationFrame(tick);
  }

  return { drag, ptr, zoneOn, down, abort: () => abort?.() };
}
