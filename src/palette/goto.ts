// Skok do Nastavení odjinud (paleta, karty, hlavička): id panelu z `settings/catalog.ts` (`osobni`, `akce`…)
// nebo kotva pole `f-<pole>` (`f-personal.links`, `f-autostart`). Skořápka `Settings.tsx` čte
// `settingsTarget`: přepne panel (u kotvy panel toho pole), posune na řádek, zvýrazní ho a cíl vynuluje.
// Ostatní id (prvek mimo Nastavení) jen posune do pohledu; vždy jde i událost `ledge:settings-panel`.
import { createSignal } from "solid-js";

const [settingsTarget, setSettingsTarget] = createSignal<string | null>(null);
export { settingsTarget, setSettingsTarget };

/** Volat PO přepnutí na stránku Nastavení (`setPage("settings")`). */
export function gotoSettingsPanel(id: string) {
  setSettingsTarget(id);
  window.dispatchEvent(new CustomEvent("ledge:settings-panel", { detail: id }));
  if (id.startsWith("f-")) return;
  // dva snímky: stránka se nejdřív vykreslí
  requestAnimationFrame(() => requestAnimationFrame(() => document.getElementById(id)?.scrollIntoView({ block: "start", behavior: "smooth" })));
}
