// Povolit / Zamítnout Clauda v paletě (`decide.ts`): jen s čekající žádostí a jen s napsaným slovesem; rozhodne až Enter (`run`).
//   node --experimental-strip-types --test src/palette/decide.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { decideItems, decideWanted } from "./decide.ts";

const pd = (id, o = {}) => ({ id, sessionId: "s1", cwd: "C:\\dev\\app", tool: "Bash", summary: "npm test", plan: null, ...o });
const S = (pending) => ({ pending, sessions: [{ id: "s1", project: "Ledge" }] });
const calls = [];
const decide = (id, allow) => calls.push([id, allow]);

test("bez čekající žádosti se nic neukáže, ani se sloveso", () => {
  assert.deepEqual(decideItems(S([]), "povolit", decide), []);
  assert.deepEqual(decideItems({ pending: undefined, sessions: [] }, "povolit", decide), []);
});

test("bez napsaného slovesa (prázdný dotaz, 1 znak, cizí slovo) se nic neukáže — žádný první řádek k Enteru", () => {
  for (const q of ["", " ", "p", "z", "a", "projekt", "poznamka", "x"]) assert.deepEqual(decideItems(S([pd(1)]), q, decide), [], JSON.stringify(q));
});

test("sloveso (i bez diakritiky, i začátek) vybere směr", () => {
  assert.deepEqual(decideWanted("po"), { allow: true, deny: false });
  assert.deepEqual(decideWanted("Schválit"), { allow: true, deny: false });
  assert.deepEqual(decideWanted("zamít"), { allow: false, deny: true });
  assert.deepEqual(decideWanted("de"), { allow: false, deny: true });
  assert.deepEqual(decideWanted("ap npm"), { allow: true, deny: false });
  assert.deepEqual(decideWanted("re"), { allow: false, deny: true });
});

test("řádek na žádost: název nástroje, projekt, a run volá decide s id a směrem až při Enteru", () => {
  const out = decideItems(S([pd(7), pd(8, { tool: "Edit", summary: "a.ts" })]), "povolit", decide);
  assert.deepEqual(out.map((i) => i.id), ["decide:7:allow", "decide:8:allow"]);
  assert.equal(out[0].title, "Povolit: Bash: npm test");
  assert.match(out[0].sub, /Ledge/);
  assert.equal(calls.length, 0, "nic se nerozhodlo samo");
  out[1].run();
  assert.deepEqual(calls, [[8, true]]);
  calls.length = 0;
  decideItems(S([pd(9)]), "zamit", decide)[0].run();
  assert.deepEqual(calls, [[9, false]]);
});

test("plán: Schválit plán s titulkem; dlouhé shrnutí se ořízne; cwd zdroj projektu", () => {
  const plan = pd(3, { plan: { title: "Refaktor X" } });
  assert.equal(decideItems(S([plan]), "schvalit", decide)[0].title, "Schválit plán: Refaktor X");
  const long = decideItems(S([pd(4, { summary: "x".repeat(300) })]), "po", decide)[0].title;
  assert.ok(long.length < 120 && long.endsWith("…"));
  assert.match(decideItems({ pending: [pd(5, { sessionId: "jina" })], sessions: [] }, "po", decide)[0].sub, /app/);
});
