// Čistá logika karty otázky Clauda (`QuestionCard.tsx`): rozpracovaná odpověď na jednu otázku (výběr +
// „Jiné…“), co jde odeslat, klávesy 1–n a zbývající čas. Bez Solidu a IPC — jen data z `types.ts`.
import type { ClaudeQuestion, QuestionReply } from "../types";

/** Rozpracovaná odpověď na jednu otázku. `otherOn` = otevřené pole „Jiné…“. */
export type Draft = { labels: string[]; other: string; otherOn: boolean };

export const emptyDraft = (): Draft => ({ labels: [], other: "", otherOn: false });

/** Klik na možnost: jednovýběr = jen ta (zavře „Jiné…“), vícevýběr = přepnout. */
export function pick(q: ClaudeQuestion, d: Draft, label: string): Draft {
  if (!q.multiSelect) return { ...d, labels: [label], otherOn: false };
  const on = d.labels.includes(label);
  return { ...d, labels: on ? d.labels.filter((l) => l !== label) : [...d.labels, label] };
}

/** „Jiné…“: jednovýběr = místo možností (zruší výběr), vícevýběr = navíc k nim. */
export function toggleOther(q: ClaudeQuestion, d: Draft): Draft {
  const otherOn = !d.otherOn;
  return { ...d, otherOn, labels: !q.multiSelect && otherOn ? [] : d.labels };
}

const otherText = (d: Draft) => (d.otherOn ? d.other.trim() : "");

export const answered = (d: Draft) => d.labels.length > 0 || otherText(d) !== "";

export const toReply = (d: Draft): QuestionReply => ({ labels: d.labels, other: otherText(d) || null });

/** Klávesa → akce na otázce: číslo možnosti, za posledním „Jiné…“; jinak nic. */
export function keyTarget(q: ClaudeQuestion, key: string): { label: string } | "other" | null {
  if (!/^[1-9]$/.test(key)) return null;
  const n = Number(key);
  if (n <= q.options.length) return { label: q.options[n - 1].label };
  return n === q.options.length + 1 ? "other" : null;
}

/** Jedna otázka s jednovýběrem → klik na možnost rovnou odpoví (jako v chatu). */
export const instant = (qs: ClaudeQuestion[]) => qs.length === 1 && !qs[0].multiSelect;

/** Zbývající čas do `expiresAt` lidsky („8 min“, „45 s“); prošlý = „“. */
export function remaining(expiresAt: number, now: number): string {
  const s = Math.floor((expiresAt - now) / 1000);
  if (s <= 0) return "";
  return s < 90 ? `${s} s` : `${Math.round(s / 60)} min`;
}

/** Poslat jádru „zobrazeno“ (`question_seen`, pojistka `claude.questionUnseenS`)? Jen držená otázka, šuplík
 *  otevřený a WebView viditelné (`media/shown.ts`), karta aspoň částí v záběru a pro to id ještě ne. */
export function shouldReportSeen(s: { answerable: boolean; shown: boolean; inView: boolean; reported: boolean }): boolean {
  return s.answerable && s.shown && s.inView && !s.reported;
}
