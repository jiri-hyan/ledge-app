// Panel Vzhled (vlna 7) jako čistá data a výpočty bez DOM (testuje `appearanceModel.test.mjs`):
//  - kroky panelu (Zavřený · Po najetí · Otevřený · Pohyb) a kde leží která kotva hledání `f-<id>`
//    (klíč z `labels.ts`) — i jestli v rozbalovacím „Pokročilé“, ať skok z hledání přepne krok a rozbalí ho;
//  - volby přepínačů v panelu (nejvýš 4 — víc = galerie dlaždic, hlídá `segments.test.mjs`);
//  - náhled „monitor ve skutečném poměru“: šířka šuplíku v px (zrcadlo Rust `appearance::geom::preset_width`),
//    skutečný počet sloupců mřížky (rozložení stylu, `theme/looks.ts` `LAYOUTS`) a ukázkové karty.
import type { Appearance, LayoutId, WidthPreset } from "../types.ts";
import { LAYOUTS, NOTCH_PX, WIDTHS } from "../theme/looks.ts";
import { sizeFor } from "../theme/size.ts";

export const STEPS = ["closed", "hover", "open", "motion"] as const;
export type StepId = (typeof STEPS)[number];

/** Kotva → krok panelu. `null` = mimo kroky (vidět vždy, třeba jazyk). Každá volba panelu Vzhled tu musí být. */
export const ANCHOR_STEP: Record<string, StepId | null> = {
  "strip.look": "closed",
  "strip.compose.deco": "closed",
  "strip.compose.fill": "closed",
  "strip.compose.sep": "closed",
  "strip.compose.mono": "closed",
  "appearance.strip": "closed",
  "strip.enabled": "closed",
  "strip.items": "closed",
  "strip.custom": "closed",
  "timer.strip": "closed",
  "strip.monitor": "closed",
  "drawer.hover": "hover",
  "drawer.hoverExpandMs": "hover",
  "appearance.style": "open",
  "appearance.layout": "open",
  "appearance.density": "open",
  "appearance.theme": "open",
  "appearance.pages": "open",
  "appearance.size": "open",
  "appearance.size.perPage": "open",
  "appearance.accent": "open",
  "appearance.accentGrad": "open",
  "appearance.backdrop": "open",
  "appearance.backdrop.cards": "open",
  "appearance.details": "open",
  "drawer.width": "open",
  "drawer.glass": "open",
  "appearance.motion": "motion",
};

/** Kotvy schované v rozbalovacím „Pokročilé“ svého kroku (skok na ně ho rozbalí). „Co proužek ukazuje“
 *  (položky, vlastní text, časovač, monitor) je od kola 21 v Základním — není to technika. */
export const ADVANCED: readonly string[] = [
  "drawer.hoverExpandMs",
  "appearance.details", "drawer.width", "drawer.glass",
];

/** Krok, na který přepnout kvůli kotvě (`undefined` = kotva nepatří žádnému kroku). */
export function stepOf(anchor: string | null | undefined): StepId | undefined {
  if (!anchor) return undefined;
  return ANCHOR_STEP[anchor] ?? undefined;
}
export const isAdvanced = (anchor: string | null | undefined) => !!anchor && ADVANCED.includes(anchor);

/** Šířka v kroku Otevřený (4 volby; „Vlastní“ px je v Pokročilé). */
export const WIDTH_STEPS: readonly WidthPreset[] = ["narrow", "medium", "wide", "full"];
/** Hustota od nejvzdušnější (`appearance.density`). */
export const DENSITY_STEPS: readonly Appearance["density"][] = ["comfy", "normal", "compact"];
export const POSITION_STEPS: readonly Appearance["strip"]["position"][] = ["left", "center", "right"];
export const SIZE_STEPS: readonly Appearance["strip"]["size"][] = ["small", "normal", "large"];
/** Co se v náhledech zavřeného Ledge a po najetí zrovna „děje“ (ukázková data). */
export const PREVIEW_STATES = ["idle", "work", "wait", "music", "timer", "done"] as const;
export type PreviewState = (typeof PREVIEW_STATES)[number];

// ---------- monitor ve skutečném poměru ----------
/** Rozsah `drawer.width` a strop podílu monitoru (zrcadlo `appearance::DRAWER_WIDTH`, `geom::MAX_SHARE`). */
export const DRAWER_MIN = 720;
export const DRAWER_MAX = 2400;
export const MAX_SHARE = 0.95;
/** Odstup šuplíku od hrany u pozice vlevo / vpravo (zrcadlo `geom::MARGIN`, logické px). */
export const EDGE_MARGIN = 24;
/** Monitor, když jeho rozměr neznáme (náhled mimo Ledge): 16 : 9. */
export const FALLBACK_MONITOR = { w: 1920, h: 1080 };

/** Šířka velkého šuplíku v logických px — zrcadlo `geom::preset_width` (monitor už v logických px). */
export function drawerWidth(preset: WidthPreset, custom: number, monW: number): number {
  if (preset === "full") return monW > 0 ? Math.floor(monW) : DRAWER_MAX;
  const px = WIDTHS[preset]?.px ?? custom;
  const w = Math.min(DRAWER_MAX, Math.max(DRAWER_MIN, Math.round(Number.isFinite(px) ? px : 1080)));
  return monW > 0 ? Math.min(w, Math.floor(monW * MAX_SHARE)) : w;
}

/** Levý okraj šuplíku na monitoru (logické px) — zrcadlo `geom::anchor_c2` + `span_x` pro monitor od 0. */
export function drawerLeft(pos: Appearance["strip"]["position"], monW: number, w: number): number {
  const full = Math.min(Math.max(0, w), monW);
  if (pos === "left") return Math.min(EDGE_MARGIN, monW - full);
  if (pos === "right") return Math.max(monW - EDGE_MARGIN - full, 0);
  return Math.floor((monW - full) / 2);
}

/** Skutečný počet sloupců mřížky karet (`--deck-cols` stylu; bez něj mřížka `.deck` = 4). */
export const deckCols = (layout: LayoutId): number => LAYOUTS[layout]?.cols ?? 4;

export type MonitorModel = {
  /** rozměr monitoru (logické px); `known` = z `window.screen`, jinak 16 : 9 bez čísel */
  monW: number;
  monH: number;
  known: boolean;
  drawerW: number;
  /** levý okraj šuplíku (logické px od levé hrany monitoru) */
  left: number;
  /** podíl šířky šuplíku na monitoru 0–1 */
  share: number;
  cols: number;
};

export function monitorModel(
  a: Pick<Appearance, "width" | "layout" | "size"> & { strip: Pick<Appearance["strip"], "position"> },
  customW: number,
  screen: { w: number; h: number } | null,
): MonitorModel {
  const known = !!screen && screen.w > 0 && screen.h > 0;
  const mon = known ? screen! : FALLBACK_MONITOR;
  // široký notch má vlastní šířku (Rust `geom::layout_width`), předvolba se nepoužije
  // vlastní velikost (`appearance.size`, kolo 23; bez záložky = celek) přebije předvolbu jako Rust `geom::sized_width`
  const own = sizeFor(a.size, "").width;
  const w = a.layout === "notch" ? drawerWidth("custom", NOTCH_PX, known ? mon.w : 0)
    : own > 0 ? drawerWidth("custom", own, known ? mon.w : 0) : drawerWidth(a.width, customW, known ? mon.w : 0);
  const drawerW = Math.min(w, mon.w);
  return {
    monW: mon.w, monH: mon.h, known, drawerW,
    left: drawerLeft(a.strip.position, mon.w, drawerW),
    share: drawerW / mon.w, cols: deckCols(a.layout),
  };
}

/** Rozpětí ukázkových karet do dvou řad mřížky o `cols` sloupcích: první karta široká (2), zbytek po jedné. */
export function sampleSpans(cols: number): number[] {
  const c = Math.max(1, Math.floor(cols));
  const first = Math.min(2, c);
  return [first, ...Array.from({ length: c - first + c }, () => 1)];
}
