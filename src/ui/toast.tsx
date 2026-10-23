// Krátké hlášky dole v šuplíku: `say` / `sayError` odkudkoli, vykresluje je `<Toast />` ve skořápce App.
import { createSignal, Show } from "solid-js";
import { t } from "../i18n";
import { humanError } from "./errortext";

const [toast, setToast] = createSignal<{ text: string; err: boolean } | null>(null);
let toastTimer: number | undefined;

export function say(text: string, err = false) {
  setToast({ text, err });
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => setToast(null), err ? 4200 : 2400);
}

/** Text chyby pro člověka: chyby jádra jsou už lidské věty (string), `Error` bez prefixu „Error:“;
 * surová technická chyba (`os error`, stacktrace, JSON) se nahradí obecnou větou (`errortext.ts`). */
export function errorText(e: unknown): string {
  return humanError(e, t("ui.error.generic", "Něco se nepovedlo."), t("ui.error.technical", "Něco se nepovedlo — podrobnosti jsou v logu."));
}

export const sayError = (e: unknown) => say(errorText(e), true);

export function Toast() {
  return <Show when={toast()}>{(m) => <div class={`toast ${m().err ? "err" : ""}`} role={m().err ? "alert" : "status"}>{m().text}</div>}</Show>;
}
