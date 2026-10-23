// Design styly (vlna 6) jako čistá data bez DOM a bez závislostí (testuje `theme.test.mjs`):
//  - `STYLES` = tabulka stylů: co který styl nastaví (barvy, tvar, písmo, materiál, hustota, proužek,
//    rozložení a šířka). NOVÝ STYL = jeden záznam tady + texty v `settings/panels/en-look.ts`
//    (+ volitelně CSS blok `html[data-style="…"]` v `styles.css`, + motiv v `themes.ts`, když chce
//    vlastní barvy). Rust styl zná jen jménem (`appearance.style`).
//  - `FONTS` / `MATERIALS` / `LAYOUTS` / `WIDTHS` = složky stylu; každá dává sadu CSS proměnných,
//    které `styles.css` čte s výchozí hodnotou = dnešní vzhled. `lookVars` je složí (volá
//    `appearanceVars`). Proměnné se vždy vydávají VŠECHNY (i výchozí), aby náhled `.theme-scope`
//    jiného stylu nezdědil hodnoty z kořene.
// Popisky jsou páry [klíč, čeština] — překlad až při vykreslení (`theme/label.ts` `lbl`).
import type { Appearance, FontLook, LayoutId, Material, StripStyle, ThemeId, WidthPreset } from "../types.ts";

export type Pair = readonly [string, string];

/** Hodnoty, které styl zapíše do configu (zbytek — pozice proužku, linka, karty, animace — zůstává). */
export type StyleParts = {
  theme: ThemeId;
  density: Appearance["density"];
  fontScale: number;
  radius: Appearance["radius"];
  /** `appearance.glass.opacity` */
  opacity: number;
  strip: StripStyle;
  stripSize: Appearance["strip"]["size"];
  font: FontLook;
  material: Material;
  layout: LayoutId;
  width: WidthPreset;
  /** `drawer.glass: acrylic` (rozmazání plochy za šuplíkem) */
  blur: boolean;
};
export type StyleDef = { id: string; name: Pair; desc: Pair; parts: StyleParts };

export const CLASSIC = "classic";

export const STYLES: readonly StyleDef[] = [
  {
    id: CLASSIC,
    name: ["appearance.style.classic", "Klasický"],
    desc: ["appearance.style.classic.desc", "Ledge, jak ho znáš: tmavé plochy, jemná záře a tvoje šířka."],
    parts: { theme: "dark", density: "normal", fontScale: 1, radius: "normal", opacity: 1, strip: "pill", stripSize: "normal", font: "system", material: "solid", layout: "classic", width: "custom", blur: false },
  },
  {
    id: "glass",
    name: ["appearance.style.glass", "Sklo"],
    desc: ["appearance.style.glass.desc", "Průsvitné vrstvy a oblé tvary, panel plovoucí pod proužkem. Rozmazání plochy zapneš v detailech."],
    parts: { theme: "midnight", density: "normal", fontScale: 1, radius: "round", opacity: 0.82, strip: "pill", stripSize: "normal", font: "system", material: "glass", layout: "float", width: "medium", blur: false },
  },
  {
    id: "terminal",
    name: ["appearance.style.terminal", "Terminál"],
    desc: ["appearance.style.terminal.desc", "Zelená na černé, neproporcionální písmo, ostré rámečky a hustý pruh přes celou šířku."],
    parts: { theme: "terminal", density: "compact", fontScale: 1, radius: "sharp", opacity: 1, strip: "term", stripSize: "normal", font: "mono", material: "frame", layout: "band", width: "full", blur: false },
  },
  {
    id: "paper",
    name: ["appearance.style.paper", "Papír"],
    desc: ["appearance.style.paper.desc", "Teplý list papíru: patkové nadpisy, velké okraje, dva sloupce. Klidné a čitelné."],
    parts: { theme: "paper", density: "comfy", fontScale: 1.05, radius: "normal", opacity: 1, strip: "tag", stripSize: "normal", font: "serif", material: "paper", layout: "sheet", width: "narrow", blur: false },
  },
  {
    id: "neon",
    name: ["appearance.style.neon", "Neon"],
    desc: ["appearance.style.neon.desc", "Tmavá panoráma se zářícími okraji ve čtyřech sloupcích. Herní a večerní."],
    parts: { theme: "neon", density: "normal", fontScale: 1, radius: "round", opacity: 0.94, strip: "glow", stripSize: "normal", font: "system", material: "glow", layout: "panorama", width: "wide", blur: false },
  },
  {
    id: "brutal",
    name: ["appearance.style.brutal", "Brutal"],
    desc: ["appearance.style.brutal.desc", "Tlusté černé okraje, posunuté stíny, plné barvy a úzké verzálky. Výrazné."],
    parts: { theme: "brutal", density: "normal", fontScale: 1, radius: "sharp", opacity: 1, strip: "block", stripSize: "normal", font: "grotesk", material: "hard", layout: "mosaic", width: "medium", blur: false },
  },
  {
    id: "pastel",
    name: ["appearance.style.pastel", "Pastel"],
    desc: ["appearance.style.pastel.desc", "Měkké vystouplé dlaždice, velké oblé rohy a pastelové barvy ve třech sloupcích."],
    parts: { theme: "pastel", density: "comfy", fontScale: 1, radius: "round", opacity: 1, strip: "drop", stripSize: "normal", font: "system", material: "soft", layout: "tiles", width: "medium", blur: false },
  },
  {
    id: "aurora",
    name: ["appearance.style.aurora", "Polární záře"],
    desc: ["appearance.style.aurora.desc", "Měkké přechody tyrkysové a fialové nahoře, oblé průsvitné plochy a vzdušné rozložení."],
    parts: { theme: "aurora", density: "comfy", fontScale: 1, radius: "round", opacity: 0.9, strip: "pill", stripSize: "normal", font: "system", material: "glass", layout: "panorama", width: "wide", blur: false },
  },
  {
    id: "mono",
    name: ["appearance.style.mono", "Mono"],
    desc: ["appearance.style.mono.desc", "Jednobarevný minimalismus: šeď na bílé, tenké linky, žádné stíny a jemné verzálky."],
    parts: { theme: "mono", density: "comfy", fontScale: 1, radius: "sharp", opacity: 1, strip: "line", stripSize: "small", font: "system", material: "solid", layout: "sheet", width: "medium", blur: false },
  },
  {
    id: "retro",
    name: ["appearance.style.retro", "Retro"],
    desc: ["appearance.style.retro.desc", "Jantarová na hnědočerné jako starý monitor: strojové písmo, tvrdé stíny a pixelová linka."],
    parts: { theme: "retro", density: "normal", fontScale: 1, radius: "sharp", opacity: 1, strip: "block", stripSize: "normal", font: "mono", material: "hard", layout: "mosaic", width: "medium", blur: false },
  },
];

export const STYLE_IDS = STYLES.map((s) => s.id);
/** Styl podle jména; neznámé = Klasický. */
export const styleOf = (id: string): StyleDef => STYLES.find((s) => s.id === id) ?? STYLES[0];

// ---------- písmo (jen systémová písma Windows, žádné stahování) ----------
const SANS = '"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif';
const DISPLAY = '"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif';
const MONO = '"Cascadia Mono", "Cascadia Code", Consolas, monospace';

type Look = { name: Pair; vars: Record<string, string> };

export const FONTS: Record<FontLook, Look> = {
  system: {
    name: ["appearance.font.system", "Segoe (výchozí)"],
    vars: { sans: SANS, display: DISPLAY, "h-font": "var(--sans)", "h-weight": "700", "h-size": "var(--fs-2xs)", "h-tt": "uppercase", "h-ls": "0.1em", "h-style": "normal" },
  },
  mono: {
    name: ["appearance.font.mono", "Strojové"],
    vars: { sans: MONO, display: MONO, "h-font": "var(--mono)", "h-weight": "700", "h-size": "var(--fs-2xs)", "h-tt": "uppercase", "h-ls": "0.06em", "h-style": "normal" },
  },
  serif: {
    name: ["appearance.font.serif", "Patkové nadpisy"],
    vars: { sans: SANS, display: '"Sitka Display", "Sitka Heading", Georgia, "Times New Roman", serif', "h-font": "var(--display)", "h-weight": "600", "h-size": "var(--fs-md)", "h-tt": "none", "h-ls": "0.01em", "h-style": "italic" },
  },
  grotesk: {
    name: ["appearance.font.grotesk", "Úzké verzálky"],
    vars: { sans: `"Bahnschrift", ${SANS}`, display: `"Bahnschrift", ${DISPLAY}`, "h-font": "var(--display)", "h-weight": "700", "h-size": "var(--fs-xs)", "h-tt": "uppercase", "h-ls": "0.04em", "h-style": "normal" },
  },
};

// ---------- materiál ploch (záře, stíny, rámečky — vše statické, žádná animace v klidu) ----------
const NONE_SH = "0 0 0 0 transparent";
const SOLID: Record<string, string> = {
  "m-glow": "1", "m-shell-ring": NONE_SH, "m-sig": "1", "m-sig-glow": "none",
  "m-card-bg": "var(--card)", "m-card-bd": "1px solid var(--line)", "m-card-sh": "none",
  "m-h-color": "var(--t3)", "m-h-prefix": "none", "m-h-shadow": "none",
  "m-pill-r": "var(--r-xs)", "m-ctl-w": "1px", "m-ctl-sh": "none",
};

export const MATERIALS: Record<Material, Look> = {
  solid: { name: ["appearance.material.solid", "Plné"], vars: SOLID },
  glass: {
    name: ["appearance.material.glass", "Sklo"],
    vars: {
      ...SOLID, "m-glow": "1.3", "m-shell-ring": "inset 0 -1px 0 color-mix(in srgb, var(--t1) 16%, transparent)",
      "m-card-bg": "color-mix(in srgb, var(--t1) 6%, transparent)", "m-card-bd": "1px solid color-mix(in srgb, var(--t1) 14%, transparent)",
      "m-card-sh": "inset 0 1px 0 color-mix(in srgb, var(--t1) 10%, transparent)", "m-h-color": "var(--t2)", "m-pill-r": "var(--r-pill)",
    },
  },
  paper: {
    name: ["appearance.material.paper", "Papír"],
    vars: {
      ...SOLID, "m-glow": "0", "m-sig": "0.5",
      "m-card-bg": "var(--raised)", "m-card-sh": "0 1px 2px color-mix(in srgb, var(--logo-shadow) 45%, transparent), 0 8px 18px -10px var(--logo-shadow)",
      "m-h-color": "var(--t2)",
    },
  },
  glow: {
    name: ["appearance.material.glow", "Záře"],
    vars: {
      ...SOLID, "m-glow": "1.8",
      "m-shell-ring": "inset 0 -1px 0 color-mix(in srgb, var(--accent) 55%, transparent), inset 1px 0 0 color-mix(in srgb, var(--accent-2) 30%, transparent), inset -1px 0 0 color-mix(in srgb, var(--accent-2) 30%, transparent)",
      "m-sig-glow": "0 0 12px 1px color-mix(in srgb, var(--accent) 70%, transparent)",
      "m-card-bg": "linear-gradient(180deg, color-mix(in srgb, var(--accent-2) 14%, transparent), color-mix(in srgb, var(--accent) 5%, transparent))",
      "m-card-bd": "1px solid color-mix(in srgb, var(--accent) 38%, transparent)", "m-card-sh": "0 0 14px -2px color-mix(in srgb, var(--accent-2) 35%, transparent)",
      "m-h-color": "var(--accent)", "m-h-shadow": "0 0 8px color-mix(in srgb, var(--accent) 65%, transparent)", "m-pill-r": "var(--r-pill)",
    },
  },
  hard: {
    name: ["appearance.material.hard", "Tvrdý stín"],
    vars: {
      ...SOLID, "m-glow": "0", "m-shell-ring": "inset 0 -3px 0 var(--t1), inset 3px 0 0 var(--t1), inset -3px 0 0 var(--t1)",
      "m-card-bg": "var(--raised)", "m-card-bd": "2px solid var(--t1)", "m-card-sh": "4px 4px 0 var(--t1)",
      "m-h-color": "var(--t1)", "m-pill-r": "0px", "m-ctl-w": "2px", "m-ctl-sh": "2px 2px 0 var(--t1)",
    },
  },
  soft: {
    name: ["appearance.material.soft", "Měkké vystouplé"],
    vars: {
      ...SOLID, "m-glow": "0.6", "m-sig": "0",
      "m-card-bg": "rgb(var(--bg))", "m-card-bd": "1px solid transparent",
      "m-card-sh": "6px 6px 14px color-mix(in srgb, rgb(var(--bg)) 72%, black), -6px -6px 14px color-mix(in srgb, rgb(var(--bg)) 55%, white)",
      "m-pill-r": "var(--r-pill)",
    },
  },
  frame: {
    name: ["appearance.material.frame", "Rámečky"],
    vars: {
      ...SOLID, "m-glow": "0.3", "m-shell-ring": "inset 0 -1px 0 var(--line-2), inset 1px 0 0 var(--line), inset -1px 0 0 var(--line)",
      "m-card-bg": "transparent", "m-card-bd": "1px dashed var(--line-2)",
      "m-h-color": "var(--accent)", "m-h-prefix": '">"', "m-pill-r": "0px",
    },
  },
};

// ---------- rozložení hlavní stránky ----------
// `cols` = `--deck-cols` (null = výchozí mřížky `.deck` = 4, layout/layout.css); `--span-*` = kolik sloupců
// zabere karta velikosti s/m/l/xl (nikdy víc než sloupců; `.deck` si rozpětí ořízne sama přes `min()`).
type LayoutDef = Look & { cols: number | null };
const PAD = "var(--s4) var(--s5) var(--s5)";
export const LAYOUTS: Record<LayoutId, LayoutDef> = {
  classic: { name: ["appearance.layout.classic", "Klasické"], cols: null, vars: { "deck-gap": "var(--s3)", "deck-pad": "0px", "l-pad": PAD } },
  float: { name: ["appearance.layout.float", "Plovoucí panel"], cols: 3, vars: { "deck-gap": "var(--s3)", "deck-pad": "0px", "l-pad": PAD } },
  band: { name: ["appearance.layout.band", "Hustý pruh"], cols: 5, vars: { "deck-gap": "var(--s2)", "deck-pad": "0px", "l-pad": "var(--s2) var(--s5) var(--s3)" } },
  sheet: { name: ["appearance.layout.sheet", "List"], cols: 2, vars: { "deck-gap": "var(--s4)", "deck-pad": "0px", "l-pad": "var(--s5) calc(var(--s5) * 1.8) calc(var(--s5) * 1.4)" } },
  panorama: { name: ["appearance.layout.panorama", "Panoráma"], cols: 4, vars: { "deck-gap": "var(--s3)", "deck-pad": "6px", "l-pad": PAD } },
  mosaic: { name: ["appearance.layout.mosaic", "Mozaika"], cols: 3, vars: { "deck-gap": "var(--s4)", "deck-pad": "0 6px 6px 0", "l-pad": PAD } },
  tiles: { name: ["appearance.layout.tiles", "Dlaždice"], cols: 3, vars: { "deck-gap": "var(--s4)", "deck-pad": "10px", "l-pad": PAD } },
};

/** Šířka předvolby v px (zrcadlo `WidthPreset::px` v look.rs); null = vlastní (`drawer.width`) / celá šířka. */
export const WIDTHS: Record<WidthPreset, { name: Pair; px: number | null }> = {
  custom: { name: ["appearance.widthPreset.custom", "Vlastní"], px: null },
  narrow: { name: ["appearance.widthPreset.narrow", "Úzký"], px: 820 },
  medium: { name: ["appearance.widthPreset.medium", "Střední"], px: 1080 },
  wide: { name: ["appearance.widthPreset.wide", "Široký"], px: 1440 },
  full: { name: ["appearance.widthPreset.full", "Celá šířka"], px: null },
};

/** Sloupce, které karta velikosti s/m/l/xl zabere při `cols` sloupcích. */
export const spans = (cols: number) => ({ s: 1, m: Math.min(2, cols), l: Math.min(3, cols), xl: cols });

/** CSS proměnné písma, materiálu a rozložení (bez `--`). Klasický styl = dnešní hodnoty. */
export function lookVars(a: Pick<Appearance, "font" | "material" | "layout">): Record<string, string> {
  const layout = LAYOUTS[a.layout] ?? LAYOUTS.classic;
  const out: Record<string, string> = { ...(FONTS[a.font] ?? FONTS.system).vars, ...(MATERIALS[a.material] ?? MATERIALS.solid).vars, ...layout.vars };
  if (layout.cols !== null) {
    out["deck-cols"] = String(layout.cols);
    for (const [k, v] of Object.entries(spans(layout.cols))) out[`span-${k}`] = String(v);
  }
  return out;
}
