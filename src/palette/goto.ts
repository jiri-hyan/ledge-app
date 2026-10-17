// Skok na panel Nastavení podle id z `settings/registry.ts` (paleta, karta Rychlé akce). Skořápka
// Nastavení může číst `settingsTarget` (a po použití ho vynulovat); bez ní aspoň posune stránku na
// prvek s tím id a pošle událost `ledge:settings-panel` (detail = id).
import { createSignal } from "solid-js";

const [settingsTarget, setSettingsTarget] = createSignal<string | null>(null);
export { settingsTarget, setSettingsTarget };

/** Volat PO přepnutí na stránku Nastavení (`setPage("settings")`). */
export function gotoSettingsPanel(id: string) {
  setSettingsTarget(id);
  window.dispatchEvent(new CustomEvent("ledge:settings-panel", { detail: id }));
  // dva snímky: stránka se nejdřív vykreslí
  requestAnimationFrame(() => requestAnimationFrame(() => document.getElementById(id)?.scrollIntoView({ block: "start", behavior: "smooth" })));
}
