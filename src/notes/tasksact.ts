// Akce nad bodem v záložce Úkoly: odškrtnout a odložit. Obě čtou otisk souboru z `note_read` (TaskRow ho nenese)
// a zapisují přes jádro; změněná mezitím = hláška, nic se nepřepíše. Otevření poznámky na řádku jde přes `store.editLine`.
import { api, type NoteView } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import type { SaveOutcome, TaskRow } from "../types";
import { setEditLine } from "./store";

async function write(r: TaskRow, op: (hash: string) => Promise<SaveOutcome | undefined>, ok: string): Promise<boolean> {
  try {
    const d = await api.noteRead(r.path);
    if (!d) return false;
    const out = await op(d.hash);
    if (out?.outcome === "saved") {
      say(ok);
      return true;
    }
    say(t("notes.tasks.changed", "Poznámka se mezitím změnila — zkus to znovu."), true);
  } catch (e) {
    sayError(e);
  }
  return false;
}

export const checkTask = (r: TaskRow) => write(r, (h) => api.noteToggleTask(r.path, r.line, true, h), t("notes.tasks.checked", "Odškrtnuto."));
/** `iso` = `YYYY-MM-DD` / `YYYY-MM-DD HH:MM`, "" = termín odebrat. */
export const snoozeTask = (r: TaskRow, iso: string) =>
  write(r, (h) => api.noteTaskSnooze(r.path, r.line, iso, h), iso ? t("notes.tasks.snoozed", "Odloženo na {when}.", { when: iso }) : t("notes.tasks.unscheduled", "Termín odebrán."));

/** Otevře poznámku v editoru a skočí na řádek bodu. */
export async function openTask(r: TaskRow, onOpen: (n: NoteView) => void): Promise<void> {
  try {
    const d = await api.noteRead(r.path);
    if (!d) return;
    setEditLine(r.line);
    onOpen(d);
  } catch (e) {
    sayError(e);
  }
}
