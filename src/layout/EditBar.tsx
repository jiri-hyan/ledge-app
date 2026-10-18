// Lišta režimu úprav nad kartami (co jde dělat + Zrušit / Hotovo) a lišta „… · Vrátit“ po odebrání
// karty, smazání záložky nebo přesunu na jinou záložku (jen v rozpracovaném rozložení).
import { Show } from "solid-js";
import { t } from "../i18n";
import { cancelEdit, finishEdit, runUndo, saving, undoBar } from "./edit";

export default function EditBar() {
  return (
    <>
      <div class="lay-bar" role="toolbar" aria-label={t("cards.layout.bar.aria", "Úpravy šuplíku")} data-anim>
        <span class="lay-bar-text">{t("cards.layout.bar.hint", "Přetáhni ⠿ jinam nebo na záložku, ⇔ změní šířku, × odebere. Záložku přejmenuješ dvojklikem.")}</span>
        <button class="btn" onClick={() => cancelEdit()}>{t("cards.layout.cancel", "Zrušit")}</button>
        <button class="btn pri" disabled={saving()} onClick={() => void finishEdit()}>{t("cards.layout.done", "Hotovo")}</button>
      </div>
      <Show when={undoBar()}>
        {(u) => (
          <div class="card-undo" role="status">
            <span>{u().text}</span>
            <button class="mini" onClick={runUndo}>{t("cards.hide.undo", "Vrátit")}</button>
          </div>
        )}
      </Show>
    </>
  );
}
