// Kde v Nastavení se edituje který klíč configu (kolo 21): cesta (prefix) → stránka a úroveň Základní /
// Pokročilé. JEDINÝ zdroj pro „chyba pole → stránka“ (`model.panelForField`), štítek „Pokročilé“ ve
// výsledcích hledání a guard `placement.test.mjs`: každý klíč configu (z Rust struktur, `scripts/config-keys.mjs`)
// má tady místo, nebo je výslovně na seznamu „nemá editor“ s důvodem (`NO_EDITOR` v `placement.test.mjs` —
// je to vývojářská poznámka, ne text UI). Vyhrává nejdelší prefix (`drawer.glass` dřív než
// `drawer`). Čistá data bez DOM a bez `t()`.

/** Základní = vidět hned; Pokročilé = schované v rozbalovacím „Pokročilé“ (v režimu Základní skryté úplně). */
export type Level = "basic" | "advanced";

/** [prefix cesty, stránka (id panelu z `registry.ts`), úroveň] */
export const PLACEMENT: readonly (readonly [string, string, Level])[] = [
  // Vzhled (i celý proužek a vzhledové klíče sekce `drawer`)
  ["appearance", "vzhled", "basic"],
  ["appearance.layout", "vzhled", "advanced"],
  ["appearance.font", "vzhled", "advanced"],
  ["appearance.fontScale", "vzhled", "advanced"],
  ["appearance.radius", "vzhled", "advanced"],
  ["appearance.material", "vzhled", "advanced"],
  ["appearance.glass", "vzhled", "advanced"],
  ["appearance.strip.style", "vzhled", "advanced"],
  ["appearance.strip.color", "vzhled", "advanced"],
  ["strip", "vzhled", "basic"],
  ["strip.opacity", "vzhled", "advanced"],
  ["timer.strip", "vzhled", "basic"],
  ["drawer.hover", "vzhled", "basic"],
  ["drawer.hoverExpandMs", "vzhled", "advanced"],
  ["drawer.width", "vzhled", "advanced"],
  ["drawer.glass", "vzhled", "advanced"],
  // Šuplík a karty → Záložky a karty (`drawer.sections` a `modes` = starý tvar, edituje ho rozložení)
  ["layout", "suplik", "basic"],
  ["drawer.sections", "suplik", "basic"],
  ["modes", "suplik", "basic"],
  ["shelf", "suplik", "basic"],
  // Šuplík a karty → Otevírání a klávesy
  ["drawer", "chovani", "basic"],
  ["hotkeys", "chovani", "basic"],
  ["palette", "chovani", "basic"],
  ["smart", "chovani", "basic"],
  // Aplikace → Spouštění a hry
  ["autostart", "spousteni", "basic"],
  ["drawer.keepWarmS", "spousteni", "basic"],
  // Šuplík a karty → Obsah karet
  ["personal", "osobni", "basic"],
  ["personal.weather.lat", "osobni", "advanced"],
  ["personal.weather.lon", "osobni", "advanced"],
  ["personal.weather.label", "osobni", "advanced"],
  ["accounts", "osobni", "advanced"],
  ["timer", "osobni", "basic"],
  ["media", "osobni", "basic"],
  ["clipboard", "osobni", "basic"],
  ["suggest", "osobni", "basic"],
  ["github", "osobni", "basic"],
  ["system", "osobni", "basic"],
  // Claude
  ["claude", "claude", "basic"],
  ["claude.port", "claude", "advanced"],
  ["mcp", "claude", "basic"],
  ["notify", "claude", "basic"],
  ["smart.approvalSound", "claude", "basic"],
  ["smart.incidents", "claude", "basic"],
  // Projekty a akce (odkazy = akce; `personal.links` jen nepřevedené)
  ["projects", "akce", "basic"],
  ["roots", "akce", "basic"],
  ["gitAlerts", "akce", "basic"],
  ["actions", "akce", "basic"],
  ["personal.links", "akce", "basic"],
  ["scripts", "akce", "advanced"],
  ["widgets", "akce", "advanced"],
  // Poznámky a lístky
  ["notes", "poznamky", "basic"],
  ["notes.sync.remote", "poznamky", "advanced"],
  ["notes.sync.intervalMin", "poznamky", "advanced"],
  // propojení se složkou (`notes/FolderLinkSection.tsx`): zapnout / složka / směr v Základním, mazání v Pokročilé
  ["notes.folderLink.propagateDeletes", "poznamky", "advanced"],
  // Na ploše
  ["desk", "naplose", "basic"],
  // Aplikace → Jazyk a průvodce · (Spouštění a hry výš) · Záloha a verze
  ["ui", "zaciname", "basic"],
  ["onboarding", "zaciname", "basic"],
  ["update", "aplikace", "basic"],
  ["update.repo", "aplikace", "advanced"],
];

/** Volby z `labels.ts`, které nejsou klíčem configu (tlačítka, souhrnné řádky), a přesto leží v „Pokročilé“.
 *  Vzhled má vlastní seznam (`appearanceModel.ts` `ADVANCED`). */
export const ADVANCED_ANCHORS: readonly string[] = ["backup.file", "about.files", "widgets.gallery"];

/** Je volba z `labels.ts` (id) v „Pokročilé“? */
export const isAdvancedField = (id: string) => ADVANCED_ANCHORS.includes(id) || placeOf(id)?.level === "advanced";

/** Nejdelší prefix z `PLACEMENT` pro cestu (i vnořenou, `projects.2.port`); neznámé = null. */
export function placeOf(path: string | null | undefined): { page: string; level: Level } | null {
  if (!path) return null;
  let best: readonly [string, string, Level] | null = null;
  for (const e of PLACEMENT) {
    if ((path === e[0] || path.startsWith(`${e[0]}.`)) && (!best || e[0].length > best[0].length)) best = e;
  }
  return best ? { page: best[1], level: best[2] } : null;
}
