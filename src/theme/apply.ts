// DOM strana vzhledu: `applyAppearance` zapíše CSS proměnné motivu a škál inline na <html>
// (`data-theme`, `data-motion`, `color-scheme`) a předá styl/rychlost animací a tvary morfu do
// motion.ts. `initAppearance` (App při startu) načte uložený vzhled z `get_config` a poslouchá
// událost `appearance` (Rust po uložení configu) → změna bez restartu. Panel Vzhled náhledově
// aplikuje rozpracovaný config a při odchodu vrátí `savedAppearance()`.
import { createSignal } from "solid-js";
import { api, on } from "../api";
import { setMotion, setShellRadius, setStripRadius } from "../motion";
import type { Appearance } from "../types";
import { DEFAULT_APPEARANCE, RADIUS, appearanceVars, effectiveMotion, normalizeAppearance, stripShape } from "./appearance";
import { THEMES } from "./themes";

const [saved, setSaved] = createSignal<Appearance>(DEFAULT_APPEARANCE);
/** Uložený (platný) vzhled — čte ho i `MainPage` (pořadí karet). */
export const savedAppearance = saved;

const reducedQuery = typeof matchMedia === "function" ? matchMedia("(prefers-reduced-motion: reduce)") : null;
const reduced = () => reducedQuery?.matches ?? false;

/** CSS proměnné jako inline styl (pro `.theme-scope` náhledy v panelu). */
export function scopeStyle(a: Appearance): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(appearanceVars(a, reduced()))) out[`--${k}`] = v;
  return out;
}

export function applyAppearance(raw: unknown) {
  const a = normalizeAppearance(raw);
  const root = document.documentElement;
  for (const [k, v] of Object.entries(appearanceVars(a, reduced()))) root.style.setProperty(`--${k}`, v);
  const motion = effectiveMotion(a.motion.style, reduced());
  root.dataset.theme = a.theme;
  root.dataset.motion = motion;
  root.style.colorScheme = THEMES[a.theme].light ? "light" : "dark";
  // pro CSS, které potřebuje tón motivu (podklad ikon opačného tónu, `ui/iconTone.ts`)
  root.dataset.scheme = THEMES[a.theme].light ? "light" : "dark";
  setMotion(motion, a.motion.speed);
  setShellRadius(26 * RADIUS[a.radius]);
  setStripRadius(stripShape(a.strip.style, a.strip.size).r);
}

function commit(raw: unknown) {
  const a = normalizeAppearance(raw);
  setSaved(a);
  applyAppearance(a);
}

/** Start šuplíku: uložený vzhled + posluchače (událost `appearance`, systémové „omezit pohyb“). */
export function initAppearance(): Promise<() => void> {
  void api.getConfig().then((cfg) => cfg && commit(cfg.appearance), () => {});
  const onReduced = () => applyAppearance(saved());
  reducedQuery?.addEventListener("change", onReduced);
  return on<Appearance>("appearance", commit).then((off) => () => {
    off();
    reducedQuery?.removeEventListener("change", onReduced);
  });
}
