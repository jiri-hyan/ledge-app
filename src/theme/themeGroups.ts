// Skupiny motivů pro výběr v Nastavení → Vzhled. Skupina je DATA u palety (`palettes.json` `group`),
// ne ruční seznam v UI; `groupFitsLuminance` (čistě) hlídá, že skupina odpovídá jasu pozadí motivu.

import PAL from "./palettes.json" with { type: "json" };
import { THEME_IDS, THEMES } from "./themes.ts";
import { lum, parseColor } from "./contrast.ts";
import type { ThemeId } from "../types.ts";

export type ThemeGroup = "dark" | "light" | "contrast";

/** Pořadí skupin v nabídce + česká podoba a klíč pro `t()`. */
export const THEME_GROUPS: { id: ThemeGroup; key: string; cs: string }[] = [
  { id: "dark", key: "appearance.group.dark", cs: "Tmavé" },
  { id: "light", key: "appearance.group.light", cs: "Světlé" },
  { id: "contrast", key: "appearance.group.contrast", cs: "Vysoký kontrast" },
];

export const groupOf = (id: ThemeId): ThemeGroup => (PAL[id] as { group: ThemeGroup }).group;

/** Vestavěné motivy skupiny v pořadí `THEME_IDS`. */
export const themesIn = (g: ThemeGroup): ThemeId[] => THEME_IDS.filter((id) => groupOf(id) === g);

/** Svítivost plochy motivu (token `bg` = „r g b“). */
export const bgLuminance = (id: ThemeId): number => lum(parseColor(THEMES[id].vars.bg));

/**
 * Sedí skupina na jas pozadí? tmavé < 0,18 (= střední šedá), světlé ≥ 0,18; vysoký kontrast = krajnost
 * (téměř černé ≤ 0,02 nebo téměř bílé ≥ 0,85).
 */
export function groupFitsLuminance(g: ThemeGroup, l: number): boolean {
  if (g === "dark") return l < 0.18;
  if (g === "light") return l >= 0.18;
  return l <= 0.02 || l >= 0.85;
}
