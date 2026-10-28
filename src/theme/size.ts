// Velikost otevřeného šuplíku (kolo 23, `appearance.size`) — čistá logika bez DOM: normalizace (zrcadlo
// Rust `appearance::size`), velikost záložky, zápis, ořez, přichycení k mřížce, šířka z tažení (od osy),
// které boky jdou táhnout a kolik karet se nevejde. Stav a tažení: `layout/drawerSize.ts`; tabulka
// v Nastavení: `settings/panels/AppearanceSize.tsx`. Test: `size.test.mjs`.
import type { DrawerSize, PageSize, StripPosition, LayoutId } from "../types.ts";

/** Rozsah šířky = Rust `DRAWER_WIDTH`, výšky = `size::HEIGHT` (1–5 řad karet). */
export const WIDTH_RANGE = [720, 2400] as const;
export const HEIGHT_RANGE = [270, 940] as const;
/** Do jaké vzdálenosti (px) tažení zaskočí na čáru mřížky. */
export const SNAP = 26;
/** Čáry mřížky šířky: po 120 px od 720 (obsahují předvolby 840 / 1080 / 1440 / 1920). */
export const WIDTH_LINES = Array.from({ length: Math.floor((WIDTH_RANGE[1] - WIDTH_RANGE[0]) / 120) + 1 }, (_, i) => WIDTH_RANGE[0] + i * 120);
/** Čáry výšky: 1–5 řad karet (hlavička + řady po 166 px). */
export const HEIGHT_LINES = [1, 2, 3, 4, 5].map((r) => 104 + 166 * r);

const ID = /^[a-z0-9-]{1,24}$/;
const px = (v: unknown, [lo, hi]: readonly [number, number]) =>
  typeof v === "number" && Number.isFinite(v) && v > 0 ? Math.min(hi, Math.max(lo, Math.round(v))) : 0;

/** Ořez na rozsahy; 0 = výchozí zůstává 0. */
export function clampSize(v: PageSize): Required<PageSize> {
  return { width: px(v.width, WIDTH_RANGE), height: px(v.height, HEIGHT_RANGE) };
}

const compact = (v: Required<PageSize>): PageSize => ({ ...(v.width ? { width: v.width } : {}), ...(v.height ? { height: v.height } : {}) });

/** Config → tvar bez nul a neplatných klíčů (jako Rust serde po ořezu). */
export function normalizeSize(raw: any): DrawerSize {
  const r = raw && typeof raw === "object" ? raw : {};
  const pages: Record<string, PageSize> = {};
  for (const [id, v] of Object.entries(r.pages && typeof r.pages === "object" ? r.pages : {})) {
    const c = compact(clampSize((v ?? {}) as PageSize));
    if (ID.test(id) && (c.width || c.height)) pages[id] = c;
  }
  return {
    ...compact(clampSize(r)),
    ...(r.perPage === true ? { perPage: true } : {}),
    ...(Object.keys(pages).length ? { pages } : {}),
  };
}

export const isDefaultSize = (s: DrawerSize | undefined) => !s || (!s.width && !s.height && !s.perPage && !Object.keys(s.pages ?? {}).length);

/** Velikost pro záložku (0 = výchozí) — zrcadlo `SizeConfig::for_page`. */
export function sizeFor(s: DrawerSize | undefined, page: string): Required<PageSize> {
  const own = s?.perPage ? s.pages?.[page] ?? {} : {};
  return { width: own.width || s?.width || 0, height: own.height || s?.height || 0 };
}

/** Zapsat velikost (oříznutou): s `perPage` do záložky, jinak do celku — zrcadlo `SizeConfig::set`. */
export function setSize(s: DrawerSize | undefined, page: string, v: PageSize): DrawerSize {
  const c = compact(clampSize(v));
  const base = { ...(s ?? {}) };
  if (base.perPage) {
    const pages = { ...(base.pages ?? {}) };
    if (c.width || c.height) pages[page] = c;
    else delete pages[page];
    return normalizeSize({ ...base, pages });
  }
  delete base.width;
  delete base.height;
  return normalizeSize({ ...base, ...c });
}

/** Velikosti smazaných záložek pryč (`ids` = id záložek z `layout.pages`); beze změny vrací tentýž objekt. */
export function pruneSizePages(s: DrawerSize | undefined, ids: readonly string[]): DrawerSize | undefined {
  const keys = Object.keys(s?.pages ?? {});
  if (!s || keys.every((k) => ids.includes(k))) return s;
  const pages = Object.fromEntries(Object.entries(s.pages!).filter(([k]) => ids.includes(k)));
  return normalizeSize({ ...s, pages });
}

/** Přepnutí záložky UVNITŘ úprav: velikost nové záložky, když se s `perPage` liší od staré, jinak `null`
 *  (zrcadlo Rust `size::switch_animates`; okno se pak animuje na tuto velikost). */
export function switchedSize(s: DrawerSize | undefined, from: string, to: string): Required<PageSize> | null {
  if (!s?.perPage || !from || from === to) return null;
  const a = sizeFor(s, from);
  const b = sizeFor(s, to);
  return a.width === b.width && a.height === b.height ? null : b;
}

/** Přichycení k nejbližší čáře mřížky do `SNAP` px; `free` (Alt) = jen zaokrouhlit. */
export function snapTo(v: number, lines: readonly number[], free: boolean): number {
  const r = Math.round(v);
  if (free) return r;
  let best = r;
  let d = SNAP + 1;
  for (const l of lines) if (Math.abs(l - v) < d) { d = Math.abs(l - v); best = l; }
  return best;
}

export type Side = "left" | "right";

/** Které boky jdou táhnout: na ose uprostřed oba (šířka symetricky), u hrany jen ten volný; široký notch má pevnou šířku. */
export function widthSides(pos: StripPosition, layout: LayoutId): Side[] {
  if (layout === "notch") return [];
  if (pos === "left") return ["right"];
  if (pos === "right") return ["left"];
  return ["left", "right"];
}

/** Šířka z tažení boku o `dx` px: na středové ose roste na obě strany (2 × posun), u hrany jen o posun. */
export function dragWidth(startW: number, dx: number, side: Side, pos: StripPosition): number {
  const dir = side === "right" ? 1 : -1;
  return startW + dir * dx * (pos === "center" ? 2 : 1);
}

/** Výsledek tažení: ořez na rozsah + přichycení (pořadí: přichytit, pak ořezat). */
export function dragged(w: number, h: number, free: boolean): Required<PageSize> {
  return clampSize({ width: snapTo(w, WIDTH_LINES, free), height: snapTo(h, HEIGHT_LINES, free) });
}

/** Kolik karet je celých pod viditelnou částí (`bottoms` = spodní hrany karet, `viewBottom` = spodek plochy). */
export function overflowCount(bottoms: readonly number[], viewBottom: number): number {
  return bottoms.filter((b) => b > viewBottom + 1).length;
}

/** Strop výšky obsahu jako Rust `drawerfit::limits` (80 % pracovní plochy, nejvýš 20 px od spodku, aspoň 240) —
 *  záloha pro `--drawer-max-h` (`fit.ts`), než přijde přesná hodnota v události `view`. */
export function fallbackMaxH(availH: number): number {
  const a = Number.isFinite(availH) && availH > 0 ? availH : 0;
  return Math.max(Math.floor(Math.min(a * 0.8, a - 20)), 240);
}

/** CSS výška hlavní stránky s pevnou výškou `h` (0 = podle obsahu). Strop jen `--drawer-max-h`, NIKDY `100vh`:
 *  výška obsahu odvozená z výšky okna, kterou Rust nastaví podle výšky obsahu, je zpětná smyčka — v 0.11.0 tak
 *  šuplík zamrzl na 219 px a nepomohl ani restart. */
export function heightCss(h: number): string | undefined {
  return h > 0 ? `min(${h}px, var(--drawer-max-h, ${h}px))` : undefined;
}

/** Výška obsahu pro Rust (`report_size`): `rect` = `getBoundingClientRect().height` (přesná, ale počítá
 *  s transformacemi), `layout` = `offsetHeight` (celé px, transformace ignoruje). Otevírací animace škáluje
 *  obsah (`scale(0.42, 0.28)`, `openAnim.ts`) — měření v tu chvíli dalo ~28 % výšky, okno se na ni zmenšilo
 *  a ResizeObserver na transformaci nereaguje → šuplík zůstal uříznutý (0.11.0/0.11.1, 219 px). */
export function contentHeight(rect: number, layout: number): number {
  return Math.abs(rect - layout) < 1 ? Math.ceil(rect) : layout;
}
