// „Psaní otevře paletu“: tisknutelný znak v otevřeném šuplíku, který nemá fokus v poli, otevře paletu
// s tímto znakem (= „otevřít → psát → Enter“ bez Ctrl+K). Čistá logika bez DOM typů — ověřuje ji `typeto.test.mjs`.

/** Minimum z `EventTarget`, které rozhodnutí potřebuje (v testu stačí prostý objekt). */
export interface KeyTarget {
  tagName?: string;
  isContentEditable?: boolean;
  closest?: (sel: string) => unknown;
}
export interface KeyLike {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  isComposing?: boolean;
}

const FIELDS = ["INPUT", "TEXTAREA", "SELECT"];

/** Cíl, do kterého se píše (pole, výběr, `contenteditable`): zkratky šuplíku (1–9, psaní do palety) mu nesmí vzít klávesy. */
export function isTextTarget(t: KeyTarget | null | undefined): boolean {
  if (!t) return false;
  if (t.tagName && FIELDS.includes(t.tagName.toUpperCase())) return true;
  return !!t.isContentEditable || !!t.closest?.('[contenteditable=""], [contenteditable="true"]');
}

/** Znak, kterým se paleta otevře; `null` = stisk není psaní (modifikátor, speciální klávesa, pole, 1–9 = záložky, mezera). */
export function typeToPalette(e: KeyLike, target: KeyTarget | null | undefined): string | null {
  if (e.ctrlKey || e.altKey || e.metaKey || e.isComposing) return null;
  if ([...e.key].length !== 1 || !e.key.trim()) return null; // „Enter“, „Dead“, „F5“, mezera
  if (e.key >= "1" && e.key <= "9") return null; // záložky podle pořadí
  if (isTextTarget(target)) return null;
  // jen holé tělo: na tlačítku / posuvníku / odkazu klávesy patří jemu
  const tag = target?.tagName?.toUpperCase();
  if (tag && tag !== "BODY" && tag !== "HTML") return null;
  return e.key;
}
