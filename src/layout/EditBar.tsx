// Lišta režimu úprav nad kartami (výrazně „Táhni za okraj = velikost“, co dál jde dělat + Zpět / Obnovit
// výchozí / Zrušit / Hotovo) a lišta „… · Vrátit“ po odebrání karty, smazání záložky, přesunu na jinou
// záložku nebo po nápovědě k velikosti (jen v rozpracovaném rozložení). Zpět = poslední změna (i Ctrl+Z),
// Obnovit výchozí se ptá dvěma kroky (a jde vrátit Zpět).
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { t } from "../i18n";
import { rich } from "../ui/rich";
import { askDiscard, canUndo, cancelEdit, finishEdit, resetToDefault, resumed, runUndo, saving, setAskDiscard, undoBar, undoLast } from "./edit";

export default function EditBar() {
  const [asking, setAsking] = createSignal(false);
  onMount(() => {
    const key = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === "z") {
        e.preventDefault();
        undoLast();
      }
    };
    window.addEventListener("keydown", key);
    onCleanup(() => window.removeEventListener("keydown", key));
  });
  return (
    <>
      <div class="lay-bar" role="toolbar" aria-label={t("cards.layout.bar.aria", "Úpravy šuplíku")} data-anim>
        <span class="lay-bar-text">
          {rich(t("cards.layout.bar.resize", "<b>Táhni za okraj karty = velikost</b> (pravý okraj šířka, spodní výška, roh obojí)."))}{" "}
          {t("cards.layout.bar.hint3", "Kartu chytíš kdekoli a přetáhneš jinam, na záložku nebo dolů na „Skrýt kartu“ (Esc tah zruší), × ji odebere. Z klávesnice: šipky přesun, + / − šířka, PageUp / PageDown výška.")}
        </span>
        <button class="btn" disabled={!canUndo()} title={t("cards.layout.undoTitle", "Vrátit poslední změnu (Ctrl+Z)")} onClick={undoLast}>{t("cards.layout.undo", "Zpět")}</button>
        <Show when={asking()} fallback={
          <button class="btn" title={t("cards.layout.resetTitle", "Záložky a karty jako po instalaci")} onClick={() => setAsking(true)}>{t("cards.layout.reset", "Obnovit výchozí")}</button>
        }>
          <span class="lay-ask" role="group" aria-label={t("cards.layout.resetAsk", "Opravdu obnovit výchozí rozložení?")}>
            {t("cards.layout.resetAsk", "Opravdu obnovit výchozí rozložení?")}
            <button class="btn" onClick={() => setAsking(false)}>{t("cards.layout.resetNo", "Ne")}</button>
            <button class="btn pri" onClick={() => { setAsking(false); resetToDefault(); }}>{t("cards.layout.resetYes", "Obnovit")}</button>
          </span>
        </Show>
        <button class="btn" onClick={() => cancelEdit()}>{t("cards.layout.cancel", "Zrušit")}</button>
        <button class="btn pri" disabled={saving()} onClick={() => void finishEdit()}>{t("cards.layout.done", "Hotovo")}</button>
      </div>
      <Show when={resumed()}>
        <div class="card-undo" role="status">
          <span>{t("cards.layout.resumed", "Máš neuložené úpravy rozložení — Hotovo uloží, Zrušit zahodí.")}</span>
        </div>
      </Show>
      <Show when={askDiscard()}>
        <div class="card-undo" role="alertdialog" aria-label={t("cards.layout.discardAsk", "Zahodit neuložené úpravy rozložení?")}>
          <span>{t("cards.layout.discardAsk", "Zahodit neuložené úpravy rozložení?")}</span>
          <button class="mini" onClick={() => setAskDiscard(false)}>{t("cards.layout.discardNo", "Pokračovat v úpravách")}</button>
          <button class="mini" onClick={() => cancelEdit()}>{t("cards.layout.discardYes", "Zahodit")}</button>
        </div>
      </Show>
      <Show when={undoBar()}>
        {(u) => (
          <div class="card-undo" role="status">
            <span>{u().text}</span>
            <Show when={u().restore}>
              <button class="mini" onClick={runUndo}>{t("cards.hide.undo", "Vrátit")}</button>
            </Show>
          </div>
        )}
      </Show>
    </>
  );
}
