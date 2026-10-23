// Vzhled jednotlivých záložek jako čistá logika bez DOM (testuje `theme.test.mjs`, zrcadlo `appearance/pages.rs`):
// `appearance.pages[id]` = { style?, theme?, accent? } přebije celkový vzhled, dokud je záložka aktivní.
// Prázdné pole = jako celek. Styl záložky přepíše barvy, hustotu, rohy, písmo, materiál a rozložení stylu
// (jako `withStyle`); proužek, animace a šířka okna zůstávají celkové (okno a proužek jsou jedny pro všechny).
// `theme` je vestavěný motiv, nebo `id` vlastního (`customThemes.ts`).
import type { Appearance, CustomTheme, PageLook, ThemeId } from "../types.ts";
import { THEME_IDS } from "./themes.ts";
import { STYLE_IDS, styleOf } from "./looks.ts";

const HEX = /^#[0-9a-fA-F]{6}$/;
const isBuiltin = (id: string): id is ThemeId => (THEME_IDS as readonly string[]).includes(id);

/** Platné záznamy z configu: neznámý styl / motiv / vadný akcent se z pole vynechá, prázdný záznam zmizí. */
export function normalizePages(raw: unknown, customs: readonly CustomTheme[]): Record<string, PageLook> {
  const out: Record<string, PageLook> = {};
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return out;
  for (const [id, r] of Object.entries(raw as Record<string, any>)) {
    if (!id || !r || typeof r !== "object") continue;
    const p: PageLook = {};
    if (typeof r.style === "string" && STYLE_IDS.includes(r.style)) p.style = r.style;
    if (typeof r.theme === "string" && (isBuiltin(r.theme) || customs.some((c) => c.id === r.theme))) p.theme = r.theme;
    if (typeof r.accent === "string" && HEX.test(r.accent)) p.accent = r.accent.toLowerCase();
    if (p.style || p.theme || p.accent) out[id] = p;
  }
  return out;
}

/** Má záložka vlastní vzhled? */
export const pageHasLook = (a: Pick<Appearance, "pages">, id: string): boolean => !!a.pages[id];

/** Vzhled platný na záložce `pageId`: celkový s přepisem záložky (bez přepisu vrací `a` samo). */
export function effectiveAppearance(a: Appearance, pageId: string): Appearance {
  const o = pageId ? a.pages[pageId] : undefined;
  if (!o) return a;
  let out = a;
  if (o.style && o.style !== a.style) {
    const p = styleOf(o.style).parts;
    out = {
      ...out, style: styleOf(o.style).id, theme: p.theme, customTheme: "", accent: "", density: p.density, fontScale: p.fontScale,
      radius: p.radius, glass: { opacity: p.opacity }, font: p.font, material: p.material, layout: p.layout,
    };
  }
  if (o.theme) out = isBuiltin(o.theme) ? { ...out, theme: o.theme, customTheme: "" } : { ...out, customTheme: o.theme };
  if (o.accent) out = { ...out, accent: o.accent };
  return out;
}

/** Změní pole vzhledu záložky (`undefined` / prázdné = jako celek); záznam bez polí zmizí. */
export function setPageLook(a: Appearance, pageId: string, patch: Partial<Record<keyof PageLook, string | undefined>>): Appearance {
  const cur: PageLook = { ...(a.pages[pageId] ?? {}) };
  for (const [k, v] of Object.entries(patch)) {
    if (v) cur[k as keyof PageLook] = v;
    else delete cur[k as keyof PageLook];
  }
  const pages = { ...a.pages };
  if (cur.style || cur.theme || cur.accent) pages[pageId] = cur;
  else delete pages[pageId];
  return { ...a, pages };
}

/** Odkazy na vlastní motiv `id` pryč (po jeho smazání): výběr celku i záložek se vrátí na „jako celek“ / vestavěný. */
export function dropThemeRefs(a: Appearance, id: string): Appearance {
  let out: Appearance = { ...a, customTheme: a.customTheme === id ? "" : a.customTheme };
  for (const [pid, p] of Object.entries(a.pages)) if (p.theme === id) out = setPageLook(out, pid, { theme: undefined });
  return out;
}
