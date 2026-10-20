// Lišta „Smazáno · Zpět“ (a Splněno / Archivováno) — vykresluje ji skořápka šuplíku i okno lístku.
import { Show } from "solid-js";
import { t } from "../i18n";
import { notice, runUndo } from "./lifecycle";

/** `pin` = varianta uvnitř okna lístku (styl v `pin/pin.css`), jinak lišta dole v šuplíku. */
export default function UndoBar(p: { pin?: boolean }) {
  return (
    <Show when={notice()}>
      {(n) => (
        <div classList={{ "card-undo": !p.pin, "pin-undo": !!p.pin }} role="status">
          <span>{n().text}</span>
          <button classList={{ mini: !p.pin, "pin-btn": !!p.pin, pri: !!p.pin }} onClick={() => void runUndo()}>{t("notes.life.undo", "Zpět")}</button>
        </div>
      )}
    </Show>
  );
}
