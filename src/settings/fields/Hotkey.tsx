// Editor zkratky „stiskni kombinaci“: klik → zachytí další stisk (Esc zruší, Backspace/Delete smaže),
// tvar `Ctrl+Alt+N` jako v configu (`model.hotkeyFromEvent`). Kolize hlásí panel Zkratky.
import { createSignal, Show } from "solid-js";
import { t } from "../../i18n";
import { hotkeyFromEvent } from "../model";

export function HotkeyInput(p: { value: string; onChange: (v: string) => void; label: string; conflict?: string | null; disabled?: boolean }) {
  const [rec, setRec] = createSignal(false);
  const key = (e: KeyboardEvent) => {
    if (!rec()) {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        setRec(true);
      }
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") return setRec(false);
    if ((e.key === "Backspace" || e.key === "Delete") && !e.ctrlKey && !e.altKey) {
      p.onChange("");
      return setRec(false);
    }
    const k = hotkeyFromEvent(e);
    if (k) {
      p.onChange(k);
      setRec(false);
    }
  };
  return (
    <span class="st-hotkey-wrap">
      <button
        class="st-hotkey"
        classList={{ rec: rec(), bad: !!p.conflict, unset: !p.value }}
        aria-label={p.label}
        disabled={p.disabled}
        onClick={() => setRec(!rec())}
        onKeyDown={key}
        onBlur={() => setRec(false)}
      >
        <Show when={!rec()} fallback={t("settings.hotkey.rec", "Stiskni kombinaci… (Esc zruší)")}>
          <Show when={p.value} fallback={t("settings.hotkey.none", "Nenastaveno")}>
            {p.value.split("+").map((k) => <kbd>{k}</kbd>)}
          </Show>
        </Show>
      </button>
      <Show when={p.conflict}><span class="st-hotkey-conflict" role="alert">{p.conflict}</span></Show>
    </span>
  );
}
