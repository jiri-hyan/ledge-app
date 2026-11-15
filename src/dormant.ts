// Skrytý šuplík neudržuje těžký strom: po `DORMANT_AFTER_MS` ve stavu `hidden` se z `.content` odmontuje hlavička,
// karty i ostatní stránky (DOM + styly + dekódované ikony/loga = největší část paměti WebView2 v klidu), zůstane
// jen prázdná skořápka. Při dalším `view` ≠ hidden se strom hned (synchronně, ještě před `animAck("prepared")`)
// namontuje znovu — první otevření po delší pauze je o pár desítek ms pomalejší. Žádný nový periodický časovač:
// `App.tsx` zakládá jen jednorázový `setTimeout` při přechodu do `hidden`, zrušený návratem.
import type { View } from "./api";
import type { Page } from "./nav";

/** Jak dlouho musí být šuplík skrytý, než se strom odmontuje (kompromis: krátké zavření = beze změny, klid = −paměť). */
export const DORMANT_AFTER_MS = 120_000;

/**
 * Má být strom šuplíku odmontovaný? Jen při skrytí, po uplynutí lhůty, na hlavní stránce a mimo režim úprav —
 * rozepsaná poznámka (`note`), Nastavení v šuplíku (`settings`), Poznámky, průvodce i rozpracované úpravy rozložení
 * zůstávají (jejich stav žije v komponentách / `edit.ts`, a stránka se při skrytí stejně vrací na `main`).
 */
export function isDormant(s: { view: View; page: Page; editing: boolean; hiddenLong: boolean }): boolean {
  return s.view === "hidden" && s.hiddenLong && s.page === "main" && !s.editing;
}
