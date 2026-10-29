// Vzhledy karty Týden (kolo 25) — seznam ve tvaru registru `layout/designs.ts` (`CARD_DESIGNS.week`),
// PRVNÍ je výchozí (dnešní pás dnů). Čistá logika bez DOM; názvy jsou páry [klíč, čeština].
export type WeekDesign = { id: string; name: [string, string]; line: [string, string] };

export const WEEK_DESIGNS: readonly WeekDesign[] = [
  { id: "strip", name: ["cards.design.week.strip", "Pás dnů"], line: ["cards.design.week.strip.line", "Měsíc, čas, pás Po–Ne a zbytek dneška."] },
  { id: "month", name: ["cards.design.week.month", "Měsíc"], line: ["cards.design.week.month.line", "Měsíční mřížka s tečkami u dnů s událostí."] },
  { id: "tearoff", name: ["cards.design.week.tearoff", "Odtrhávací list"], line: ["cards.design.week.tearoff.line", "Velké číslo dne, den v týdnu a kolik událostí zbývá."] },
  { id: "timeline", name: ["cards.design.week.timeline", "Osa dne"], line: ["cards.design.week.timeline.line", "Dnešek 7–22 h s bloky událostí a ryskou teď."] },
  { id: "dial", name: ["cards.design.week.dial", "Ciferník"], line: ["cards.design.week.dial.line", "24h ciferník s výsečemi událostí a ručičkou."] },
  { id: "agenda", name: ["cards.design.week.agenda", "Program"], line: ["cards.design.week.agenda.line", "Nejbližší události s odpočtem."] },
];

/** Platné id vzhledu karty; neznámé / chybějící = výchozí (první). */
export function weekDesign(id: string | undefined): string {
  return WEEK_DESIGNS.find((d) => d.id === id)?.id ?? WEEK_DESIGNS[0].id;
}
