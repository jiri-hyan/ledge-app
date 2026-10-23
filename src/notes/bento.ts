// Varianta „Bento“ karty Poznámky (rozhodnutí šéfa k bodu 3 zadání týden 43 =
// dlaždice uvnitř karty): velká dlaždice s připnutou / nejnovější otevřenou poznámkou a malé dlaždice akcí
// (Nová .md, šablony, Uložit jako HTML, Otevřít). Čistá logika bez DOM; kresba je `NotesCard.tsx`.
import type { NoteView } from "../types.ts";
import { isPinned, type PinsLike } from "../pin/model.ts";

/** Která poznámka je velká: připnutá na plochu, jinak první nesplněná, jinak první. */
export function heroOf(items: readonly NoteView[], pins: PinsLike): NoteView | null {
  return items.find((n) => isPinned(pins, n.path)) ?? items.find((n) => !n.done) ?? items[0] ?? null;
}

/** Náhled těla: prvních `max` neprázdných řádků, nadpis bez `#`, úkol si nechá značku (`task`). */
export function bentoLines(body: string | null | undefined, max = 4): { text: string; task: boolean }[] {
  return (body ?? "")
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean)
    .slice(0, max)
    .map((l) => ({ text: l.replace(/^#{1,6}\s+/, ""), task: /^[-*+]\s\[[ xX]\]/.test(l) }));
}

/** Šablony nových poznámek (kategorie z výchozích `notes.kinds`; neznámá kategorie projde i tak). */
export const TEMPLATES = [
  { id: "qa", kind: "ukol", name: ["notes.bento.tmplQa", "QA checklist"], body: ["notes.bento.tmplQaBody", "# QA checklist\n- [ ] Spuštění\n- [ ] Rozložení a texty\n- [ ] Ovládání\n- [ ] Chybové stavy a logy"] },
  { id: "bug", kind: "ukol", name: ["notes.bento.tmplBug", "Chyba"], body: ["notes.bento.tmplBugBody", "# Chyba\n**Popis:** \n\n**Kroky:**\n1. \n2. \n\n**Čekal jsem:**\n\n**Stalo se:**"] },
  { id: "meet", kind: "info", name: ["notes.bento.tmplMeet", "Schůzka"], body: ["notes.bento.tmplMeetBody", "# Schůzka\n**Kdo:** \n\n**Body:**\n- \n\n**Úkoly:**\n- [ ] "] },
  { id: "idea", kind: "napad", name: ["notes.bento.tmplIdea", "Nápad"], body: ["notes.bento.tmplIdeaBody", "# Nápad\n**Co:** \n\n**Proč:**\n- \n\n**Jak:**\n- "] },
] as const;
