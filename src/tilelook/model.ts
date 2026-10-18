// Vzhled jednotlivých zástupců (vlna 8 P) — čistá logika bez DOM a Solid (test `model.test.mjs`):
// zrcadlo `TileLook` z `src-tauri/src/tilelook.rs` (`projects[].look`, `actions[].look`), normalizace,
// předvolby barev, čitelná kresba na podkladu (kontrast ≥ 4.5:1), změna vzhledu v configu a přehled
// „Upravené karty a zástupci“ pro Nastavení (co je upravené + „Obnovit vše“).
// Prázdný / chybějící vzhled = dlaždice přesně jako dřív (UI kreslí původní cestu).
import type { LookIcon, TileLook, TileShape } from "../types.ts";

export type { LookIcon, TileLook, TileShape };
export type Text = readonly [string, string];

/** Tvary ikony; první = výchozí (dnešní zaoblený čtverec). Nejvýš 4 → segment. */
export const SHAPES: readonly TileShape[] = ["rounded", "circle", "square", "glass"];
export const SHAPE_NAMES: Record<TileShape, Text> = {
  rounded: ["cards.tile.shape.rounded", "Zaoblený"],
  circle: ["cards.tile.shape.circle", "Kruh"],
  square: ["cards.tile.shape.square", "Čtverec"],
  glass: ["cards.tile.shape.glass", "Sklo"],
};
/** Předvolby barvy podkladu (zrcadlo `TILE_COLORS` v tilelook.rs; test hlídá shodu). */
export const TILE_COLORS = [
  "#64748b", "#1f2937", "#dc2626", "#ea580c", "#f59e0b", "#16a34a", "#0d9488", "#0891b2", "#2563eb", "#4f46e5", "#7c3aed", "#db2777",
] as const;
export const MAX_LABEL = 40;
export const MAX_EMOJI = 8;
/** Rychlý výběr emoji (libovolné jde napsat do pole). */
export const EMOJI = ["⭐", "🚀", "🎮", "🎵", "🎨", "📚", "💼", "🏠", "🛠️", "🧪", "📈", "💬", "📷", "🎬", "🌐", "🔒", "☕", "🌿", "🐞", "⚡", "📦", "🗂️", "✏️", "❤️"] as const;

export const isHex = (s: unknown): s is string => typeof s === "string" && /^#[0-9a-fA-F]{6}$/.test(s);
const isShape = (x: unknown): x is TileShape => typeof x === "string" && (SHAPES as readonly string[]).includes(x);
export const glyphOk = (g: unknown): g is string => typeof g === "string" && /^[0-9A-Fa-f]{4}$/.test(g) && parseInt(g, 16) >= 0xe000 && parseInt(g, 16) <= 0xf8ff;
export const keyOk = (k: unknown): k is string => typeof k === "string" && /^[0-9a-f]{16}$/.test(k);
const textOk = (s: string, max: number) => [...s].length <= max && !/[\u0000-\u001f\u007f]/.test(s);

function normIcon(raw: unknown): LookIcon | undefined {
  const r = (raw ?? {}) as Record<string, unknown>;
  const v = typeof r.value === "string" ? r.value : "";
  if (r.kind === "emoji" && v.trim() && textOk(v, MAX_EMOJI)) return { kind: "emoji", value: v };
  if (r.kind === "glyph" && glyphOk(v)) return { kind: "glyph", value: v.toUpperCase() };
  if (r.kind === "image" && keyOk(v)) return { kind: "image", value: v };
  return undefined;
}

/** Obranné čtení vzhledu (snapshot, config); neplatná pole pryč, prázdný vzhled = `undefined`. */
export function normalizeTileLook(raw: unknown): TileLook | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const r = raw as Record<string, unknown>;
  const out: TileLook = {};
  const icon = normIcon(r.icon);
  if (icon) out.icon = icon;
  if (isHex(r.color)) out.color = r.color.toLowerCase();
  if (isShape(r.shape) && r.shape !== "rounded") out.shape = r.shape;
  if (typeof r.label === "string" && r.label.trim() && textOk(r.label, MAX_LABEL)) out.label = r.label;
  if (r.hideLabel === true) out.hideLabel = true;
  return Object.keys(out).length ? out : undefined;
}
export const isPlain = (l: TileLook | undefined | null): boolean => !normalizeTileLook(l);
export const sameLook = (a: TileLook | undefined, b: TileLook | undefined) => JSON.stringify(normalizeTileLook(a) ?? null) === JSON.stringify(normalizeTileLook(b) ?? null);

/** Změna vzhledu (`undefined` v patchi = pole pryč). */
export function patchLook(l: TileLook | undefined, patch: Partial<Record<keyof TileLook, unknown>>): TileLook | undefined {
  const next: Record<string, unknown> = { ...(l ?? {}) };
  for (const [k, v] of Object.entries(patch)) {
    if (v === undefined || v === false || v === "") delete next[k];
    else next[k] = v;
  }
  return normalizeTileLook(next);
}

/** Popisek dlaždice: vlastní, jinak název. */
export const tileLabel = (name: string, l: TileLook | undefined): string => l?.label?.trim() || name;

// ---------- barvy ----------

const lin = (c: number) => {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
};
/** Relativní jas `#rrggbb` (WCAG). */
export function lum(hex: string): number {
  const n = parseInt(hex.slice(1), 16);
  return 0.2126 * lin((n >> 16) & 255) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255);
}
export function contrast(a: string, b: string): number {
  const [x, y] = [lum(a), lum(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
/** Kresba (systémová ikona, písmena) na podkladu `hex`: černá (`true`) nebo bílá — co má větší kontrast
 *  (vždy ≥ 4.5:1). Zrcadlo `tilelook::ink_dark`; CSS tokeny `--black` / `--white`. */
export const inkDark = (hex: string): boolean => contrast(hex, "#000000") > contrast(hex, "#ffffff");

// ---------- config ----------

export type LookKind = "project" | "action";
const listOf = (kind: LookKind) => (kind === "project" ? "projects" : "actions");

/** Config se změněným vzhledem jednoho zástupce (prázdný = pole `look` pryč). Vstup zůstane netknutý. */
export function withTileLook<T extends Record<string, any>>(cfg: T, kind: LookKind, id: string, look: TileLook | undefined): T {
  const key = listOf(kind);
  const list = Array.isArray(cfg[key]) ? (cfg[key] as any[]) : [];
  const norm = normalizeTileLook(look);
  return {
    ...cfg,
    [key]: list.map((x) => {
      if (!x || x.id !== id) return x;
      const out = { ...x };
      if (norm) out.look = norm;
      else delete out.look;
      return out;
    }),
  };
}

export type Customized = { kind: LookKind; id: string; name: string };

/** Zástupci s vlastním vzhledem (přehled v Nastavení), v pořadí configu. */
export function customizedTiles(cfg: any): Customized[] {
  const out: Customized[] = [];
  for (const kind of ["project", "action"] as const) {
    for (const x of Array.isArray(cfg?.[listOf(kind)]) ? cfg[listOf(kind)] : []) {
      if (x && typeof x.id === "string" && normalizeTileLook(x.look)) out.push({ kind, id: x.id, name: (kind === "project" ? x.name : x.label) || x.id });
    }
  }
  return out;
}

/** Bez vlastních vzhledů zástupců (Obnovit vše). */
export function resetTileLooks<T extends Record<string, any>>(cfg: T): T {
  let out = cfg;
  for (const c of customizedTiles(cfg)) out = withTileLook(out, c.kind, c.id, undefined);
  return out;
}
