// Výběr design stylu (čistá logika bez DOM, testuje `theme.test.mjs`): styl zapíše své hodnoty do
// polí vzhledu (`withStyle`) a do `drawer.glass` (rozmazání); pole zůstávají přebitelná („Upravit
// detaily“), `styleChanges` řekne, které složky se od stylu liší, „Obnovit styl“ = znovu `withStyle`.
// Pozice a linka proužku, krytí proužku, karty (`layout`), animace a jazyk styl nemění.
// Tabulka stylů je `looks.ts` `STYLES`.
import type { Appearance } from "../types.ts";
import { normalizeAppearance } from "./appearance.ts";
import { styleOf, type StyleDef } from "./looks.ts";

/** Vzhled po výběru stylu `id` (z libovolného, i neúplného vzhledu). */
export function withStyle(raw: unknown, id: string): Appearance {
  const a = normalizeAppearance(raw);
  const p = styleOf(id).parts;
  return {
    ...a,
    style: styleOf(id).id,
    theme: p.theme,
    accent: "",
    density: p.density,
    fontScale: p.fontScale,
    radius: p.radius,
    glass: { opacity: p.opacity },
    strip: { ...a.strip, style: p.strip, size: p.stripSize },
    font: p.font,
    material: p.material,
    layout: p.layout,
    width: p.width,
  };
}

/** Celý config po výběru stylu (paleta příkazů): vzhled + rozmazání `drawer.glass`. Vstup se nemění. */
export function configWithStyle(cfg: any, id: string): any {
  const blur = styleOf(id).parts.blur;
  return { ...cfg, appearance: withStyle(cfg?.appearance, id), drawer: { ...(cfg?.drawer ?? {}), glass: blur ? "acrylic" : "solid" } };
}

/** Složky stylu, které jde přebít (pořadí = pořadí v panelu). */
export const PARTS = ["colors", "font", "material", "density", "shape", "strip", "layout"] as const;
export type PartId = (typeof PARTS)[number];

/** Které složky se liší od výchozích hodnot zvoleného stylu (`blur` = `drawer.glass === "acrylic"`). */
export function styleChanges(a: Appearance, blur: boolean): PartId[] {
  const s: StyleDef = styleOf(a.style);
  const p = s.parts;
  const diff: Record<PartId, boolean> = {
    colors: a.theme !== p.theme || a.accent !== "",
    font: a.font !== p.font || a.fontScale !== p.fontScale,
    material: a.material !== p.material || a.glass.opacity !== p.opacity || blur !== p.blur,
    density: a.density !== p.density,
    shape: a.radius !== p.radius,
    strip: a.strip.style !== p.strip || a.strip.size !== p.stripSize,
    layout: a.layout !== p.layout || a.width !== p.width,
  };
  return PARTS.filter((k) => diff[k]);
}
