// Čistá logika karty Přihrádka (bez DOM): typy z jádra (`commands/shelf.rs`), kdy kartu ukázat,
// práh začátku tahu ven a poznání tahu souborů (HTML5 `dataTransfer.types`).

export interface ShelfItem {
  path: string;
  name: string;
  dir: string;
  /** soubor na disku není (smazaný, přejmenovaný, odpojený disk) — Ledge ho sám nemaže */
  missing: boolean;
  addedMs: number;
}

export interface ShelfList {
  enabled: boolean;
  items: ShelfItem[];
}

/** Formát cest do schránky: prostá cesta / zmínka pro Claude Code (`@C:\…`, s mezerou `@"C:\… …"`). */
export type ClipFormat = "plain" | "claude";

/** Od kolika px pohybu s drženým tlačítkem začne tah ven (menší pohyb = klik). */
export const DRAG_PX = 6;

export const isDrag = (dx: number, dy: number): boolean => dx * dx + dy * dy >= DRAG_PX * DRAG_PX;

/** Karta jen se zapnutou a neprázdnou přihrádkou (prázdná by jen zabírala místo). */
export const cardVisible = (l: ShelfList | undefined): boolean => !!l && l.enabled && l.items.length > 0;

/** Táhnou se soubory zvenku (ne přeřazování karet uvnitř šuplíku)? */
export const carriesFiles = (types: readonly string[] | undefined | null): boolean => !!types && Array.from(types).includes("Files");
