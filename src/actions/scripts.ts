// Most k Rustu pro Script Commands (`commands/scripts.rs`) a stránky Nastavení Windows: seznam skriptů
// (sken při otevření palety / panelu), spuštění přes stejný tok jako akce (`runAction`: potvrzení,
// „běží“, výsledek do hlášky / výstupu), „Otevřít složku skriptů“ a otevření stránky nastavení.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { ActionOutcome, ActionView, ScriptsListing } from "../types";
import { runAction } from "./api";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const scriptsApi = {
  list: () => call<ScriptsListing>("scripts_list"),
  /** `confirmed` = přesně `confirmText`, který člověk viděl (obsahuje otisk obsahu) */
  run: (id: string, confirmed: string | null) => call<ActionOutcome>("run_script", { id, confirmed }),
  /** založí složku (výslovný klik) a otevře ji; vrací cestu */
  openDir: () => call<string>("scripts_open_dir"),
  /** `ms-settings:…` / `control:Microsoft.…` (jádro ověří tvar) */
  winSettings: (target: string) => call<void>("win_settings_open", { target }),
};

/** Spustí akci nebo skript (dlaždice karty, paleta) — skript přes `run_script`, jinak `run_action`. */
export function runAny(a: ActionView, ask: (a: ActionView) => Promise<boolean>): Promise<boolean> {
  return a.script ? runAction(a, ask, (confirmed) => scriptsApi.run(a.id, confirmed)) : runAction(a, ask);
}
