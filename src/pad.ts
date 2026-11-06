// Dvoumístné číslo s nulou vpředu („07“) — jediná kopie pro čistou logiku (bez importů za běhu, jde z testů přes node).
export const pad2 = (n: number): string => String(n).padStart(2, "0");
