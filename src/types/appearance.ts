// --- appearance ---
// Zrcadlo `AppearanceConfig` (src-tauri/src/appearance.rs) — `config.appearance`, událost `appearance`.
export type ThemeId = "dark" | "midnight" | "graphite" | "nord" | "light" | "contrast" | "terminal" | "paper" | "neon" | "brutal" | "pastel" | "aurora" | "mono" | "retro";
export type MotionStyle = "spring" | "smooth" | "snappy" | "bounce" | "fade" | "notch" | "drop" | "unfold" | "none";
export type StripPosition = "center" | "left" | "right";
export type StripStyle = "pill" | "line" | "dot" | "term" | "tag" | "glow" | "block" | "drop";
/** styl zavřeného Ledge (`strip.look`); seznam s názvy je `strip/looks.ts` */
export type { StripLookId } from "../strip/looks";
/** složky design stylu (vlna 6, `appearance/look.rs`); tabulka stylů je `theme/styles.ts` */
export type FontLook = "system" | "mono" | "serif" | "grotesk";
export type Material = "solid" | "glass" | "paper" | "glow" | "hard" | "soft" | "frame";
export type LayoutId = "classic" | "float" | "band" | "sheet" | "panorama" | "mosaic" | "tiles";
export type WidthPreset = "custom" | "narrow" | "medium" | "wide" | "full";
export interface Appearance {
  /** design styl (`classic` = vzhled před vlnou 6); hodnoty stylu jsou v polích níž */
  style: string;
  theme: ThemeId;
  /** `#rrggbb`, prázdné = výchozí akcent motivu */
  accent: string;
  density: "compact" | "normal" | "comfy";
  /** 0.85–1.25 */
  fontScale: number;
  radius: "sharp" | "normal" | "round";
  /** krytí pozadí šuplíku jako násobek krytí motivu (0.25–1) */
  glass: { opacity: number };
  /** speed 0.5–2 (2 = dvakrát rychleji) */
  motion: { style: MotionStyle; speed: number };
  /** color: prázdné = výchozí přechod, `accent`, nebo `#rrggbb` */
  strip: { style: StripStyle; size: "small" | "normal" | "large"; color: string; position: StripPosition };
  font: FontLook;
  material: Material;
  layout: LayoutId;
  /** předvolba šířky; `custom` = `drawer.width` v px */
  width: WidthPreset;
  /** vzhled jednotlivých záložek podle id (`layout.pages`); chybí = jako celek */
  pages: Record<string, PageLook>;
  /** vlastní motivy (editor v Nastavení → Vzhled), nejvýš 20 */
  customThemes: CustomTheme[];
  /** vybraný vlastní motiv pro celý vzhled (`id`), prázdné = vestavěný `theme` */
  customTheme: string;
}
/** Vzhled jedné záložky: prázdné pole = jako celek (`theme` = vestavěný motiv nebo `id` vlastního). */
export interface PageLook {
  style?: string;
  theme?: string;
  accent?: string;
}
/** Vlastní motiv (zrcadlo `appearance::pages::CustomTheme`): `colors` = všech 11 klíčů `CUSTOM_KEYS` jako `#rrggbb`. */
export interface CustomTheme {
  id: string;
  name: string;
  base: ThemeId;
  colors: Record<string, string>;
}

// Co se stane po najetí myší (`drawer.hover`, data `src/hover/modes.ts`); chybí v configu = `full`.
// `StripLookId` (vzhled zavřeného Ledge) je re-export z `src/strip/looks.ts` výše.
export type { HoverModeId } from "../hover/modes";
