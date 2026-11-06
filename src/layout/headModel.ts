// Hlavička šuplíku jako čistá data (kolo 27, testuje `headModel.test.mjs`): které díly a v jakém pořadí,
// doplnění configu (`appearance.head` / `appearance.tabs`, zrcadlo Rust `appearance::head`), řazení štítků do
// jedné řady („+N“) a ručičky analogových hodin. Žádné DOM, žádné časovače.
import type { ChipStyle, ClockStyle, HeadLook, HeadPart, TabsLook, TabsStyle } from "../types.ts";

export const DEFAULT_ORDER: readonly HeadPart[] = ["clock", "chips", "tabs", "edit", "note", "search", "settings", "close"];
/** Díly, které nejdou vypnout. */
export const LOCKED: readonly HeadPart[] = ["settings", "close"];
export const CLOCK_STYLES: readonly ClockStyle[] = ["big", "line", "seconds", "analog", "time"];
export const CHIP_STYLES: readonly ChipStyle[] = ["auto", "pills", "merged", "dots", "icons"];
export const TABS_STYLES: readonly TabsStyle[] = ["segment", "underline", "pills", "icons", "dots"];

export const DEFAULT_HEAD: HeadLook = { clock: "big", chips: "auto" };

const pick = <T extends string>(v: unknown, ok: readonly T[], d: T): T => (ok.includes(v as T) ? (v as T) : d);
const parts = (v: unknown): HeadPart[] => (Array.isArray(v) ? v.filter((x, i, a): x is HeadPart => DEFAULT_ORDER.includes(x) && a.indexOf(x) === i) : []);

/** Config → platná hlavička; pole se zapisují jen když se liší od výchozího (starý config zůstane beze změny). */
export function normalizeHead(raw: unknown): HeadLook {
  const r = (raw ?? {}) as Record<string, unknown>;
  const order = parts(r.order);
  const hidden = parts(r.hidden).filter((x) => !LOCKED.includes(x));
  return {
    ...(order.length ? { order } : {}),
    ...(hidden.length ? { hidden } : {}),
    clock: pick(r.clock, CLOCK_STYLES, "big"),
    ...(r.showDate === false ? { showDate: false } : {}),
    chips: pick(r.chips, CHIP_STYLES, "auto"),
    ...(r.showZero === true ? { showZero: true } : {}),
  };
}
export const normalizeTabs = (raw: unknown): TabsLook => ({ style: pick((raw as { style?: unknown } | undefined)?.style, TABS_STYLES, "segment") });
export const isDefaultHead = (h: HeadLook) => JSON.stringify(normalizeHead(h)) === JSON.stringify(DEFAULT_HEAD);
export const isDefaultTabs = (t: TabsLook | undefined) => normalizeTabs(t).style === "segment";

/** Všech osm dílů v pořadí z configu (chybějící na konec v dnešním pořadí), `close` vždy poslední. */
export function fullOrder(h: HeadLook | undefined): HeadPart[] {
  const o = parts(h?.order);
  const all = [...o, ...DEFAULT_ORDER.filter((x) => !o.includes(x))];
  return [...all.filter((x) => x !== "close"), "close"];
}
/** Díly, které se kreslí (pořadí + vypnuté pryč; `settings` a `close` zůstanou vždy). */
export function visibleParts(h: HeadLook | undefined): HeadPart[] {
  const off = parts(h?.hidden).filter((x) => !LOCKED.includes(x));
  return fullOrder(h).filter((x) => !off.includes(x));
}
/** Přesun dílu o jedno místo (−1 doleva, +1 doprava) v úplném pořadí; `close` se nehýbe a nic se nepřesune přes něj. */
export function movePart(h: HeadLook | undefined, part: HeadPart, dir: -1 | 1): HeadPart[] {
  const o = fullOrder(h);
  const i = o.indexOf(part);
  const j = i + dir;
  if (part === "close" || i < 0 || j < 0 || o[j] === "close") return o;
  [o[i], o[j]] = [o[j], o[i]];
  return o;
}
/** Zapsat pořadí do hlavičky; shodné s dnešním = bez pole. */
export function withOrder(h: HeadLook, order: HeadPart[]): HeadLook {
  const same = order.length === DEFAULT_ORDER.length && order.every((x, i) => x === DEFAULT_ORDER[i]);
  const { order: _o, ...rest } = h;
  return same ? rest : { ...rest, order };
}
export function withHidden(h: HeadLook, part: HeadPart, hide: boolean): HeadLook {
  if (LOCKED.includes(part)) return h;
  const cur = (h.hidden ?? []).filter((x) => x !== part);
  const next = hide ? [...cur, part] : cur;
  const { hidden: _h, ...rest } = h;
  return next.length ? { ...rest, hidden: next } : rest;
}

/** Úhly ručiček ve stupních od 12:00 (hodinová, minutová, vteřinová). */
export function handAngles(d: Date): { h: number; m: number; s: number } {
  const s = d.getSeconds();
  const m = d.getMinutes() + s / 60;
  return { h: ((d.getHours() % 12) + m / 60) * 30, m: m * 6, s: s * 6 };
}

/** Jednořádkové datum „po 2. 11.“ z krátkého názvu dne. */
export const shortDate = (dow: string, d: Date) => `${dow} ${d.getDate()}. ${d.getMonth() + 1}.`;
/** „20:02:05“. */
export const withSeconds = (hhmm: string, d: Date) => `${hhmm}:${String(d.getSeconds()).padStart(2, "0")}`;

/** Stupeň zhuštění štítků: základní styl a zkrácení textů sloučené pilulky (0 plné, 1 zkrácené, 2 jen čísla s tečkami). */
export type ChipLevel = { style: "pills" | "merged" | "dots" | "icons"; shrink: 0 | 1 | 2 };
/** Stupně od nejpohodlnějšího po nejhustší. Auto zhušťuje postupně; ruční volba se chová jako dřív, jen sloučená pilulka se místo schování zkracuje. */
export function chipLevels(style: ChipStyle): ChipLevel[] {
  const l = (s: ChipLevel["style"], shrink: 0 | 1 | 2 = 0): ChipLevel => ({ style: s, shrink });
  switch (style) {
    case "pills": return [l("pills")];
    case "merged": return [l("merged"), l("merged", 1), l("merged", 2)];
    case "dots": return [l("dots")];
    case "icons": return [l("icons")];
    default: return [l("pills"), l("merged"), l("merged", 1), l("dots"), l("icons")];
  }
}
/** Důležitost štítku (menší = důležitější): Claude čeká > pád > pracuje > nedokončené > neuloženo > běží > ostatní. */
export const CHIP_PRIO = { waiting: 0, crash: 1, working: 2, unfinished: 3, dirty: 4, running: 5, other: 6 } as const;
/** Součet šířek štítků s mezerami. */
export const rowWidth = (widths: readonly number[], gap: number) => widths.reduce((a, w) => a + w, 0) + Math.max(0, widths.length - 1) * gap;
/** První stupeň, ve kterém se vejdou všechny štítky (`measure` vrátí jejich šířky); jinak poslední a `fits=false`. */
export function pickLevel(levels: readonly ChipLevel[], measure: (l: ChipLevel) => { widths: number[]; gap: number }, avail: number): { level: ChipLevel; fits: boolean } {
  for (const level of levels) {
    const m = measure(level);
    if (rowWidth(m.widths, m.gap) <= avail) return { level, fits: true };
  }
  return { level: levels[levels.length - 1], fits: false };
}
/** Které štítky (indexy) schovat pod „+N“: nejdřív nejméně důležité, dokud se zbytek i s tlačítkem „+N“ nevejde; nejdůležitější zůstane nejdéle. */
export function hideByPriority(widths: readonly number[], prios: readonly number[], gap: number, avail: number, moreW: number): Set<number> {
  const hidden = new Set<number>();
  const order = widths.map((_, i) => i).sort((a, b) => prios[b] - prios[a] || b - a);
  const shown = () => widths.filter((_, i) => !hidden.has(i));
  if (rowWidth(widths, gap) <= avail) return hidden;
  for (const i of order) {
    hidden.add(i);
    const w = shown();
    if (rowWidth(w, gap) + (w.length ? gap : 0) + moreW <= avail) break;
  }
  return hidden;
}
