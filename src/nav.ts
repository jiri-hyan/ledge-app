// Stránky šuplíku a navigace mezi nimi (řídí skořápka App; karty dostávají jen callbacky).
import type { NoteView } from "./types";

export type Page = "main" | "settings" | "note" | "notes" | "welcome";
export const asPage = (p: string | null | undefined): Page => (p === "settings" || p === "note" || p === "notes" ? p : "main");
/** Stránky, které chtějí klávesnici (vstup jen výslovným klikem → `focusDrawer`). */
export const typing = (p: Page) => p === "note" || p === "notes" || p === "welcome" || p === "settings";

export type NotesNav = { onNew: (kind?: string) => void; onOpen: (n: NoteView) => void; onAll: (general?: boolean) => void };
