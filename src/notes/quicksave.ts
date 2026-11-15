// Společné jádro ukládání konceptu: jediné místo, kde se koncept (`draft.ts`) mění v poznámku — používá ho
// plný editor (`NoteEditor`) i `QuickNote`. Odděleno od `quick.ts`, ať čistý plán kláves jde testovat v nodu.

import { api } from "../api";
import { errorText, say } from "../ui/toast";
import { t } from "../i18n";
import * as D from "./draft";
import { clearMeta, imagesFor } from "./imgmeta";
import { pinAfterCreate } from "./pinafter";
import { togglePin } from "./pins";

/** Je co uložit? (text, nebo aspoň obrázek ze schránky) */
export const draftHasContent = () => D.draft().trim() !== "" || D.draftImgs().length > 0;

export type Saved = { path: string; /** koncept se vynuloval (mezitím nedopsáno) */ reset: boolean };

/** Uloží koncept jako novou poznámku (`pin` ji rovnou připne na plochu). Prázdný koncept → null (nic se
 *  nevytvoří). Mezitím dopsaný text se nezahazuje: koncept zůstane a `reset` je false. */
export async function saveDraft(pin = false): Promise<Saved | null> {
  if (!draftHasContent()) return null;
  const sent = { ...D.draftMeta(), images: D.draftImgs() };
  const path = await api.noteCreate({ text: sent.text, project: sent.project || null, remind: sent.remind || null, images: imagesFor(sent.images, sent.text), kind: sent.kind || null, title: sent.title.trim() || undefined });
  // poznámka už existuje: selhání připnutí nesmí zrušit reset konceptu (druhé Uložit by ji zduplikovalo)
  const pinErr = await pinAfterCreate(pin, path, togglePin, errorText);
  const same = D.draft() === sent.text && D.draftImgs() === sent.images && D.draftProj() === sent.project && D.draftRemind() === sent.remind;
  if (same) {
    clearMeta();
    D.resetDraft();
  } else say(t("notes.editor.savedKept", "Uloženo — mezitím dopsaný text zůstal v konceptu."));
  if (pinErr) say(pinErr, true);
  return { path: path ?? "", reset: same };
}

/** Zkratka: cesta nové poznámky, nebo null (nebylo co uložit). */
export async function saveDraftNote(): Promise<string | null> {
  return (await saveDraft())?.path ?? null;
}
