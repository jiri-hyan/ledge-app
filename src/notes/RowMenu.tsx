// ⋯ na řádku poznámky (karta, stránka Poznámky): Splnit · Archivovat · Smazat (v archivu Vrátit z archivu ·
// Smazat). Seznam je společné menu `ui/Popover.tsx` (pevná pozice u tlačítka — karta s omezenou výškou ho neořízne). Texty akcí jsou společné s editorem a lístkem (`actionText`).

import { For, Show } from "solid-js";
import type { NoteView } from "../api";
import { t } from "../i18n";
import { createMenu, PopItem, PopMenu } from "../ui/Popover";
import { archiveNote, completeNote, deleteNote, reopenNote, unarchiveNote } from "./lifecycle";
import type { LifeAction } from "./lifeactions";
import { lifeActions } from "./lifeactions";
import { actionIcon, actionText } from "./lifelabels";

/** Provede akci nad poznámkou ze šuplíku (lišta Zpět se ukáže sama). */
export function runAction(a: LifeAction, n: NoteView): Promise<boolean> {
  switch (a) {
    case "done": return completeNote(n);
    case "undone": return reopenNote(n);
    case "archive": return archiveNote(n);
    case "unarchive": return unarchiveNote(n);
    case "delete": return deleteNote(n);
  }
}

export default function RowMenu(p: { n: NoteView; archived: boolean }) {
  const menu = createMenu();
  const label = t("notes.life.more", "Splnit, archivovat, smazat…");
  const acts = () => lifeActions(p.n.done, p.archived);
  return (
    <>
      <button class="note-act" classList={{ on: menu.open() }} aria-haspopup="menu" aria-expanded={menu.open()} title={label} aria-label={label} onClick={menu.toggle}>⋯</button>
      <Show when={menu.at()}>
        {(at) => (
          <PopMenu anchor={at().anchor} keyboard={at().keyboard} trigger={menu.trigger()} onClose={menu.close} align="end" label={label}>
            <For each={acts()}>
              {(a) => (
                <PopItem danger={a === "delete"} onClick={() => { menu.close(); void runAction(a, p.n); }}>
                  {actionIcon(a)}{actionText(a)}
                </PopItem>
              )}
            </For>
          </PopMenu>
        )}
      </Show>
    </>
  );
}
