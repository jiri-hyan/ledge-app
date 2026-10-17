// Krátké hlášky dole v šuplíku: `say` / `sayError` odkudkoli, vykresluje je `<Toast />` ve skořápce App.
import { createSignal, Show } from "solid-js";
import { t } from "../i18n";

const [toast, setToast] = createSignal<{ text: string; err: boolean } | null>(null);
let toastTimer: number | undefined;

export function say(text: string, err = false) {
  setToast({ text, err });
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => setToast(null), err ? 4200 : 2400);
}

/** Text chyby pro člověka: chyby jádra jsou už česky (string), `Error` bez prefixu „Error:“. */
export function errorText(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e ?? "");
  return msg.trim() || t("ui.error.generic", "Něco se nepovedlo.");
}

export const sayError = (e: unknown) => say(errorText(e), true);

export function Toast() {
  return <Show when={toast()}>{(m) => <div class={`toast ${m().err ? "err" : ""}`} role={m().err ? "alert" : "status"}>{m().text}</div>}</Show>;
}
