// Pohybový engine šuplíku: pružiny (fyzikální simulace → CSS linear()), morf tvaru přes clip-path,
// postupné naskakování obsahu a měření plynulosti (requestAnimationFrame). Styl a rychlost animací
// z `appearance.motion` (`setMotion`, presety v `theme/appearance.ts`) mění jen PARAMETRY pružin a
// délky — mechanika morfu (handshake, clip-path) je stejná. `none` = délka 0 (okamžitě, ne rozbité).

import { api } from "./api";
import type { MotionStyle } from "./types";
import { simulateSpring, type Spring } from "./theme/springs";

/** Přesah okna nad horní hranu obrazovky (logické px) — musí sedět s drawer.rs a --overlap. */
export const OVERLAP = 18;

export type { Spring };

const cache = new Map<string, Spring>();

let motionStyle: MotionStyle = "spring";
let motionSpeed = 1;

/** Styl a rychlost animací (apply.ts; systémové „omezit pohyb“ už převedené na `none`). */
export function setMotion(style: MotionStyle, speed: number) {
  if (style === motionStyle && speed === motionSpeed) return;
  motionStyle = style;
  motionSpeed = speed > 0 ? speed : 1;
  cache.clear();
}

/** Aktuální (už efektivní) styl a rychlost — pro přechody mimo pružiny (`settings/widen.ts`). */
export const motionNow = (): { style: MotionStyle; speed: number } => ({ style: motionStyle, speed: motionSpeed });

/** Délka (ms) podle rychlosti; styl `none` = 0. Pro pevné délky mimo pružiny (zavírání, zmizení). */
export const dur = (ms: number) => (motionStyle === "none" ? 0 : Math.round(ms / motionSpeed));

/**
 * Tlumená pružina → CSS `linear()` easing podle aktuálního stylu a rychlosti (simulace je čistá
 * funkce `theme/springs.ts` `simulateSpring`; tady jen cache na klíč parametrů).
 */
export function spring(stiffness = 300, damping = 24, mass = 1): Spring {
  const key = `${stiffness}/${damping}/${mass}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const s = simulateSpring(motionStyle, motionSpeed, stiffness, damping, mass);
  cache.set(key, s);
  return s;
}

let stripRadius = 14;
/** Rádius proužku (logické px) podle `appearance.strip` — výchozí tvar morfu. */
export const setStripRadius = (r: number) => (stripRadius = r);

/** Tvar panelu složeného do obdélníku w×h (logické px) uprostřed horní hrany. Nezávislé na velikosti okna. */
export function collapsedClip(w: number, h: number, r = stripRadius): string {
  const half = Math.max(0, w / 2);
  return `inset(0px calc(50% - ${half}px) calc(100% - ${OVERLAP + h}px) calc(50% - ${half}px) round 0px 0px ${r}px ${r}px)`;
}

/** Proužek roztažený do celé šířky (mezikrok animace „Z výřezu“): výška `h` + kousek, rohy jako šuplík. */
export function bandClip(h: number, r = shellR): string {
  return `inset(0px 0px calc(100% - ${OVERLAP + Math.round(h * 1.6)}px) 0px round 0px 0px ${r}px ${r}px)`;
}

const fullClip = (r: number) => `inset(0px 0px 0px 0px round 0px 0px ${r}px ${r}px)`;
/** Plný tvar šuplíku = `--r-shell` (živá vazba ES modulu: `setShellRadius` ji přepíše i v importu). */
export let FULL_CLIP = fullClip(26);
let shellR = 26;
/** Rádius šuplíku (logické px) — stejný jako ve `FULL_CLIP`. */
export const shellRadius = () => shellR;
export const setShellRadius = (r: number) => {
  shellR = Math.round(r * 100) / 100;
  FULL_CLIP = fullClip(shellR);
};

/** Měří snímky po dobu `ms` a pošle statistiku do Rustu (/status → drawer.anim). */
export function measure(kind: string, ms: number) {
  const deltas: number[] = [];
  let last = performance.now();
  const start = last;
  const tick = (now: number) => {
    deltas.push(now - last);
    last = now;
    if (now - start < ms) requestAnimationFrame(tick);
    else {
      const body = deltas.slice(1);
      if (!body.length) return;
      const avg = body.reduce((a, b) => a + b, 0) / body.length;
      api.animStats({ kind, frames: body.length, avgMs: +avg.toFixed(2), worstMs: +Math.max(...body).toFixed(2), durationMs: Math.round(now - start) });
    }
  };
  requestAnimationFrame(tick);
}

/** Výchozí stav obsahu před naskočením (bez animace, okamžitě). */
export function preHide(root: HTMLElement) {
  root.querySelectorAll<HTMLElement>("[data-anim]").forEach((el) => {
    el.getAnimations().forEach((a) => a.cancel());
    el.style.opacity = "0";
  });
}

/** Postupné naskočení obsahu: hlavička, dlaždice (pop), karty. */
export function revealContent(root: HTMLElement, startDelay = 70) {
  const pop = spring(420, 22);
  const soft = spring(260, 26);
  let tile = 0;
  let block = 0;
  root.querySelectorAll<HTMLElement>("[data-anim]").forEach((el) => {
    const kind = el.dataset.anim;
    el.style.opacity = "";
    if (kind === "tile") {
      el.animate(
        [
          { opacity: 0, transform: "translateY(-14px) scale(0.82)" },
          { opacity: 1, transform: "translateY(0) scale(1)" },
        ],
        { duration: pop.duration, easing: pop.easing, delay: dur(startDelay + 40 + tile++ * 22), fill: "backwards" },
      );
    } else {
      el.animate(
        [
          { opacity: 0, transform: "translateY(-10px)", filter: "blur(4px)" },
          { opacity: 1, transform: "translateY(0)", filter: "blur(0)" },
        ],
        { duration: soft.duration, easing: soft.easing, delay: dur(startDelay + block++ * 45), fill: "backwards" },
      );
    }
  });
}

/** Rychlé zmizení obsahu před stažením do proužku. */
export function concealContent(root: HTMLElement, ms = 120): Promise<void> {
  const els = Array.from(root.querySelectorAll<HTMLElement>("[data-anim]"));
  const anims = els.map((el) =>
    el.animate([{ opacity: 1, transform: "none" }, { opacity: 0, transform: "translateY(-6px) scale(0.98)" }], {
      duration: dur(ms),
      easing: "cubic-bezier(.4,0,1,1)",
      fill: "forwards",
    }),
  );
  return Promise.all(anims.map((a) => a.finished)).then(() => undefined, () => undefined);
}
