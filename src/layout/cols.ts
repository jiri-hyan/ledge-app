// Kolik sloupců má mřížka karet teď (`--deck-cols` stylu, výchozí 4) — pro ⋯ na kartě, které mřížku nevidí.
import { deckCols } from "./size";

export const colsNow = (): number => deckCols(getComputedStyle(document.querySelector(".deck") ?? document.documentElement).getPropertyValue("--deck-cols"));
