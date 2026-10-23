// Poznámky nalezené podle OBSAHU (`note_search`, Ctrl+K): doplní / nahradí řádky poznámek v paletě
// o úryvek z těla. Čistá funkce bez závislostí na jádru — ověřuje ji `notehits.test.mjs`.
import type { NoteView } from "../api";
import type { Nav, PItem } from "./items";

/** Zásah hledání v těle: poznámka + úryvek kolem shody (prázdný, když sedí titulek). */
export type NoteHit = NoteView & { snippet: string };

/**
 * Řádky poznámek ze zásahů: poznámka už v paletě (`note:<cesta>`, z `note_list`) dostane úryvek jako druhý
 * řádek, starší (mimo posledních 200) se přidá. `keywords` = dotaz, ať je řádek nalezen i když dotaz v titulku
 * není (hledání titulku a podtitulku řadí `rank`; zásah jde slaběji než shoda v titulku). Ostatní položky
 * zůstanou beze změny.
 */
export function mergeNoteHits(items: PItem[], hits: NoteHit[], query: string, nav: Nav, untitled: string): PItem[] {
  if (!hits.length) return items;
  const byId = new Map(hits.map((h) => [`note:${h.path}`, h]));
  const seen = new Set<string>();
  const out = items.map((it) => {
    const h = byId.get(it.id);
    if (!h) return it;
    seen.add(it.id);
    return { ...it, sub: h.snippet || it.sub, keywords: `${it.keywords ?? ""} ${query}` };
  });
  for (const [id, h] of byId) {
    if (seen.has(id)) continue;
    out.push({ id, section: "note", order: 5, title: h.title || untitled, sub: h.snippet, keywords: query, run: () => nav.onOpenNote(h) });
  }
  return out;
}
