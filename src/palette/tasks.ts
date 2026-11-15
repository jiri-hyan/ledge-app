// „Odškrtnout úkol“ v paletě: prefix `x:` + slova → otevřené `- [ ]` úkoly z indexu v paměti jádra (`api.noteTasks`, bez čtení
// poznámek). Tady jen čistá část: výběr úkolů, kontrola řádku těla před zápisem a řádky palety — `tasks.test.mjs`. Řádek
// úkolu = řádek TĚLA (od 0); otisk souboru se načte až při odškrtnutí (`api.noteRead`), zápis `noteToggleTask` ho hlídá v jádře.
import { t } from "../i18n.ts";
import { fold } from "./search.ts";
import { parseTail, taskLineOf } from "../notes/taskline.ts";
import type { PItem } from "./items";
import type { TaskRow } from "../types";

export const TASK_PREFIX = "x:";
/** Nejvíc úkolů z jedné poznámky a celkem (paleta není seznam úkolů). */
export const TASKS_PER_NOTE = 5;
export const TASKS_MAX = 12;

/** Text za `x:` (i prázdný); dotaz bez prefixu = `null`. */
export function taskQuery(q: string): string | null {
  const m = /^\s*x:(.*)$/i.exec(q);
  return m ? m[1].trim() : null;
}

/** Otevřené (ne navržené, ne odeslané propojené aplikaci) úkoly odpovídající všem slovům dotazu v textu nebo titulku poznámky. */
export function matchTasks(rows: readonly TaskRow[], query: string): TaskRow[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  return rows.filter((r) => r.state === "open" && !r.forwarded && r.text.trim() && words.every((w) => fold(`${r.text} ${r.noteTitle}`).includes(w)));
}

/** Řádek těla `line` je pořád otevřený úkol téhož textu jako řádek indexu (index může být o krok pozadu; text v indexu je
 *  bez termínu / priority / důkazu a oříznutý na `…`). Jinak se neodškrtává naslepo. */
export function lineStillTask(body: string, line: number, text: string): boolean {
  const m = taskLineOf(body.split("\n")[line] ?? "");
  if (!m || m.state !== "open") return false;
  const now = parseTail(m.rest).text.trim();
  const want = text.endsWith("…") ? text.slice(0, -1) : text;
  return text.endsWith("…") ? now.startsWith(want) : now === want;
}

/** Řádky palety „Odškrtnout: <úkol>“ — nanejvýš `TASKS_PER_NOTE` z poznámky a `TASKS_MAX` celkem. */
export function taskItems(rows: readonly TaskRow[], query: string, toggle: (row: TaskRow) => unknown): PItem[] {
  const out: PItem[] = [];
  const per = new Map<string, number>();
  for (const k of matchTasks(rows, query)) {
    if (out.length >= TASKS_MAX) break;
    const n = per.get(k.path) ?? 0;
    if (n >= TASKS_PER_NOTE) continue;
    per.set(k.path, n + 1);
    out.push({ id: `task:${k.path}:${k.line}`, section: "note", order: 0, glyph: "☐", title: t("palette.task.check", "Odškrtnout: {text}", { text: k.text }), sub: k.noteTitle, run: () => toggle(k) });
  }
  return out;
}

/** Příkaz „Odškrtnout úkol…“: přepne dotaz na `x:` (paleta zůstane otevřená). */
export function taskCommand(open: (query: string) => void): PItem {
  return { id: "app:task", section: "note", order: 4, glyph: "☐", stay: true, title: t("palette.task.open", "Odškrtnout úkol…"), sub: t("palette.task.openSub", "Nebo napiš x: a slovo z úkolu, třeba x:mléko"), keywords: "ukol odskrtnout hotovo task check done todo checklist", run: () => open(TASK_PREFIX) };
}
