// Šířka šuplíku: co přebíjí předvolbu „Šířka“ v Nastavení (zrcadlo `appearance/geom.rs::sized_width`).
// Pořadí v Rustu: rozložení Notch (pevná šířka, ignoruje i tažení) > vlastní šířka z tažení v ✎
// (`appearance.size.width`, s `perPage` i šířky záložek) > předvolba `appearance.width` > `drawer.width`.
import type { Appearance, DrawerSize } from "../types";

export type WidthOverride =
  | { kind: "notch" }
  | { kind: "drag"; px: number; perPage: boolean };

/** Co právě přebíjí předvolbu šířky; `null` = předvolba platí. Čistá funkce. */
export function widthOverride(a: Pick<Appearance, "layout" | "size">): WidthOverride | null {
  if (a.layout === "notch") return { kind: "notch" };
  const s = a.size;
  if (!s) return null;
  const own = [s.width, ...(s.perPage ? Object.values(s.pages ?? {}).map((p) => p.width) : [])].filter((w): w is number => !!w && w > 0);
  return own.length ? { kind: "drag", px: own[0], perPage: !!s.perPage } : null;
}

/** `appearance.size` bez vlastní šířky (výšky zůstanou) — tlačítko „Zrušit přebití“. */
export function clearDragWidth(s: DrawerSize | undefined): DrawerSize {
  const out: DrawerSize = { ...(s ?? {}), width: 0 };
  if (s?.pages) out.pages = Object.fromEntries(Object.entries(s.pages).map(([k, p]) => [k, { ...p, width: 0 }]));
  return out;
}
