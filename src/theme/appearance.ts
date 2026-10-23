// Čistá logika vzhledu (bez DOM, testuje `theme.test.mjs`): výchozí hodnoty a doplnění configu,
// CSS proměnné pro motiv + akcent + škály (`appearanceVars`), odvozený akcent (`accentPair`), tvar proužku pro morf (`stripShape`, zrcadlo `appearance::geom::strip_metrics`)
// a presety pružin (`springPreset`). Design styly (tabulka, písmo, materiál, rozložení) jsou v
// `looks.ts`, výběr stylu v `styles.ts`. DOM a události řeší `apply.ts`.
import type { Appearance, MotionStyle, StripStyle } from "../types.ts";
import { THEME_IDS } from "./themes.ts";
import { normalizeCustomThemes, themeOf } from "./customThemes.ts";
import { normalizePages } from "./pages.ts";
import { CLASSIC, FONTS, LAYOUTS, MATERIALS, STYLE_IDS, WIDTHS, lookVars } from "./looks.ts";

export const DEFAULT_APPEARANCE: Appearance = {
  style: CLASSIC,
  theme: "dark",
  accent: "",
  density: "normal",
  fontScale: 1,
  radius: "normal",
  glass: { opacity: 1 },
  motion: { style: "spring", speed: 1 },
  strip: { style: "pill", size: "normal", color: "", position: "center" },
  font: "system",
  material: "solid",
  layout: "classic",
  width: "custom",
  pages: {},
  customThemes: [],
  customTheme: "",
};

export const DENSITY = { compact: 0.78, normal: 1, comfy: 1.22 } as const;
export const RADIUS = { sharp: 0.35, normal: 1, round: 1.4 } as const;
export const STRIP_SIZE = { small: 0.85, normal: 1, large: 1.2 } as const;
/** Styly animací v pořadí panelu (zrcadlo `MotionStyle` v appearance.rs). */
export const MOTION_STYLES: MotionStyle[] = ["spring", "smooth", "snappy", "bounce", "fade", "notch", "drop", "unfold", "none"];
export const STRIP_STYLES: StripStyle[] = ["pill", "line", "dot", "term", "tag", "glow", "block", "drop"];
const keysOf = <T extends string>(o: Record<T, unknown>) => Object.keys(o) as T[];

const pick = <T extends string>(v: unknown, ok: readonly T[], d: T): T => (ok.includes(v as T) ? (v as T) : d);
const num = (v: unknown, lo: number, hi: number, d: number) => (typeof v === "number" && v >= lo && v <= hi ? v : d);
export const isHex = (s: unknown): s is string => typeof s === "string" && /^#[0-9a-fA-F]{6}$/.test(s);

/** Config z Rustu (nebo rozpracovaný v panelu) → úplný a platný vzhled; neplatné = výchozí. */
export function normalizeAppearance(raw: any): Appearance {
  const d = DEFAULT_APPEARANCE;
  const r = raw ?? {};
  const customThemes = normalizeCustomThemes(r.customThemes);
  return {
    style: pick(r.style, STYLE_IDS, CLASSIC),
    theme: pick(r.theme, THEME_IDS, d.theme),
    accent: isHex(r.accent) ? r.accent : "",
    density: pick(r.density, ["compact", "normal", "comfy"] as const, d.density),
    fontScale: num(r.fontScale, 0.85, 1.25, 1),
    radius: pick(r.radius, ["sharp", "normal", "round"] as const, d.radius),
    glass: { opacity: num(r.glass?.opacity, 0.25, 1, 1) },
    motion: { style: pick(r.motion?.style, MOTION_STYLES, "spring"), speed: num(r.motion?.speed, 0.5, 2, 1) },
    strip: {
      style: pick(r.strip?.style, STRIP_STYLES, "pill"),
      size: pick(r.strip?.size, ["small", "normal", "large"] as const, "normal"),
      color: r.strip?.color === "accent" || isHex(r.strip?.color) ? r.strip.color : "",
      position: pick(r.strip?.position, ["center", "left", "right"] as const, "center"),
    },
    font: pick(r.font, keysOf(FONTS), d.font),
    material: pick(r.material, keysOf(MATERIALS), d.material),
    layout: pick(r.layout, keysOf(LAYOUTS), d.layout),
    width: pick(r.width, keysOf(WIDTHS), d.width),
    pages: normalizePages(r.pages, customThemes),
    customThemes,
    customTheme: customThemes.some((c) => c.id === r.customTheme) ? r.customTheme : "",
  };
}

/** `#rrggbb` → [h 0–360, s 0–1, l 0–1]. */
export function hexToHsl(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => v / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h * 60, s, l];
}

export function hslToHex(h: number, s: number, l: number): string {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return "#" + [f(0), f(8), f(4)].map((v) => Math.round(v * 255).toString(16).padStart(2, "0")).join("");
}

/** Druhá barva podpisového přechodu: odstín posunutý o −40° (fialová → modrá jako výchozí přechod). */
export function accentPair(hex: string): [string, string] {
  const [h, s, l] = hexToHsl(hex);
  return [hex.toLowerCase(), hslToHex((h + 320) % 360, s, l)];
}

/** Systémové „omezit pohyb“ = styl `none`. */
export const effectiveMotion = (style: MotionStyle, reduced: boolean): MotionStyle => (reduced ? "none" : style);

/** Mikroanimace (CSS): pružné `--spring` a `--ease` podle stylu. */
const CSS_EASE: Record<Exclude<MotionStyle, "none">, { spring: string; ease: string }> = {
  spring: { spring: "cubic-bezier(0.34, 1.56, 0.64, 1)", ease: "cubic-bezier(0.2, 0.9, 0.25, 1.05)" },
  smooth: { spring: "cubic-bezier(0.25, 0.8, 0.3, 1)", ease: "cubic-bezier(0.2, 0.8, 0.3, 1)" },
  snappy: { spring: "cubic-bezier(0.3, 1.3, 0.5, 1)", ease: "cubic-bezier(0.1, 0.95, 0.2, 1.02)" },
  bounce: { spring: "cubic-bezier(0.34, 1.9, 0.6, 1)", ease: "cubic-bezier(0.25, 1.15, 0.4, 1.08)" },
  fade: { spring: "cubic-bezier(0.25, 0.8, 0.3, 1)", ease: "cubic-bezier(0.2, 0.8, 0.3, 1)" },
  notch: { spring: "cubic-bezier(0.3, 1.35, 0.5, 1)", ease: "cubic-bezier(0.2, 0.9, 0.25, 1.05)" },
  drop: { spring: "cubic-bezier(0.34, 1.6, 0.64, 1)", ease: "cubic-bezier(0.2, 0.9, 0.25, 1.05)" },
  unfold: { spring: "cubic-bezier(0.25, 0.8, 0.3, 1)", ease: "cubic-bezier(0.2, 0.8, 0.3, 1)" },
};

export type WidenMotion = { shrinkMs: number; fadeMs: number; restoreMs: number; easing: string };

/** Přechod rozšíření/zúžení Nastavení (`settings/widen.ts`) podle stylu pohybu. `null` = okamžitě, bez
 *  přechodu (styl `none` nebo systémové „omezit pohyb“). Délky = základ / rychlost (jako `dur` v
 *  motion.ts a `--dur-*` v CSS), křivka zúžení = `--ease` stylu. Rozšíření jde pružinou `spring()`. */
export function widenMotion(style: MotionStyle, speed: number, reduced: boolean): WidenMotion | null {
  const s = effectiveMotion(style, reduced);
  if (s === "none") return null;
  const k = Number.isFinite(speed) && speed > 0 ? speed : 1;
  const ms = (base: number) => Math.round(base / k);
  return { shrinkMs: ms(210), fadeMs: ms(120), restoreMs: ms(160), easing: CSS_EASE[s].ease };
}

/** Všechny CSS proměnné vzhledu (bez `--`) → hodnota. Výchozí vzhled = hodnoty ze `styles.css`. */
export function appearanceVars(a: Appearance, reducedMotion = false): Record<string, string> {
  const out: Record<string, string> = { ...themeOf(a).vars };
  if (isHex(a.accent)) {
    const [c1, c2] = accentPair(a.accent);
    out.accent = c1;
    out["accent-2"] = c2;
    out.focus = c1;
  }
  out.fz = String(a.fontScale);
  out.dens = String(DENSITY[a.density]);
  out.rr = String(RADIUS[a.radius]);
  out.glass = String(a.glass.opacity);
  out.speed = String(a.motion.speed);
  const style = effectiveMotion(a.motion.style, reducedMotion);
  const ease = CSS_EASE[style === "none" ? "spring" : style];
  out.spring = ease.spring;
  out.ease = ease.ease;
  return { ...out, ...lookVars(a) };
}

/** Výška a rádius proužku (logické px) — výchozí tvar morfu ve frontendu. */
const STRIP_HR: Record<StripStyle, [number, number]> = {
  pill: [32, 14], line: [26, 3], dot: [22, 11], term: [24, 0], tag: [30, 6], glow: [28, 14], block: [30, 0], drop: [34, 17],
};
export function stripShape(style: StripStyle, size: Appearance["strip"]["size"]): { h: number; r: number } {
  const k = STRIP_SIZE[size];
  const [h, r] = STRIP_HR[style] ?? STRIP_HR.pill;
  return { h: h * k, r: r * k };
}

/** Pružina podle stylu: `null` = bez animace; `smooth` tlumí kriticky (bez překmitu), `snappy` je
 *  tužší a skoro bez překmitu. Parametry jdou do simulace `spring()` v motion.ts. */
export function springPreset(style: MotionStyle, stiffness: number, damping: number, mass = 1): { stiffness: number; damping: number } | null {
  const critical = (k: number) => 2 * Math.sqrt(k * mass);
  switch (style) {
    case "none":
      return null;
    case "smooth":
    case "fade":
      return { stiffness, damping: Math.max(damping, critical(stiffness)) };
    case "bounce":
      return { stiffness: stiffness * 1.15, damping: damping * 0.62 };
    case "snappy": {
      const k = stiffness * 1.7;
      return { stiffness: k, damping: Math.max(damping, 0.85 * critical(k)) };
    }
    default:
      return { stiffness, damping };
  }
}
