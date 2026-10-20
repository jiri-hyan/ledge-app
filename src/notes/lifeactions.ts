// Čistá logika akcí nad poznámkou (Splnit · Archivovat · Smazat): které akce nabídnout a jestli je
// cesta v archivu. Bez DOM a Tauri (test `lifeactions.test.mjs`); používá ji ⋯ řádku, editor i lístek.

export type LifeAction = "done" | "undone" | "archive" | "unarchive" | "delete";

/** Jak dlouho je vidět lišta „… · Zpět“ (ms). Rust drží odklad o dvě sekundy déle (`GRACE`). */
export const UNDO_MS = 8000;

/** Cesta poznámky v archivu (`…\_archiv\x.md`)? */
export const isArchivedPath = (p: string) => /[\\/]_archiv[\\/][^\\/]+$/i.test(p);

/** Akce, které dávají smysl: archivovaná se dá vrátit nebo smazat, aktivní splnit / vrátit mezi nehotové, archivovat, smazat. */
export function lifeActions(done: boolean, archived: boolean): LifeAction[] {
  return archived ? ["unarchive", "delete"] : [done ? "undone" : "done", "archive", "delete"];
}

/** Titulek zkrácený do lišty (jedna řádka, bez zalomení). */
export function shortTitle(title: string, max = 36): string {
  const s = title.replace(/\s+/g, " ").trim();
  return s.length > max ? `${s.slice(0, max - 1).trimEnd()}…` : s;
}
