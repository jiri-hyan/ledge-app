// „Do kalendáře“ (L74): z poznámky vybere událost — připomínku poznámky (`remind:`), jinak první otevřený bod
// s termínem `⏰`. Čisté funkce bez DOM a bez api (test `write.test.mjs`); zápis dělá Rust `calendar/write.rs`
// přes `calendarAdd`. Délka a předstih jsou konstanty (žádná volba v Nastavení).
import { forwardedAt, parseTail, taskLineOf } from "../notes/taskline.ts";

export const CAL_DUR_MIN = 30;
export const CAL_REMIND_MIN = 10;

export type CalSource = "remind" | "task";

export interface CalDraft {
  title: string;
  /** `YYYY-MM-DD` (celodenní) nebo `YYYY-MM-DDTHH:MM` (místní čas) */
  startIso: string;
  allDay: boolean;
  /** semeno stabilního uid: stejný bod = stejná událost i po odložení `⏰` (cesta + zdroj + název, BEZ času;
   *  `.ics` se stejným UID klient aktualizuje, ne zdvojí) */
  uid: string;
  source: CalSource;
}

const LOCAL = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/;
const FENCE = /^\s*(```|~~~)/;

/** První otevřený bod `- [ ]` (i číslovaný) s termínem (mimo bloky kódu), který ještě neodešel do propojené aplikace
 *  (`↳ [` pod bodem = žije jinde; agenda i index ho vynechávají); název = text bez tokenů. */
export function firstDatedTask(body: string): { title: string; due: string; allDay: boolean } | null {
  let fenced = false;
  const lines = body.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (FENCE.test(line)) {
      fenced = !fenced;
      continue;
    }
    const m = fenced ? null : taskLineOf(line);
    if (!m || m.state !== "open" || forwardedAt(lines, i)) continue;
    const t = parseTail(m.rest);
    const title = t.text.trim();
    if (t.due && title) return { title, due: t.due, allDay: t.allDay };
  }
  return null;
}

/** Událost pro „Do kalendáře“, nebo `null` (poznámka nemá `remind` ani bod s `⏰` → položka se nenabídne). */
export function pickCalEvent(path: string, noteTitle: string, body: string, remind: string): CalDraft | null {
  const title = noteTitle.trim();
  if (title && LOCAL.test(remind.trim())) {
    const startIso = remind.trim();
    return { title, startIso, allDay: false, uid: `${path}|remind`, source: "remind" };
  }
  const task = firstDatedTask(body);
  if (!task) return null;
  const startIso = task.allDay ? task.due : task.due.replace(" ", "T");
  return { title: task.title, startIso, allDay: task.allDay, uid: `${path}|task|${task.title}`, source: "task" };
}
