// DOM strana vzhledu: `applyAppearance` zapíše CSS proměnné motivu, škál a design stylu inline na <html>
// (`data-theme`, `data-motion`, `data-style`, `data-layout`, `data-material`, `data-font`,
// `color-scheme`) a předá styl/rychlost animací a tvary morfu do
// motion.ts. `initAppearance` (App při startu) načte uložený vzhled z `get_config` a poslouchá
// událost `appearance` (Rust po uložení configu) → změna bez restartu. Panel Vzhled náhledově
// aplikuje rozpracovaný config a při odchodu vrátí `savedAppearance()`. Aktivní záložka hlavní stránky
// (`setActivePage`, volá `MainPage`) může mít vlastní vzhled (`theme/pages.ts`): přepnutí záložky přehraje
// krátký přechod barev (`html[data-look-shift]` ve `styles.css`, jen po dobu přechodu — v klidu nic).
import { createSignal } from "solid-js";
import { api, on } from "../api";
import { setMotion, setShellRadius, setStripRadius } from "../motion";
import type { Appearance } from "../types";
import { DEFAULT_APPEARANCE, RADIUS, appearanceVars, effectiveMotion, normalizeAppearance, stripShape } from "./appearance";
import { themeOf } from "./customThemes";
import { effectiveAppearance } from "./pages";

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

/** Proměnné zapsané minule — co nový vzhled nevydá (např. `--deck-cols` u Klasického), se smaže. */
let written = new Set<string>();

/** Záložka hlavní stránky, která je zrovna vidět ("" = žádná, třeba Nastavení → platí celkový vzhled). */
let activePage = "";
/** Záložka hlavní stránky, která je zrovna vidět ("" = žádná) — pro zkratky, které otevírají panel „Vzhled karty“. */
export const activePageId = () => activePage;
/** Naposledy použitý vzhled — přepnutí záložky ho jen znovu použije s přepisem záložky. */
let lastRaw: unknown = DEFAULT_APPEARANCE;
let shiftTimer = 0;

export function applyAppearance(raw: unknown) {
  lastRaw = raw;
  const full = normalizeAppearance(raw);
  const a = effectiveAppearance(full, activePage);
  const root = document.documentElement;
  const vars = appearanceVars(a, reduced());
  const now = new Set(Object.keys(vars));
  for (const k of written) if (!now.has(k)) root.style.removeProperty(`--${k}`);
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(`--${k}`, v);
  written = now;
  const motion = effectiveMotion(a.motion.style, reduced());
  const theme = themeOf(a);
  root.dataset.theme = a.customTheme ? "custom" : a.theme;
  root.dataset.motion = motion;
  root.dataset.style = a.style;
  root.dataset.layout = a.layout;
  root.dataset.material = a.material;
  root.dataset.font = a.font;
  root.style.colorScheme = theme.light ? "light" : "dark";
  // pro CSS, které potřebuje tón motivu (podklad ikon opačného tónu, `ui/iconTone.ts`)
  root.dataset.scheme = theme.light ? "light" : "dark";
  setMotion(motion, a.motion.speed);
  setShellRadius(26 * RADIUS[a.radius]);
  setStripRadius(stripShape(a.strip.style, a.strip.size).r);
}

/** Aktivní záložka hlavní stránky. Změna s jiným vzhledem záložky přehraje krátký přechod barev. */
export function setActivePage(id: string) {
  if (id === activePage) return;
  const pages = normalizeAppearance(lastRaw).pages;
  const differs = !!pages[activePage] || !!pages[id];
  activePage = id;
  const root = document.documentElement;
  const shift = differs && root.dataset.motion !== "none" && !reduced();
  if (shift) {
    root.dataset.lookShift = "1";
    clearTimeout(shiftTimer);
    shiftTimer = window.setTimeout(() => delete root.dataset.lookShift, 450);
  }
  applyAppearance(lastRaw);
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
