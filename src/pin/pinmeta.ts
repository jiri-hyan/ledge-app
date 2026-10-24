// Barva a štítky z okna lístku (Lístky 2.0): zápis do frontmatteru poznámky přes `note_set_meta` s otiskem
// (jako `note_save`) — změněná jinde = načíst a jednou znovu; rozepsaný text se uloží předem (`flush`).
// Odebrat jde jen štítek z frontmatteru (`frontTags`), `#štítek` v textu se maže v textu.
import type { NoteDetail, SaveOutcome } from "../types";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { notesApi } from "../notes/api";
import { frontTags, withTag } from "../notes/meta";

type Ctx = {
  note: () => NoteDetail | null;
  setNote: (n: NoteDetail) => void;
  flush: () => Promise<boolean>;
  busy: (f: () => Promise<SaveOutcome | undefined>) => Promise<SaveOutcome | undefined>;
  markSaved: () => void;
  load: () => Promise<void>;
};

export function pinMeta(c: Ctx) {
  /** `patch(n)` = co zapsat nad aktuální verzí poznámky; `null` = nic (beze změny). */
  async function meta(patch: (n: NoteDetail) => { color?: string; tags?: string[] } | null): Promise<boolean> {
    if (!(await c.flush())) return false;
    try {
      for (let i = 0; i < 2; i++) {
        const n = c.note();
        const x = n && patch(n);
        if (!n || !x) return false;
        const r = await c.busy(() => notesApi.setMeta(n.path, n.hash, x));
        if (r?.outcome === "saved") {
          c.setNote(r.note);
          c.markSaved();
          return true;
        }
        await c.load();
      }
      say(t("notes.pin.retry", "Poznámka se právě měnila jinde — zkus to znovu."), true);
    } catch (e) {
      sayError(e);
    }
    return false;
  }
  const front = (n: NoteDetail) => frontTags(n.tags, n.body);
  async function addTag(raw: string): Promise<boolean> {
    if (!raw.trim()) return false;
    const n = c.note();
    if (n && !withTag(front(n), raw)) {
      say(t("notes.pin.tagBad", "Štítek smí mít písmena, číslice a - _ / (bez mezer), nebo už u poznámky je."), true);
      return false;
    }
    return meta((n) => {
      const next = withTag(front(n), raw);
      return next ? { tags: next } : null;
    });
  }
  return { meta, front, addTag };
}
