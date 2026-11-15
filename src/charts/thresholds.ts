// JEDNA tabulka prahů „zelená → jantar → červená“ pro limity Clauda i zátěž (CPU, RAM, GPU). Zrcadlo Rust
// `looks/capsule.rs` `WARN_PCT` / `FULL_PCT` (hlídá `thresholds.test.mjs`). Kdo potřebuje práh limitu, importuje odsud —
// žádné další „70“, „80“ nebo „90“ u limitů (kolo 34, N1). Přehled míst: docs/codemap/limits-detail.md „Tabulka prahů“.

/** Od tolika % jantar (pozor): karta, prstenec, proužek, hrana, zvýraznění resetu, „jen při aktivitě“. */
export const WARN_PCT = 75;
/** Od tolika % červená (plno): totéž; ostrůvek „limit blízko“ a čas resetu v proužku. */
export const FULL_PCT = 90;

export type Tone = "ok" | "warn" | "bad";

/** Úroveň podle procenta (limit i zátěž): < `WARN_PCT` ok, < `FULL_PCT` warn, jinak bad. */
export const toneOf = (pct: number): Tone => (pct >= FULL_PCT ? "bad" : pct >= WARN_PCT ? "warn" : "ok");
