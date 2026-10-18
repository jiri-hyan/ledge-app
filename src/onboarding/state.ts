// Kdy se průvodce ukáže: nový uživatel (`onboarding.done === false` v configu) při prvním otevření
// šuplíku, kdokoli ručně z Nastavení (`startWizard`). Config se čte jednou za život WebView — při té
// příležitosti se nastaví i jazyk z `ui.lang`. Skořápka (App) jen sleduje `wizardOpen`.
import { createSignal } from "solid-js";
import { api } from "../api";
import { setLang } from "../i18n";
import { resetWizard } from "./store";
import "./en";

const [open, setOpen] = createSignal(false);
export const wizardOpen = open;
let checked: Promise<void> | null = null;

/** Při otevření šuplíku: jednou přečíst config (jazyk, příznak průvodce). */
export function checkFirstRun(): Promise<void> {
  checked ??= api
    .getConfig()
    .then((cfg) => {
      const lang = cfg?.ui?.lang;
      if (lang === "cs" || lang === "en") setLang(lang);
      if (cfg?.onboarding?.done === false) {
        resetWizard();
        setOpen(true);
      }
    })
    .catch(() => {});
  return checked;
}

/** Ruční spuštění (Nastavení → Začínáme): vždy od začátku. */
export function startWizard() {
  resetWizard();
  setOpen(true);
}

export const closeWizard = () => setOpen(false);
