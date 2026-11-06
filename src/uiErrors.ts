// Chyby frontendu do `ledge.log` (příkaz `ui_log`): `window.onerror`, `unhandledrejection` a pády vykreslení
// (`ui/Boundary.tsx`). Žádné časovače ani smyčky — jen dva posluchače událostí. Do logu jde jen text chyby
// (jádro ho ještě zkrátí a zahladí cesty), nikdy obsah poznámek; stejná chyba se hlásí jednou.
import { api, inTauri } from "./api";
import { errorLine, fresh, quietOn } from "./ui/errorline";

export type UiErrorKind = "error" | "promise" | "render";

export function logUiError(kind: UiErrorKind, e: unknown, where?: string) {
  if (!inTauri) return;
  const line = errorLine(e, where);
  if (fresh(line)) void api.uiLog(kind, line).catch(() => {});
}

let installed = false;

/** Zapojí globální posluchače (jednou na okno). */
export function installErrorLog() {
  if (installed || typeof window === "undefined") return;
  installed = true;
  window.addEventListener("error", (ev) => {
    const at = ev.filename ? `${ev.filename.split("/").pop()}:${ev.lineno}` : undefined;
    logUiError("error", ev.error ?? ev.message, at);
  });
  window.addEventListener("unhandledrejection", (ev) => logUiError("promise", ev.reason));
}

/**
 * Selhání příkazu se zapíše do logu a vrátí se `fallback` (prázdný stav) — místo tichého `.catch(() => null)`.
 * `where` = krátký název místa bez cest a obsahu. Nepoužívat tam, kde je selhání běžné a očekávané
 * (pravidelná kontrola, smazaná poznámka): limit logu je 20 za minutu.
 */
export function quiet<T, F = T>(p: Promise<T>, fallback: F, where: string): Promise<T | F> {
  return quietOn((e, w) => logUiError("promise", e, w), p, fallback, where);
}
