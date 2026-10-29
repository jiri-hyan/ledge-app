// Barva poznámky z menu (⋯ řádku karty, ⋯ editoru): zápis `color:` do frontmatteru přes `note_set_meta` s otiskem.
// Poznámka se přečte čerstvě (otisk), jinde změněná = jedno opakování; jen jména z palety osmi barev, "" = zpět
// na barvu projektu. Vrací zapsanou poznámku (editor si ji načte), jinak undefined.
import { api } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import type { NoteDetail } from "../types";
import { notesApi } from "./api";
import { NOTE_COLORS } from "./meta";

/** Platné jméno z palety, nebo "" (= bez vlastní barvy); jiné (volný hex, nesmysl) → null = nepsat. */
export function pickable(id: string): string | null {
  return id === "" ? "" : NOTE_COLORS.some((c) => c.id === id) ? id : null;
}

export async function setNoteColor(path: string, id: string): Promise<NoteDetail | undefined> {
  const color = pickable(id);
  if (color === null) return undefined;
  try {
    for (let i = 0; i < 2; i++) {
      const d = await api.noteRead(path);
      if (!d) return undefined;
      const r = await notesApi.setMeta(path, d.hash, { color });
      if (r?.outcome === "saved") return r.note;
    }
    say(t("notes.pin.retry", "Poznámka se právě měnila jinde — zkus to znovu."), true);
  } catch (e) {
    sayError(e);
  }
  return undefined;
}
