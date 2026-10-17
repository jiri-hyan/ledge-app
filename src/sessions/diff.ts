// Čistá logika náhledu diffu v kartě schválení (`DiffPreview.tsx`): sbalený řádek, znaménka, okraj s čísly.
// Diff sám počítá Rust (`claude/diff.rs` + `claude/preview.rs`) při příchodu žádosti. Bez Solidu a IPC.
import type { DiffLine, EditPreview } from "../types";

/** „+12 −3“ (U+2212 jako v kartě) */
export const counts = (pv: Pick<EditPreview, "added" | "removed">): string => `+${pv.added} −${pv.removed}`;

/** Co ukáže sbalený řádek: počty (diff), nový soubor, nebo jen hláška bez řádků. */
export function headKind(pv: Pick<EditPreview, "shown">): "counts" | "newFile" | "unread" | "tooBig" | "binary" {
  return pv.shown === "diff" ? "counts" : pv.shown;
}

/** Jde rozbalit (jsou řádky k ukázání)? */
export const expandable = (pv: Pick<EditPreview, "lines">): boolean => pv.lines.length > 0;

/** Znaménko řádku. */
export function sign(kind: DiffLine["kind"]): string {
  return kind === "add" ? "+" : kind === "del" ? "−" : kind === "gap" ? "⋯" : " ";
}

/** Číslo do okraje: odebraný řádek = číslo v původním souboru, jinak v novém; bez čísel prázdné. */
export function gutter(l: DiffLine, numbered: boolean): string {
  if (!numbered || l.kind === "gap") return "";
  const n = l.kind === "del" ? l.old : l.new;
  return n == null ? "" : String(n);
}

/** Šířka okraje ve znacích (nejdelší číslo); 0 = okraj se nekreslí. */
export function gutterWidth(lines: DiffLine[], numbered: boolean): number {
  return numbered ? lines.reduce((w, l) => Math.max(w, gutter(l, true).length), 0) : 0;
}
