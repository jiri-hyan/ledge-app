// Přepínač Nastavení „Základní / Pokročilé“ (kolo 21): v Základním jsou rozbalovací „Pokročilé“ (porty, repa,
// soubory, klíče — `fields/Advanced.tsx`, `panels/apUi.tsx`) skryté úplně; skok z hledání nebo odjinud na
// volbu uvnitř ji ukáže i tak. Jen pohodlí tohoto počítače: úložiště prohlížeče (try/catch), config se nemění.
import { createSignal } from "solid-js";

export type ViewMode = "basic" | "advanced";
export const MODE_KEY = "ledge.settings.mode";

/** Uložená hodnota → režim; cokoli neznámého = Základní. */
export const parseMode = (raw: string | null | undefined): ViewMode => (raw === "advanced" ? "advanced" : "basic");

const read = (): ViewMode => {
  try {
    return parseMode(localStorage.getItem(MODE_KEY));
  } catch {
    return "basic";
  }
};
const [mode, setModeSig] = createSignal<ViewMode>(read());
export { mode };

export function setMode(m: ViewMode) {
  setModeSig(m);
  try {
    localStorage.setItem(MODE_KEY, m);
  } catch {
    // paměť je jen pohodlí
  }
}
