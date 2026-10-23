// Sdílená mechanika tahu v šuplíku (DOM): plynulý odsun prvků (FLIP) a přistání ducha na mezeru. Používá ji tah
// karet (`cardDrag.ts` + `Deck.tsx`), částí karet (`partDrag.ts`) i záložek (`tabDrag.ts`) — jeden motor, ne tři.
// Pravidla výkonu (software render WebView2): jen `transform` a `opacity` přes Web Animations, nic v klidu,
// „bez animací“ (`motionNow` styl none) i systémové omezení pohybu = prvky se přesunou hned.
// Čistá logika (plán, deltas, transformace) je v `dnd.ts` (testy).
import { dur, motionNow, spring } from "../motion";
import { flipDeltas, flipPlan, landTransforms, type LandBox } from "./dnd";

export const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Změna pořadí / velikosti s krátkým posunem prvků `els` (FLIP transformem; bez kývání). Rozběhlý posun se zruší
 *  a nový začne z místa, kde prvek právě je. */
export function flipEls(els: HTMLElement[], update: () => void) {
  const m = motionNow();
  const plan = flipPlan(m.style, m.speed, reducedMotion());
  if (!plan) return update();
  const at = (el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top };
  };
  const first = els.map(at);
  for (const el of els) el.getAnimations?.().forEach((a) => a.id === "lay-flip" && a.cancel());
  update();
  flipDeltas(first, els.map(at)).forEach((d, k) => {
    if (!d || !els[k].isConnected) return;
    const a = els[k].animate?.([{ transform: `translate(${d.dx}px, ${d.dy}px)` }, { transform: "none" }], { duration: plan.ms, easing: plan.easing });
    if (a) a.id = "lay-flip";
  });
}

/** Duch dosedne na mezeru `to` (viewport) pružinou a během přistání zmizí; bez cíle nebo bez animací jen krátce
 *  zhasne tam, kde je. Duch se po skončení odstraní. `from` = jeho levý horní roh v okamžiku puštění. */
export function landGhost(ghost: HTMLElement, from: { x: number; y: number }, to: LandBox | null) {
  const done = () => ghost.remove();
  const m = motionNow();
  const animated = typeof ghost.animate === "function" && !reducedMotion() && m.style !== "none";
  if (!animated) return done();
  if (!to) return fadeOut(ghost, dur(120), 0.78, done);
  const sp = spring(420, 34);
  const ms = Math.min(520, Math.max(160, sp.duration));
  const tr = landTransforms(from, { w: ghost.offsetWidth, h: ghost.offsetHeight }, to);
  ghost.style.transformOrigin = "0 0";
  ghost.animate([{ transform: tr.from }, { transform: tr.to }], { duration: ms, easing: sp.easing, fill: "forwards" });
  const f = ghost.animate([{ opacity: 0.78 }, { opacity: 0.78, offset: 0.6 }, { opacity: 0 }], { duration: ms, fill: "forwards" });
  f.onfinish = done;
}

function fadeOut(ghost: HTMLElement, ms: number, from: number, done: () => void) {
  const f = ghost.animate([{ opacity: from }, { opacity: 0 }], { duration: ms, fill: "forwards" });
  f.onfinish = done;
}
