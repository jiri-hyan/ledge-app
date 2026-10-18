// Angličtina pro rychlé odpovědi palety (`src/palette/units.ts`): značky jednotek, které se česky píšou
// jinak. Jména emoji jsou v `en-emoji.ts` (líně s tabulkou). Import s příponou `.ts` — načítají ho i
// node testy (`units.test.mjs`, `quick.test.mjs`).
import { registerDict } from "../i18n.ts";

registerDict("en", {
  "palette.unit.week": "wk",
  "palette.unit.month": "mo",
  "palette.unit.year": "yr",
});
