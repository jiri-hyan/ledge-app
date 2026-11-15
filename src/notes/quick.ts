// Rychlá poznámka (dock / malé okno): čistý plán kláves, bez závislostí (node test). Ukládání konceptu
// je v `quicksave.ts` (`saveDraftNote`). Ověřuje `quick.test.mjs`.

export type QuickKey = "save" | "save-close" | "none";

/** Co udělat s klávesou v rychlé poznámce. Ctrl+Enter = uložit (volající při pinu zůstane, jinak zavře);
 *  Esc = uložit a zavřít (bez textu jen zavřít — volající dostane z `saveDraftNote` null a zavře). */
export function quickKeyPlan(e: { key: string; ctrl: boolean; hasText: boolean }): QuickKey {
  if (e.key === "Escape") return "save-close";
  if (e.key === "Enter" && e.ctrl && e.hasText) return "save";
  return "none";
}

/** Po úspěšném `dictateStart` (Win+H) se ~15 s ignoruje blur okna: panel hlasového psaní může okno deaktivovat a blur by
 *  uprostřed diktátu uložil poznámku a zavřel dok (L139). Esc / Ctrl+Enter fungují dál (klávesy, ne blur). */
export const DICTATE_BLUR_GRACE_MS = 15_000;

/** Co s blurem okna v rychlé poznámce: `skip` = hotovo / ukládá se / diktát běží (nic), `flush` = připnuté (jen koncept na
 *  disk), `commit` = uložit a zavřít. `dictateUntil` = čas (ms), do kdy diktát blur omlouvá (0 = nikdy nezačal). */
export function blurPlan(s: { finished: boolean; busy: boolean; pinned: boolean; dictateUntil: number; now: number }): "skip" | "flush" | "commit" {
  if (s.finished || s.busy || s.now < s.dictateUntil) return "skip";
  return s.pinned ? "flush" : "commit";
}
