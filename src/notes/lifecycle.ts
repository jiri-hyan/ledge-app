// Akce nad poznámkou — Splnit, Archivovat (i Vrátit z archivu), Smazat (do Koše Windows) — a lišta
// „… · Zpět“ po nich (`UndoBar`). Každá akce vrací, jestli se povedla; chyby jdou do hlášky. Smazání
// je bez dialogu: Zpět stačí (Rust drží odklad `.smazane` o dvě sekundy déle než lišta, pak jde do Koše).
// Modul používá šuplík i okno lístku (každé má vlastní instanci stavu lišty).

import { createSignal } from "solid-js";
import { api, type NoteView } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { shortTitle, UNDO_MS } from "./lifeactions";

/** Poznámka pro akce: stačí cesta a titulek (řádek karty i detail). */
export type Target = Pick<NoteView, "path" | "title">;

type Notice = { id: number; text: string; undo: () => Promise<void>; onUndone?: () => void };
export const [notice, setNotice] = createSignal<Notice | null>(null);
let seq = 0;
let timer: number | undefined;

export type NoticeOpts = {
  /** lišta doběhla bez Zpět (lístek se po ní zavře) */
  onExpire?: () => void;
  /** Zpět se povedlo */
  onUndone?: () => void;
};

/** Ukáže lištu s textem a „Zpět“ na `UNDO_MS`. Nová lišta nahradí starou. */
export function offerUndo(text: string, undo: () => Promise<void>, opts: NoticeOpts = {}) {
  clearTimeout(timer);
  const id = ++seq;
  setNotice({ id, text, undo, onUndone: opts.onUndone });
  timer = window.setTimeout(() => {
    if (notice()?.id !== id) return;
    setNotice(null);
    opts.onExpire?.();
  }, UNDO_MS);
}

export async function runUndo() {
  const n = notice();
  if (!n) return;
  clearTimeout(timer);
  setNotice(null);
  try {
    await n.undo();
    n.onUndone?.();
  } catch (e) {
    sayError(e);
  }
}

const label = (n: Target) => shortTitle(n.title) || t("notes.imageOnly", "(jen obrázek)");

/** Splnit: poznámka se schová pod „i hotové“. */
export async function completeNote(n: Target, opts?: NoticeOpts): Promise<boolean> {
  try {
    await api.noteDone(n.path, true);
  } catch (e) {
    sayError(e);
    return false;
  }
  offerUndo(t("notes.life.done", "Splněno: {title}", { title: label(n) }), () => api.noteDone(n.path, false).then(() => undefined), opts);
  return true;
}

/** Vrátit mezi nehotové (bez lišty — je to vrácení). */
export async function reopenNote(n: Target): Promise<boolean> {
  try {
    await api.noteDone(n.path, false);
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Archivovat: poznámka zmizí z karty i výchozího přehledu, najdeš ji v Archivu. */
export async function archiveNote(n: Target, opts?: NoticeOpts & { keepPin?: boolean }): Promise<boolean> {
  try {
    const moved = await api.noteArchive(n.path, !!opts?.keepPin);
    if (!moved) return false;
    offerUndo(t("notes.life.archived", "Archivováno: {title}", { title: label(n) }), () => api.noteUnarchive(moved).then(() => undefined), opts);
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Vrátit z archivu na původní místo. */
export async function unarchiveNote(n: Target): Promise<boolean> {
  try {
    await api.noteUnarchive(n.path);
    say(t("notes.life.unarchived", "Vráceno z archivu."));
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Smazat: do Koše Windows (jde obnovit), s lištou Zpět. */
export async function deleteNote(n: Target, opts?: NoticeOpts & { keepPin?: boolean }): Promise<boolean> {
  try {
    const d = await api.noteDelete(n.path, !!opts?.keepPin);
    if (!d) return false;
    offerUndo(
      t("notes.life.deleted", "Smazáno: {title}", { title: shortTitle(d.title || n.title) || t("notes.imageOnly", "(jen obrázek)") }),
      async () => {
        await api.noteDeleteUndo(d.id);
        say(t("notes.life.restored", "Poznámka je zpátky."));
      },
      opts,
    );
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}
