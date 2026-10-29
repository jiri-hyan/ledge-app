// --- appearance ---
// Zrcadlo `AppearanceConfig` (src-tauri/src/appearance.rs) — `config.appearance`, událost `appearance`.
export type ThemeId = "dark" | "midnight" | "graphite" | "nord" | "light" | "contrast" | "terminal" | "paper" | "neon" | "brutal" | "pastel" | "aurora" | "mono" | "retro" | "oled" | "warm";
export type MotionStyle = "spring" | "smooth" | "snappy" | "bounce" | "fade" | "notch" | "drop" | "unfold" | "liquid" | "none";
export type StripPosition = "center" | "left" | "right";
export type StripStyle = "pill" | "line" | "dot" | "term" | "tag" | "glow" | "block" | "drop";
/** styl zavřeného Ledge (`strip.look`); seznam s názvy je `strip/looks.ts` */
export type { StripLookId } from "../strip/looks";
/** vlastní zavřený Ledge (`strip.compose`, Rust `looks::compose::Compose`); seznamy a předvolby jsou `strip/compose.ts` */
export type { StripCompose, ComposeShape, ComposeItem, ItemStyle } from "../strip/compose";
/** složky design stylu (vlna 6, `appearance/look.rs`); tabulka stylů je `theme/styles.ts` */
export type FontLook = "system" | "mono" | "serif" | "grotesk";
export type Material = "solid" | "glass" | "paper" | "glow" | "hard" | "soft" | "frame" | "acrylic" | "tinted";
export type LayoutId = "classic" | "float" | "band" | "sheet" | "panorama" | "mosaic" | "tiles" | "panel" | "notch";
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
  /** velikost otevřeného šuplíku (kolo 23); chybí = šířka z předvolby, výška podle obsahu */
  size?: DrawerSize;
  /** pozadí otevřeného šuplíku (kolo 25); chybí = dnešní pozadí */
  backdrop?: Backdrop;
  /** druhá barva gradientního akcentu `#rrggbb` (kolo 25); chybí = odvozená z `accent` */
  accent2?: string;
  /** akcent z barev pozadí (kolo 25); chybí = vypnuto */
  accentFromBackdrop?: boolean;
}
/** Druh gradientu (zrcadlo `appearance::backdrop::KINDS`). */
export type GradKind = "linear" | "radial" | "conic" | "mesh";
export interface GradStop { c: string; p: number }
/** Vlastní gradient (zrcadlo `appearance::backdrop::CustomGradient`): 2–5 zastávek, úhel 0–360. */
export interface CustomGradient { id: string; name: string; kind: GradKind; angle: number; stops: GradStop[] }
export type LiveKind = "fire" | "aurora" | "lava" | "stars" | "rain" | "grain";
export type CardsMode = "auto" | "solid" | "tint" | "glass";
export type FlameColor = "classic" | "blue" | "green" | "violet";
/** `appearance.backdrop` (zrcadlo `appearance::backdrop::BackdropConfig`). */
export interface Backdrop {
  /** hotový (`theme/backdropPresets.ts`) nebo vlastní (`my-…`) gradient; prázdné = bez gradientu */
  id: string;
  live: LiveKind | "";
  flame: FlameColor;
  /** 0.1–1 */
  intensity: number;
  /** 0.3–2 */
  speed: number;
  /** karty nad pozadím: `auto` = průsvitné, když je pozadí zvolené */
  cards: CardsMode;
  custom: CustomGradient[];
}
/** Šířka / výška v logických px, 0 / chybí = výchozí (zrcadlo `appearance::size::PageSize`). */
export interface PageSize { width?: number; height?: number }
/** `appearance.size` (zrcadlo `appearance::size::SizeConfig`): `perPage` = pamatovat pro každou záložku. */
export interface DrawerSize extends PageSize { perPage?: boolean; pages?: Record<string, PageSize> }
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
