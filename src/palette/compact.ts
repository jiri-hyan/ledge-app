// Kompaktní paleta (`palette.compact`, jako PowerToys Command Palette): po otevření jen pole,
// seznam se rozbalí až při psaní; s prázdným polem ukáže šipka dolů nedávné (bez nich vše), šipka
// nahoru z prvního řádku ho zase sbalí. Čistá logika — `Palette.tsx` jen vykresluje; výšku šuplíku
// přeměří `ResizeObserver` v `App.tsx` sám (seznam zmizí z DOM).
import type { Item, Ranked } from "./search";

/** Co je pod polem: celý seznam (běžná paleta / výsledky dotazu), jen nedávné, nic (jen pole). */
export type Shown = "list" | "recent" | "none";

/** Stav sbalení: `browsing` = uživatel s prázdným polem rozbalil seznam šipkou dolů. */
export type Fold = { compact: boolean; query: string; browsing: boolean };

export function shown(f: Fold, recentCount: number): Shown {
  if (!f.compact || f.query.trim()) return "list";
  if (!f.browsing) return "none";
  return recentCount > 0 ? "recent" : "list";
}

/** Řádky k vykreslení: `ranked` z `rank()` zúžené podle `shown` (nedávné v jejich pořadí z `rank`). */
export function visible<T extends Item>(ranked: Ranked<T>[], what: Shown, recent: ReadonlySet<string>): Ranked<T>[] {
  if (what === "none") return [];
  if (what === "recent") return ranked.filter((r) => recent.has(r.item.id));
  return ranked;
}

/**
 * Šipka v paletě → nový výběr a stav sbalení. `n` = počet právě viditelných řádků.
 * Sbalená kompaktní paleta: ↓ rozbalí (výběr na první řádek), ↑ nic. Rozbalená bez dotazu: ↑ z prvního
 * řádku sbalí. Jinak dokola jako dřív.
 */
export function arrow(key: "ArrowDown" | "ArrowUp", f: Fold, sel: number, n: number): { sel: number; browsing: boolean } {
  const empty = f.compact && !f.query.trim();
  if (empty && !f.browsing) return { sel: 0, browsing: key === "ArrowDown" };
  if (empty && key === "ArrowUp" && sel === 0) return { sel: 0, browsing: false };
  if (!n) return { sel: 0, browsing: f.browsing };
  return { sel: key === "ArrowDown" ? (sel + 1) % n : (sel - 1 + n) % n, browsing: f.browsing };
}
