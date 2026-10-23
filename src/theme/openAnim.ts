// Plán animace otevření / zavření šuplíku podle stylu pohybu (čistá funkce bez DOM, testuje
// `theme.test.mjs`). Mechanika morfu se nemění: Rust ukáže okno po `prepared`, frontend přehraje
// plán na `.shell` a po zavření pošle `closed` (`App.tsx`). Styl mění jen to, CO se animuje:
//  - pružina / plynulá / svižná: tvar (clip-path) z proužku do šuplíku pružinou stylu (dnešní morf);
//  - z výřezu: nejdřív roztažení do šířky ve výšce proužku, pak rozbalení dolů (Dynamic Island);
//  - odskok: celý šuplík spadne shora a dvakrát odskočí;
//  - rozplynutí: jen průhlednost a jemné zvětšení;
//  - pád: celý šuplík spadne shora, jednou se jemně odrazí a usadí (jen posun a průhlednost);
//  - rozvinutí: tvar se rozvine shora dolů bez překmitu a karty naskakují jedna po druhé (`Plan.reveal`);
//  - žádná (i systémové „omezit pohyb“): `null` = okamžitě.
// Stejný plán přehrává náhled v Nastavení (`settings/panels/AppearanceMotion.tsx`) na malém šuplíku.
import type { MotionStyle } from "../types.ts";
import { simulateSpring } from "./springs.ts";

/** Tvary clip-path: složený do proužku, proužek roztažený do šířky, celý šuplík. */
export type Geo = { collapsed: string; band: string; full: string };
export type Plan = {
  frames: Keyframe[];
  duration: number;
  easing: string;
  delay: number;
  /** počáteční styl panelu před ukázáním okna (`prepare`) */
  start: Partial<Record<"clipPath" | "transform" | "opacity", string>>;
  /** celá plocha se zároveň „nadechne“ (jen u morfu tvaru) */
  breath: boolean;
  /** postupné naskakování obsahu: `step` ms mezi kartami, `blur` = jako výchozí (jemné rozmazání); bez pole = výchozí */
  reveal?: { step: number; blur: boolean };
};

/** Pojistka zavření v Rustu (`Drawer::hide`) je 600 ms — zavírací plán se do ní vždy vejde. */
export const CLOSE_CAP_MS = 540;

/** Odstup naskočení karet u rozvinutí (20–30 ms na kartu). */
export const UNFOLD_STEP_MS = 25;
const ms = (base: number, speed: number) => Math.round(base / (Number.isFinite(speed) && speed > 0 ? speed : 1));
const clipMorph = new Set<MotionStyle>(["spring", "smooth", "snappy"]);

/** Otevření. `fromVisible` = přechod z už viditelného pohledu (peek → šuplík): vždy morf tvaru. */
export function openPlan(style: MotionStyle, speed: number, g: Geo, fromVisible = false): Plan | null {
  if (style === "none") return null;
  if (clipMorph.has(style) || fromVisible) {
    const sp = simulateSpring(clipMorph.has(style) ? style : "spring", speed, 240, 25);
    return { frames: [{ clipPath: g.collapsed }, { clipPath: g.full }], duration: sp.duration, easing: sp.easing, delay: 0, start: { clipPath: g.collapsed }, breath: true };
  }
  if (style === "notch") {
    return {
      frames: [
        { clipPath: g.collapsed, offset: 0, easing: "cubic-bezier(0.25, 0.8, 0.25, 1)" },
        { clipPath: g.band, offset: 0.36, easing: "cubic-bezier(0.3, 1.35, 0.5, 1)" },
        { clipPath: g.full, offset: 1 },
      ],
      duration: ms(560, speed),
      easing: "linear",
      delay: 0,
      start: { clipPath: g.collapsed },
      breath: true,
    };
  }
  if (style === "bounce") {
    const y = (p: number) => `translateY(${p}%)`;
    return {
      frames: [
        { transform: y(-100), offset: 0, easing: "cubic-bezier(0.5, 0, 1, 0.6)" },
        { transform: y(0), offset: 0.5, easing: "cubic-bezier(0, 0.4, 0.5, 1)" },
        { transform: y(-6), offset: 0.68, easing: "cubic-bezier(0.5, 0, 1, 0.6)" },
        { transform: y(0), offset: 0.82, easing: "cubic-bezier(0, 0.4, 0.5, 1)" },
        { transform: y(-1.5), offset: 0.91, easing: "cubic-bezier(0.5, 0, 1, 0.6)" },
        { transform: y(0), offset: 1 },
      ],
      duration: ms(720, speed),
      easing: "linear",
      delay: 0,
      start: { transform: y(-100) },
      breath: false,
    };
  }
  if (style === "drop") {
    // gravitace: pomalý start, rychlý dopad, jeden jemný odraz nahoru a usazení (transform + opacity, nic víc)
    return {
      frames: [
        { transform: "translateY(-60%)", opacity: 0, offset: 0, easing: "cubic-bezier(0.5, 0, 0.9, 0.5)" },
        { opacity: 1, offset: 0.3 },
        { transform: "translateY(0%)", opacity: 1, offset: 0.58, easing: "cubic-bezier(0.1, 0.6, 0.4, 1)" },
        { transform: "translateY(-2.5%)", opacity: 1, offset: 0.78, easing: "cubic-bezier(0.5, 0, 0.9, 0.5)" },
        { transform: "translateY(0%)", opacity: 1, offset: 1 },
      ],
      duration: ms(520, speed),
      easing: "linear",
      delay: 0,
      start: { transform: "translateY(-60%)", opacity: "0" },
      breath: false,
    };
  }
  if (style === "unfold") {
    return {
      frames: [
        { clipPath: g.collapsed, offset: 0, easing: "cubic-bezier(0.2, 0.8, 0.3, 1)" },
        { clipPath: g.band, offset: 0.22, easing: "cubic-bezier(0.2, 0.8, 0.3, 1)" },
        { clipPath: g.full, offset: 1 },
      ],
      duration: ms(520, speed),
      easing: "linear",
      delay: 0,
      start: { clipPath: g.collapsed },
      breath: false,
      reveal: { step: UNFOLD_STEP_MS, blur: false },
    };
  }
  // fade
  return {
    frames: [
      { opacity: 0, transform: "scale(0.96)", transformOrigin: "50% 0" },
      { opacity: 1, transform: "scale(1)", transformOrigin: "50% 0" },
    ],
    duration: ms(300, speed),
    easing: "cubic-bezier(0.2, 0.8, 0.2, 1)",
    delay: 0,
    start: { opacity: "0" },
    breath: false,
  };
}

/** Zavření (stažení do proužku). Vždy kratší než pojistka v Rustu (`CLOSE_CAP_MS` i se zpožděním). */
export function closePlan(style: MotionStyle, speed: number, g: Geo): Plan | null {
  if (style === "none") return null;
  const cap = (d: number, delay = 0) => Math.min(d, CLOSE_CAP_MS - delay);
  const base = { delay: 0, start: {}, breath: false };
  if (style === "notch") {
    return {
      ...base,
      frames: [
        { clipPath: g.full, offset: 0, easing: "cubic-bezier(0.55, 0, 0.75, 0.05)" },
        { clipPath: g.band, offset: 0.6, easing: "cubic-bezier(0.4, 0, 0.6, 1)" },
        { clipPath: g.collapsed, offset: 1 },
      ],
      duration: cap(ms(300, speed)),
      easing: "linear",
    };
  }
  if (style === "bounce") {
    return {
      ...base,
      frames: [{ transform: "translateY(0%)" }, { transform: "translateY(2%)", offset: 0.25 }, { transform: "translateY(-100%)" }],
      duration: cap(ms(280, speed)),
      easing: "cubic-bezier(0.5, 0, 0.8, 0.2)",
    };
  }
  if (style === "drop") {
    return {
      ...base,
      frames: [{ transform: "translateY(0%)", opacity: 1 }, { transform: "translateY(-40%)", opacity: 0 }],
      duration: cap(ms(240, speed)),
      easing: "cubic-bezier(0.5, 0, 0.9, 0.5)",
    };
  }
  if (style === "fade") {
    return {
      ...base,
      frames: [{ opacity: 1, transform: "scale(1)", transformOrigin: "50% 0" }, { opacity: 0, transform: "scale(0.97)", transformOrigin: "50% 0" }],
      duration: cap(ms(200, speed)),
      easing: "cubic-bezier(0.4, 0, 1, 1)",
    };
  }
  // dnešní stažení tvaru (pružina / plynulá / svižná)
  const delay = ms(40, speed);
  return { ...base, frames: [{ clipPath: g.full }, { clipPath: g.collapsed }], duration: cap(ms(240, speed), delay), delay, easing: "cubic-bezier(.55,0,.75,.05)" };
}
