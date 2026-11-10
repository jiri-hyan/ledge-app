// Šířka šuplíku: co přebíjí předvolbu „Šířka“ v Nastavení (zrcadlo `appearance/geom.rs::sized_width`).
// Pořadí v Rustu: vlastní šířka z tažení v ✎ (`appearance.size.width`, s `perPage` i šířky záložek)
// > rozložení Notch (pevná šířka) > předvolba `appearance.width` > `drawer.width`.
import type { Appearance, DrawerSize } from "../types";

export type WidthOverride =
  | { kind: "notch" }
  | { kind: "drag"; px: number; perPage: boolean };

/** Co právě přebíjí předvolbu šířky; `null` = předvolba platí. Čistá funkce. */
export function widthOverride(a: Pick<Appearance, "layout" | "size">): WidthOverride | null {
  const s = a.size;
  const own = s ? [s.width, ...(s.perPage ? Object.values(s.pages ?? {}).map((p) => p.width) : [])].filter((w): w is number => !!w && w > 0) : [];
  if (own.length) return { kind: "drag", px: own[0], perPage: !!s?.perPage };
  if (a.layout === "notch") return { kind: "notch" };
  return null;
}

/** `appearance.size` bez vlastní šířky (výšky zůstanou) — tlačítko „Zrušit přebití“. */
export function clearDragWidth(s: DrawerSize | undefined): DrawerSize {
  const out: DrawerSize = { ...(s ?? {}), width: 0 };
  if (s?.pages) out.pages = Object.fromEntries(Object.entries(s.pages).map(([k, p]) => [k, { ...p, width: 0 }]));
  return out;
}
