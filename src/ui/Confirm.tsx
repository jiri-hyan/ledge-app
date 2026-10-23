// Jediné potvrzení nevratné akce (nahrazuje tři vlastní): inline panel v místě akce, ne modální okno — otázka,
// vysvětlení a dvě tlačítka, Esc = Zrušit. Co jde vrátit, se potvrzení nedělá: provede se hned a lišta „Zpět“
// (`notes/UndoBar`, `cards/CardMenu` `HiddenNotice`). Fokus dostane „ano“ jen v okně, které fokus už má
// (Nastavení) — v šuplíku bez fokusu Ledge nic nekrade.
import { Show, type JSX } from "solid-js";
import { t } from "../i18n";
import "./confirm.css";

export function Confirm(p: { when: boolean; text: string; detail?: JSX.Element; yes: string; danger?: boolean; busy?: boolean; onYes: () => void; onNo: () => void }) {
  return (
    <Show when={p.when}>
      <div
        class="ui-confirm"
        classList={{ danger: p.danger }}
        role="alertdialog"
        aria-label={p.text}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            p.onNo();
          }
        }}
      >
        <div class="ui-confirm-text">
          <strong>{p.text}</strong>
          <Show when={p.detail}><small>{p.detail}</small></Show>
        </div>
        <span class="ui-confirm-btns">
          <button class="mini" onClick={() => p.onNo()}>{t("ui.confirm.cancel", "Zrušit")}</button>
          <button class="mini" classList={{ ok: !p.danger, danger: p.danger }} disabled={p.busy} ref={(el) => queueMicrotask(() => document.hasFocus() && el.focus())} onClick={() => p.onYes()}>{p.yes}</button>
        </span>
      </div>
    </Show>
  );
}
