// Potvrzení přímo v panelu (ne modální okno): otázka, vysvětlení a dvě tlačítka. Esc = Zrušit.
import { Show, type JSX } from "solid-js";
import { t } from "../../i18n";

export function Confirm(p: { when: boolean; text: string; detail?: JSX.Element; yes: string; danger?: boolean; onYes: () => void; onNo: () => void }) {
  return (
    <Show when={p.when}>
      <div
        class="st-confirm"
        classList={{ danger: p.danger }}
        role="alertdialog"
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            p.onNo();
          }
        }}
      >
        <div>
          <strong>{p.text}</strong>
          <Show when={p.detail}><small>{p.detail}</small></Show>
        </div>
        <span class="st-confirm-btns">
          <button class="mini" onClick={() => p.onNo()}>{t("settings.ui.cancel", "Zrušit")}</button>
          <button class="mini" classList={{ ok: !p.danger, danger: p.danger }} ref={(el) => queueMicrotask(() => el.focus())} onClick={() => p.onYes()}>{p.yes}</button>
        </span>
      </div>
    </Show>
  );
}
