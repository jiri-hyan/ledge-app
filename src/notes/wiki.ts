// Vazby `[[název poznámky]]` při psaní (editor i lístek) — čistá logika našeptávače (testy `wiki.test.mjs`):
// je kurzor uvnitř rozepsaného `[[…` (`wikiQuery`) a jak vybraný titulek vložit (`applyWiki`).

/** Rozepsané `[[…` před kurzorem: pozice `[[` a text za ní; jinak null (zavřené `]]`, konec řádku, závorka). */
export function wikiQuery(text: string, caret: number): { start: number; query: string } | null {
  const before = text.slice(0, Math.max(0, caret));
  const start = before.lastIndexOf("[[");
  if (start < 0) return null;
  const query = before.slice(start + 2);
  if (/[[\]\n\r|]/.test(query) || query.length > 80) return null;
  return { start, query };
}

/** Vloží `[[titulek]]` místo rozepsaného `[[dotaz` (i s už napsaným `]]` za kurzorem); kurzor za vazbu. */
export function applyWiki(text: string, start: number, caret: number, title: string): { text: string; caret: number } {
  const clean = title.replace(/[[\]\r\n|]/g, " ").replace(/\s+/g, " ").trim();
  const after = text.slice(caret).startsWith("]]") ? text.slice(caret + 2) : text.slice(caret);
  const link = `[[${clean}]]`;
  return { text: text.slice(0, start) + link + after, caret: start + link.length };
}
