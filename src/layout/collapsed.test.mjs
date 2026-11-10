// Testy pro sbalené karty na 1 řádek (collapsed.ts)
//   node --experimental-strip-types --test src/layout/collapsed.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  COLLAPSED_KEY,
  cardCollapseKey,
  toggleSet,
  isCardCollapsed,
  toggleCardCollapsed,
  setCardCollapsed,
} from "./collapsed.ts";

test("COLLAPSED_KEY je ledge.collapsedCards.v1", () => {
  assert.equal(COLLAPSED_KEY, "ledge.collapsedCards.v1");
});

test("cardCollapseKey formátuje page:card", () => {
  assert.equal(cardCollapseKey("home", "notes"), "home:notes");
  assert.equal(cardCollapseKey("dev", "projects"), "dev:projects");
});

test("toggleSet přidá klíč pokud chybí, odebere pokud existuje", () => {
  const initial = ["a", "b"];
  const added = toggleSet(initial, "c");
  assert.deepEqual(added, ["a", "b", "c"]);

  const removed = toggleSet(added, "b");
  assert.deepEqual(removed, ["a", "c"]);

  const removedAgain = toggleSet(removed, "c");
  assert.deepEqual(removedAgain, ["a"]);

  const addedBack = toggleSet(removedAgain, "a");
  assert.deepEqual(addedBack, []);
});

test("toggleCardCollapsed a isCardCollapsed reagují správně", () => {
  const p = "testPage";
  const c = "testCard";

  // Ujistíme se, že je výchozí stav false
  setCardCollapsed(p, c, false);
  assert.equal(isCardCollapsed(p, c), false);

  // Přepneme na true
  const res1 = toggleCardCollapsed(p, c);
  assert.equal(res1, true);
  assert.equal(isCardCollapsed(p, c), true);

  // Přepneme zpět na false
  const res2 = toggleCardCollapsed(p, c);
  assert.equal(res2, false);
  assert.equal(isCardCollapsed(p, c), false);

  // Explicitní setCardCollapsed
  setCardCollapsed(p, c, true);
  assert.equal(isCardCollapsed(p, c), true);

  setCardCollapsed(p, c, true); // no-op idempotent
  assert.equal(isCardCollapsed(p, c), true);

  setCardCollapsed(p, c, false);
  assert.equal(isCardCollapsed(p, c), false);
});
