// Široký notch (rozložení `notch`, kolo 22): čistá logika lišty sekcí nad kartami záložky — které sekce
// lišta ukáže (jen z karet, které záložka má, a jen s daty), které karty tím převezme (pod lištou se pak
// nekreslí) a pás dnů aktuálního týdne. Bez DOM a bez importů za běhu (`model.test.mjs`).

export type NotchSection = "player" | "days" | "system" | "volume";

/** Pořadí sekcí zleva (obr. „široký notch“): přehrávač | dny a čas | systém | hlasitost. */
export const SECTION_ORDER: readonly NotchSection[] = ["player", "days", "system", "volume"];

/** Karta záložky → sekce lišty, které z ní vzniknou. Kalendář, Dnes a Týden zůstávají kartami (mají události),
 *  Rychlé přepínače taky (mikrofon, výstup). */
export const NOTCH_OF: Readonly<Record<string, readonly NotchSection[]>> = {
  media: ["player", "volume"],
  clock: ["days"],
  bigclock: ["days"],
  system: ["system"],
};

/** Co je k dispozici: právě hraje / hlasitost (snapshot, zapnuté části karty). Systém se čte až v sekci. */
export type NotchData = { media: boolean; volume: boolean };

/** Sekce lišty pro karty záložky (jen s daty — bez skladby žádný přehrávač, bez zařízení žádná hlasitost). */
export function notchSections(cards: readonly string[], data: NotchData): NotchSection[] {
  const want = new Set(cards.flatMap((c) => NOTCH_OF[c] ?? []));
  const ok: Record<NotchSection, boolean> = { player: data.media, volume: data.volume, days: true, system: true };
  return SECTION_ORDER.filter((s) => want.has(s) && ok[s]);
}

/** Karty, které lišta převzala (aspoň jedna jejich sekce je vidět) — pod lištou se nekreslí znovu. */
export function takenCards(cards: readonly string[], sections: readonly NotchSection[]): string[] {
  return cards.filter((c) => (NOTCH_OF[c] ?? []).some((s) => sections.includes(s)));
}

export type WeekDay = { date: number; dow: number; today: boolean; weekend: boolean };

/** Pás dnů týdne s dneškem (pondělí–neděle; `dow` 0 = neděle jako `Date.getDay()`). */
export function weekStrip(now: Date): WeekDay[] {
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const fromMonday = (today.getDay() + 6) % 7;
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - fromMonday + i);
    return { date: d.getDate(), dow: d.getDay(), today: i === fromMonday, weekend: d.getDay() === 0 || d.getDay() === 6 };
  });
}

/** Hlasitost 0–100 z posuvníku (svislý vstup dává text), mimo rozsah ořízne, nesmysl = null. */
export function volumeOf(v: unknown): number | null {
  const n = typeof v === "number" ? v : typeof v === "string" && v.trim() !== "" ? Number(v) : NaN;
  return Number.isFinite(n) ? Math.round(Math.min(100, Math.max(0, n))) : null;
}
