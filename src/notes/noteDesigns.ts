// Vzhledy karty Poznámky (kolo 26) — seznam ve tvaru registru `layout/designs.ts` (`CARD_DESIGNS.notes`),
// PRVNÍ je výchozí (dnešní seznam). Čistá logika bez DOM; názvy jsou páry [klíč, čeština]. Kresba je `NotesCard.tsx`.
export type NoteDesign = { id: string; name: [string, string]; line: [string, string] };

export const NOTES_DESIGNS: readonly NoteDesign[] = [
  { id: "list", name: ["cards.design.notes.list", "Seznam"], line: ["cards.design.notes.list.line", "Řádky s kategorií, názvem, projektem a stářím jako dřív."] },
  { id: "compact", name: ["cards.design.notes.compact", "Kompaktní řádky"], line: ["cards.design.notes.compact.line", "Jen název na řádku, nejvíc poznámek najednou."] },
  { id: "cards", name: ["cards.design.notes.cards", "Lístečky"], line: ["cards.design.notes.cards.line", "Mřížka malých lístečků; počet sloupců podle šířky karty."] },
  { id: "bento", name: ["cards.design.notes.bento", "Bento"], line: ["cards.design.notes.bento.line", "Velká připnutá nebo nejnovější poznámka, pod ní šablony a akce."] },
  { id: "kanban", name: ["cards.design.notes.kanban", "Kanban"], line: ["cards.design.notes.kanban.line", "Dva sloupce: otevřené a hotové poznámky."] },
  { id: "timeline", name: ["cards.design.notes.timeline", "Časová osa"], line: ["cards.design.notes.timeline.line", "Poznámky na svislé ose podle data vytvoření."] },
];

export type NotesDesignId = "list" | "compact" | "cards" | "bento" | "kanban" | "timeline";

/** Platné id vzhledu; neznámé / chybějící = výchozí (seznam). */
export const notesDesign = (id: string | undefined): NotesDesignId => (NOTES_DESIGNS.some((d) => d.id === id) ? (id as NotesDesignId) : "list");

/** Starý `look.view` (varianta zobrazení před vzhledy karty) → vzhled; seznam a neznámé nic. */
export const LEGACY_VIEW_DESIGN: Readonly<Record<string, string>> = { tiles: "cards", compact: "compact", bento: "bento" };

/** Počet v hlavičce sloupce Kanbanu: je-li v celku víc, než karta ukazuje (`CARD_ROWS`), „zobrazeno / celkem“ (N3b, kolo 34);
 *  celek neznámý (`null`) nebo shodný = jen zobrazený počet. */
export const kanbanCount = (shown: number, total: number | null | undefined): string => (total != null && total > shown ? `${shown} / ${total}` : String(shown));

/** Počet ve skupině: skupiny se dělí jen nad ořezaným seznamem karty, takže při ořezu je to dolní odhad („3+“). */
export const groupCount = (shown: number, cut: boolean): string => (cut && shown > 0 ? `${shown}+` : String(shown));

/** Rozdělení pro Kanban: otevřené / hotové (pořadí zůstává). */
export function kanbanSplit<T extends { done: boolean }>(items: readonly T[]): { open: T[]; done: T[] } {
  return { open: items.filter((n) => !n.done), done: items.filter((n) => n.done) };
}
