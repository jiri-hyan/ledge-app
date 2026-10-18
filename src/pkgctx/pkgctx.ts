// Bublina MSIX (Rust `pkgctx.rs`, `commands/pkgctx.rs`): běží Ledge uvnitř cizího balíčku (spuštěný
// z Claude desktop)? Stav se za běhu procesu nemění → načte se jednou a sdílí (Nastavení, hlavička, panel
// aktualizace). „Restartovat mimo“ vrací stejný tvar jako instalace aktualizace (`InstallOutcome`).
import { invoke } from "@tauri-apps/api/core";
import { createSignal } from "solid-js";
import { inTauri, type InstallOutcome } from "../api";

export interface PkgPackage {
  full: string;
  family: string;
  /** „Claude“ — jméno pro text */
  name: string;
  /** false = bez identity balíčku, zjištěno sondou virtualizace */
  identity: boolean;
}

export interface PkgView {
  package: PkgPackage | null;
  /** fyzická složka configu v bublině (jen uvnitř) */
  bubbleConfigDir: string | null;
  configDir: string;
}

const [view, setView] = createSignal<PkgView | null>(null);
let asked = false;

/** Stav bubliny; první volání ho načte (jednou za běh UI), do té doby `null`. */
export function pkgView(): PkgView | null {
  if (!asked && inTauri) {
    asked = true;
    invoke<PkgView>("pkg_context").then(setView, () => {});
  }
  return view();
}

/** Spustí Ledge mimo bublinu a tuhle instanci ukončí (Rust); `confirmed` = odsouhlasené běžící servery. */
export const relaunchOutside = (confirmed: string[]) => (inTauri ? invoke<InstallOutcome>("relaunch_outside", { confirmed }) : Promise.resolve(undefined));
