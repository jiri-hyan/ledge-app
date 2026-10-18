// Zpět v režimu úprav (`history.ts`): každá změna rozložení jde vrátit, Obnovit výchozí taky.
//   node --experimental-strip-types --test src/layout/history.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { defaultLayout, moveCard, removeCard, setSize } from "./model.ts";
import { MAX_UNDO, popUndo, pushUndo, resetLayout } from "./history.ts";

test("případ 10: Zpět po každé změně (velikost, přesun, odebrání), nejen po odebrání", () => {
  // dřív: Vrátit jen po odebrání karty / smazání záložky / přesunu na záložku, a jen 6 s
  const a = defaultLayout();
  const b = setSize(a, "prace", "claude", "s");
  const c = moveCard(b, "prace", 4, 0);
  const d = removeCard(c, "prace", "repos").layout;
  let st = [];
  for (const [prev, next] of [[a, b], [b, c], [c, d]]) st = pushUndo(st, prev, next);
  assert.equal(st.length, 3);
  let r = popUndo(st);
  assert.deepEqual(r.layout, c);
  r = popUndo(r.stack);
  assert.deepEqual(r.layout, b);
  r = popUndo(r.stack);
  assert.deepEqual(r.layout, a);
  assert.equal(popUndo(r.stack).layout, null, "prázdné = nic");
  // změna bez efektu se do historie nepíše (Zpět by „nic nedělalo“)
  assert.equal(pushUndo([], a, setSize(a, "prace", "claude", "m")).length, 0);
  // strop
  let big = [];
  for (let i = 0; i < MAX_UNDO + 5; i++) big = pushUndo(big, setSize(a, "prace", "claude", i % 2 ? "s" : "l"), a);
  assert.equal(big.length, MAX_UNDO);
});

test("případ 11: Obnovit výchozí = výchozí rozložení, jednorázové úpravy se nespustí znovu", () => {
  const custom = removeCard(defaultLayout(), "osobni", "projects").layout;
  custom.upgrades = ["personalProjects", "budouciUprava"];
  const r = resetLayout(custom);
  assert.deepEqual(r.pages, defaultLayout().pages);
  assert.deepEqual(r.upgrades, ["personalProjects", "budouciUprava"], "značky zůstanou");
  assert.deepEqual(resetLayout(defaultLayout()), defaultLayout());
});
