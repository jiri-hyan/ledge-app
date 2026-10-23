// Pomůcka testů: ÚPLNÉ rozložení (všechny běžné karty na Práci a Osobní) — původní výchozí rozložení před
// čistým startem (`layout::clean_default`). Testy úprav (přesun, velikost, katalog…) potřebují co upravovat;
// co je výchozí pro nového uživatele, hlídá `model.test.mjs` přes fixture `layout-default.json`.
import { UP_PERSONAL_PROJECTS } from "./model.ts";

const c = (id, size) => ({ id, size });

export function fullLayout() {
  return {
    pages: [
      { id: "prace", name: "", cards: [c("projects", "xl"), c("actions", "xl"), c("shelf", "xl"), c("running", "xl"), c("claude", "m"), c("repos", "m"), c("notes", "xl")] },
      {
        id: "osobni",
        name: "",
        cards: [c("projects", "xl"), c("clock", "s"), c("timer", "s"), c("weather", "s"), c("calendar", "s"), c("media", "m"), c("links", "m"), c("actions", "xl"), c("shelf", "xl"), c("notes", "xl")],
      },
    ],
    hiddenParts: [],
    upgrades: [UP_PERSONAL_PROJECTS],
  };
}
