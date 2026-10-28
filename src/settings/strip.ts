// Editor proužku v Nastavení (panel Vzhled, `panels/StripSettings.tsx`): čistá logika nad `strip` z configu (zrcadlo
// `stripconf.rs`) — které položky jde skrýt „v klidu“, změna barvy a „v klidu skryté“ jednoho pole,
// náhled (ukázkové texty, ne skutečná data — v UI označené jako ukázka). Bez importů za běhu (`strip.test.mjs`).

/** Všechny položky v pořadí editoru (Rust `StripItem`, camelCase). */
export const STRIP_ITEM_IDS = ["mode", "clock", "running", "claude", "repos", "limits", "waiting", "custom", "timer", "media", "github", "system", "mail"] as const;
export type StripItemId = (typeof STRIP_ITEM_IDS)[number];

/** Výchozí krytí klasického proužku (`strip.opacity`; zrcadlo `Default` ve `stripconf.rs`, hlídá strip.test.mjs). */
export const STRIP_OPACITY = 0.82;

/** Položky, které se v klidu ukazují samy — volba „jen při dění“ má smysl jen u nich (Rust `is_event` = false
 *  nebo limity pod 70 %). Ostatní se ukážou jen, když mají co hlásit. */
export const QUIETABLE: readonly string[] = ["mode", "clock", "custom", "limits", "system"];

/** Předvolby barvy (Rust `StripColor`) a CSS proměnná pro náhled. */
export const STRIP_COLORS = ["accent", "text", "muted", "violet", "blue", "teal", "green", "amber", "red"] as const;
export type StripColorId = (typeof STRIP_COLORS)[number];
export const COLOR_VAR: Record<StripColorId, string> = {
  accent: "var(--accent)",
  text: "var(--t0)",
  muted: "var(--t2)",
  violet: "var(--violet)",
  blue: "var(--blue)",
  teal: "var(--teal)",
  green: "var(--green)",
  amber: "var(--amber)",
  red: "var(--red)",
};

/** Nejvýš tolik znaků textu položky `custom` (Rust `stripconf::CUSTOM_MAX`, Unicode skaláry). */
export const CUSTOM_MAX = 24;
export const customLen = (s: string): number => [...(s ?? "")].length;
/** Oříznutí na `CUSTOM_MAX` znaků (emoji = jeden znak, ne dvě UTF-16 jednotky). */
export const clipCustom = (s: string): string => [...(s ?? "")].slice(0, CUSTOM_MAX).join("");

/** Mapa barev s jednou změnou (`null` = výchozí barva → klíč pryč); vstup netknutý. */
export function withColor(colors: Record<string, string> | undefined, id: string, color: string | null): Record<string, string> {
  const out = { ...(colors ?? {}) };
  if (color) out[id] = color;
  else delete out[id];
  return out;
}

/** Seznam „v klidu skrytých“ s jednou změnou (pořadí zachované, bez duplicit); vstup netknutý. */
export function withQuiet(quiet: string[] | undefined, id: string, on: boolean): string[] {
  const rest = (quiet ?? []).filter((x) => x !== id);
  return on ? [...rest, id] : rest;
}

export type PreviewSeg = { id: string; text: string; color: string | null; hidden: boolean };

/** Náhled proužku: zapnuté položky v pořadí s ukázkovým textem (`sample` = id → text, prázdný = položka se
 *  teď neukáže), barvou z configu a příznakem „v klidu skryté“ (`busy` = náhled „když se něco děje“). */
export function previewSegments(strip: { items?: string[]; colors?: Record<string, string>; quiet?: string[] }, sample: Record<string, string>, busy: boolean): PreviewSeg[] {
  return (strip.items ?? [])
    .filter((id) => (sample[id] ?? "") !== "")
    .map((id) => ({ id, text: sample[id], color: strip.colors?.[id] ?? null, hidden: !busy && (strip.quiet ?? []).includes(id) }));
}
