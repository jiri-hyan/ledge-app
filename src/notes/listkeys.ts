// Napojení chytrých seznamů (`lists.ts`) na textarea: Enter bez modifikátorů pokračuje / ukončuje seznam,
// Tab / Shift+Tab vnořuje. Ctrl+Enter, Shift+Enter, výběr textu a psaní přes IME zůstávají ve výchozím chování.

import { onEnter, onTab } from "./lists";

/** Obslouží klávesu v textarea; vrací true, když ji vyřídil seznam (volající pak nic dalšího nedělá).
 *  `commit` dostane nový text — volající ho zapíše do svého signálu a naplánuje uložení jako při `input`. */
export function listKey(e: KeyboardEvent, ta: HTMLTextAreaElement, commit: (text: string) => void): boolean {
  if (e.isComposing || e.ctrlKey || e.altKey || e.metaKey) return false;
  const a = ta.selectionStart;
  const b = ta.selectionEnd;
  if (e.key === "Enter" && !e.shiftKey && a === b) {
    const r = onEnter(ta.value, a);
    if (!r) return false;
    e.preventDefault();
    ta.value = r.text;
    ta.setSelectionRange(r.caret, r.caret);
    commit(r.text);
    return true;
  }
  if (e.key === "Tab") {
    const r = onTab(ta.value, a, b, e.shiftKey);
    if (!r) return false;
    e.preventDefault();
    if (r.text !== ta.value) {
      ta.value = r.text;
      ta.setSelectionRange(r.selStart, r.selEnd);
      commit(r.text);
    }
    return true;
  }
  return false;
}
