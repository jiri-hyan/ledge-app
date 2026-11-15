// Enter v potvrzení spuštění akce v paletě (L239): potvrdit smí až záměrný stisk — ne auto-repeat při držení
// Enteru a ne dvojitý Enter hned po zobrazení potvrzení (člověk by nestihl přečíst, co spouští).
export const CONFIRM_MIN_MS = 400;

/** Smí Enter odpovědět „Spustit“? `shownAt` = kdy se potvrzení ukázalo, `repeat` = `KeyboardEvent.repeat`. */
export function confirmEnterOk(shownAt: number, now: number, repeat: boolean, minMs = CONFIRM_MIN_MS): boolean {
  return !repeat && now - shownAt >= minMs;
}
