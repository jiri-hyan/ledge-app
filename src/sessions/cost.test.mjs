// Tabulkové testy ceny session (`cost.ts`).
//   node --experimental-strip-types --test src/sessions/cost.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { sessionCost, sessionCostText } from "./cost.ts";

const s = (id, over = {}) => ({ id, tokens: 1000, cost: 1.5, unpriced: 0, requests: 3, firstMs: 1, lastMs: 2, partial: false, ...over });
const f = (v) => `${v} $`;

test("sessionCost: chybějící záznam, cena a seznam = null (prázdný stav, žádné nuly)", () => {
  assert.equal(sessionCost(undefined, "a"), null);
  assert.equal(sessionCost([], "a"), null);
  assert.equal(sessionCost([s("b")], "a"), null);
  assert.equal(sessionCost([s("a", { cost: null, unpriced: 5 })], "a"), null);
  assert.equal(sessionCost([s("a", { cost: Number.NaN })], "a"), null);
});

test("sessionCost + text: přesná částka, dolní odhad při partial / nenaceněných tokenech", () => {
  const rows = [
    [{}, "1.5 $", false],
    [{ partial: true }, "≥ 1.5 $", true],
    [{ unpriced: 7 }, "≥ 1.5 $", true],
    [{ unpriced: -3 }, "1.5 $", false],
    [{ cost: 0 }, "0 $", false],
    [{ cost: -1 }, "0 $", false],
  ];
  for (const [over, want, lower] of rows) {
    const c = sessionCost([s("x"), s("a", over)], "a");
    assert.equal(sessionCostText(c, f), want, JSON.stringify(over));
    assert.equal(c.lower, lower);
  }
});

test("sessionCost: přenáší tokeny, odpovědi a časy pro tooltip", () => {
  const c = sessionCost([s("a", { tokens: 42, requests: 9, firstMs: 10, lastMs: 20 })], "a");
  assert.deepEqual([c.tokens, c.requests, c.firstMs, c.lastMs], [42, 9, 10, 20]);
});
