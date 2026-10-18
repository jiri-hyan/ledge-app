// Lišta režimu úprav nad kartami (výrazně „Táhni za okraj = velikost“, co dál jde dělat + Zrušit / Hotovo) a lišta „… · Vrátit“ po odebrání
// karty, smazání záložky nebo přesunu na jinou záložku (jen v rozpracovaném rozložení).
import { Show } from "solid-js";
import { t } from "../i18n";
import { rich } from "../ui/rich";
import { cancelEdit, finishEdit, runUndo, saving, undoBar } from "./edit";

export default function EditBar() {
  return (
    <>
      <div class="lay-bar" role="toolbar" aria-label={t("cards.layout.bar.aria", "Úpravy šuplíku")} data-anim>
        <span class="lay-bar-text">
          {rich(t("cards.layout.bar.resize", "<b>Táhni za okraj karty = velikost</b> (pravý okraj šířka, spodní výška, roh obojí)."))}{" "}
          {t("cards.layout.bar.hint2", "⠿ přesune jinam nebo na záložku, × odebere. Volné místo v řadě vyplní karty samy. Záložku přejmenuješ dvojklikem.")}
        </span>
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
