// Sdílený stav poznámek mezi kartou, stránkou Poznámky a editorem: co se má otevřít (nová / úprava),
// kam se vrátit, vybraná záložka (localStorage, jen pohodlí) a drobné akce nad řádkem.

import { createSignal } from "solid-js";
import { api, type NoteView, type Snapshot } from "../api";
import { say } from "../ui/toast";
import { t } from "../i18n";

/** Předvyplnění nové poznámky (dlaždice projektu, tlačítko na záložce). */
export type NotePreset = { project?: string; remind?: boolean; kind?: string };
export const [notePreset, setNotePreset] = createSignal<NotePreset | null>(null);

/** Upravovaná poznámka (cesta) — null = nová poznámka. */
export const [editPath, setEditPath] = createSignal<string | null>(null);
/** Filtr projektu stránky Poznámky: "*" = všechny, "" = bez projektu, jinak id. */
export const [pageProject, setPageProject] = createSignal("*");
/** Stránka Poznámky ukazuje Archiv (poznámky z `_archiv`) místo aktivních. */
export const [pageArchive, setPageArchive] = createSignal(false);
/** Kam se editor vrátí po uložení / Esc. */
export const [returnTo, setReturnTo] = createSignal<"main" | "notes">("main");

const TAB_KEY = "ledge.notes.tab";
function loadTab(): string {
  try {
    return localStorage.getItem(TAB_KEY) || "all";
  } catch {
    return "all";
  }
}
const [tab, setTabRaw] = createSignal(loadTab());
export { tab };
export function setTab(t: string) {
  setTabRaw(t);
  try {
    localStorage.setItem(TAB_KEY, t);
  } catch {
    /* bez úložiště jen nezapamatováno */
  }
}

export const projName = (s: Snapshot, id: string | null) => (id ? s.projects.find((x) => x.id === id)?.name ?? id : t("notes.general", "Obecné"));

const hhmmOf = (ms: number) => {
  const d = new Date(ms);
  return `${d.getHours()}:${String(d.getMinutes()).padStart(2, "0")}`;
};

/** Štítek termínu: po termínu (jantar) · dnes (modrá) · jindy (šedá). */
export function remindLabel(n: NoteView, now: number): [string, string] | null {
  if (n.remindMs == null || n.done) return null;
  if (n.remindMs <= now) return ["a", t("notes.remind.overdue", "po termínu {time}", { time: hhmmOf(n.remindMs) })];
  const d = new Date(n.remindMs);
  if (d.toDateString() === new Date(now).toDateString()) return ["b", t("notes.remind.today", "dnes {time}", { time: hhmmOf(n.remindMs) })];
  return ["n", t("notes.remind.short", "{d}. {m}. {time}", { d: d.getDate(), m: d.getMonth() + 1, time: hhmmOf(n.remindMs) })];
}

// cesty, u kterých právě běží přepnutí done — druhé kliknutí čeká, ne závodí
const [doneBusy, setDoneBusy] = createSignal<ReadonlySet<string>>(new Set());
export { doneBusy };

export async function setDone(n: NoteView, done: boolean) {
  const path = n.path;
  if (doneBusy().has(path)) return;
  setDoneBusy(new Set([...doneBusy(), path]));
  try {
    await api.noteDone(path, done);
  } catch (e) {
    say(String(e), true);
  } finally {
    const next = new Set(doneBusy());
    next.delete(path);
    setDoneBusy(next);
  }
}

/** Otevře `.md` výchozí aplikací (sekundární akce; šuplík se zavře). */
export async function openExternal(path: string) {
  try {
    await api.noteOpen(path);
  } catch (e) {
    say(String(e), true);
  }
}
