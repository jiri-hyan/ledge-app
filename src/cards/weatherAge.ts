// Čerstvost počasí — JEDNO pravidlo pro všechny, kdo číslo ukazují (karta Počasí, ostrůvek, lišta notche, hrdina).
// Zrcadlí Rust: `weather::MAX_AGE_MS` (stará) a `looks::facts::WEATHER_SHOW_MS` (proužek číslo po tomhle věku
// schová). Čistá funkce bez importů za běhu (test `weatherAge.test.mjs`).

/** Od tohoto věku (nebo při `status: "stale"`) je počasí „stará data“ — karta to řekne, ostatní ho neukážou / označí. */
export const WX_OLD_MS = 30 * 60_000;
/** Od tohoto věku se číslo neukáže nikde (proužek v Rustu: `WEATHER_SHOW_MS`). */
export const WX_GONE_MS = 3 * 60 * 60_000;

export type WeatherAge = "fresh" | "old" | "gone";

/** Čerstvé (< 30 min a poslední pokus uspěl) · stará data · zmizet (≥ 3 h nebo bez času stažení). */
export function weatherAge(w: { status: string; fetchedMs?: number | null } | null | undefined, now: number): WeatherAge {
  if (!w || w.fetchedMs == null) return "gone";
  const age = now - w.fetchedMs;
  if (age >= WX_GONE_MS) return "gone";
  return w.status === "stale" || age >= WX_OLD_MS ? "old" : "fresh";
}
