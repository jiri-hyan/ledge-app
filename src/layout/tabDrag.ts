// Gesto přetažení záložky (řadič) — stejný motor jako u karet (`dnd.ts` čisté funkce, `flip.ts` FLIP a přistání
// ducha). V režimu úprav záložku chytíš tahem (od `GRAB_PX`), mimo úpravy podržením (`HOLD_MS`, rovnou zapne
// úpravy) — klik a dvojklik (přejmenovat) zůstanou. Tah: poloprůhledný duch záložky vodorovně pod kurzorem, na
// místě dopadu mezera, ostatní záložky se plynule odsunou (CSS `order` + FLIP), Esc / ztráta ukazatele = nic
// se nezmění, puštění → `env.commit(from, to)`. Pointer události na `window`, rAF se tu nepoužívá vůbec.
import { batch, createSignal } from "solid-js";
import { GRAB_PX, HOLD_MS, flyPos, holdStep, nearestCenter, offsetRect } from "./dnd";
import { landGhost } from "./flip";
import type { DragEnd } from "./drag";

/** Rozdělaný tah záložky: odkud kam (indexy v `layout.pages`). */
export type TabDragState = { id: string; from: number; to: number };

export type TabDragEnv = {
  /** kontejner záložek (`.lay-tabs`) */
  box: () => HTMLElement;
  editing: () => boolean;
  /** zapnout režim úprav (podržení mimo úpravy); s chybou configu se nezapnou */
  begin: () => void;
  /** nemá se začínat (právě se přejmenovává) */
  busy: () => boolean;
  /** změna s plynulým odsunem záložek (FLIP) */
  flip: (update: () => void) => void;
  commit: (from: number, to: number) => void;
};

export function createTabDrag(env: TabDragEnv) {
  const [drag, setDrag] = createSignal<TabDragState | null>(null, { equals: (a, b) => a === b || (!!a && !!b && a.id === b.id && a.from === b.from && a.to === b.to) });
  let abort: (() => void) | null = null;

  const tabs = () => Array.from(env.box().querySelectorAll<HTMLElement>(":scope > .lay-tab[data-page]"));
  const rectOf = (el: HTMLElement) => {
    const b = env.box();
    const r = b.getBoundingClientRect();
    return offsetRect({ left: r.left, top: r.top, clientLeft: b.clientLeft, clientTop: b.clientTop, scrollLeft: b.scrollLeft, scrollTop: b.scrollTop }, el);
  };

  /** pointerdown na tlačítku záložky `id` (index `i`) */
  function down(e: PointerEvent, id: string, i: number) {
    if (e.button !== 0 || abort || env.busy()) return;
    const hold = !env.editing();
    const x0 = e.clientX;
    const y0 = e.clientY;
    const t0 = performance.now();
    const r0 = (e.currentTarget as HTMLElement).closest<HTMLElement>(".lay-tab")!.getBoundingClientRect();
    const frac = r0.width > 0 ? (x0 - r0.left) / r0.width : 0.5;
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
      }
      session(id, i, last, frac);
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

  function session(id: string, from: number, pt: { x: number; y: number }, frac: number) {
    const box = env.box();
    const el = tabs()[from];
    if (!el) return;
    // úpravy mohly záložky rozšířit (× u názvu) — střed a duch se berou až teď
    const r0 = el.getBoundingClientRect();
    const centers = tabs().map((x) => {
      const r = x.getBoundingClientRect();
      return r.left + r.width / 2;
    });
    const ghost = el.cloneNode(true) as HTMLElement;
    ghost.querySelectorAll(".lay-tab-x").forEach((n) => n.remove());
    ghost.classList.add("lp-tabfly");
    ghost.removeAttribute("data-page");
    ghost.setAttribute("aria-hidden", "true");
    ghost.inert = true;
    Object.assign(ghost.style, { width: `${r0.width}px`, height: `${r0.height}px`, order: "", transform: `translate(${r0.left}px, ${r0.top}px)` });
    box.appendChild(ghost);
    const dx = frac * r0.width;
    let last = pt;
    const place = () => {
      // vodorovně za kurzorem, svisle zůstává na řadě záložek
      const p = flyPos({ dx, dy: 0 }, last.x, r0.top);
      ghost.style.transform = `translate(${p.x}px, ${p.y}px)`;
    };
    const retarget = () => {
      const d = drag();
      if (!d) return;
      const to = nearestCenter(centers, last.x);
      if (to !== d.to) env.flip(() => setDrag({ ...d, to }));
    };
    const move = (ev: PointerEvent) => {
      // tlačítko bylo puštěno mimo okno (pointerup nedorazil): tah se zruší
      if (ev.pointerType === "mouse" && ev.buttons === 0) return finish("cancel");
      last = { x: ev.clientX, y: ev.clientY };
      place();
      retarget();
    };
    const up = () => finish("up");
    const cancel = () => finish("cancel");
    const key = (ev: KeyboardEvent) => {
      if (ev.key !== "Escape") return;
      ev.preventDefault();
      ev.stopPropagation();
      finish("escape");
    };
    const off = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", key, true);
      abort = null;
    };
    function finish(how: DragEnd) {
      off();
      const d = drag();
      const apply = () =>
        batch(() => {
          if (d && how === "up" && d.to !== d.from) env.commit(d.from, d.to);
          setDrag(null);
        });
      // zrušeno: záložky se plynule vrátí; puštění: náhled už je výsledek
      if (how !== "up" || !d || d.to === d.from) env.flip(apply);
      else apply();
      const home = box.querySelector<HTMLElement>(`:scope > .lay-tab[data-page="${CSS.escape(id)}"]`);
      landGhost(ghost, flyPos({ dx, dy: 0 }, last.x, r0.top), home ? rectOf(home) : null);
      // po tahu nesmí z puštění vzniknout klik na záložku pod kurzorem (přepnutí záložky)
      const eat = (c: Event) => (c.stopPropagation(), c.preventDefault());
      window.addEventListener("click", eat, true);
      window.setTimeout(() => window.removeEventListener("click", eat, true), 80);
    }
    setDrag({ id, from, to: from });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", key, true);
    abort = cancel;
    place();
  }

  return { drag, down, abort: () => abort?.() };
}
