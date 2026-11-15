// Cache obalů (`Player.tsx`): klíč `coverKey` → data URL (až ~5 MB). Vlastní modul, aby ji `App.tsx` mohl při dormantu
// vyprázdnit bez importu celé karty (Player je lazy v chunku karet).
export const covers = new Map<string, Promise<string | null>>();

/** Dormant šuplíku: obaly pryč; příští zobrazení je vyžádá znovu (L170). */
export const dropCovers = () => covers.clear();
