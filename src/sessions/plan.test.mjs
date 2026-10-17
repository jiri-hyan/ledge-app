// Tabulkové testy karty plánu (`plan.ts`): co pošlou tři akce do `decide`.
//   node --experimental-strip-types --test src/sessions/plan.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { planDecide, planTitle } from "./plan.ts";

test("tři akce plánu → decide", () => {
  for (const [action, text, want] of [
    ["approve", "", { allow: true }],
    ["approveEdits", "", { allow: true, extra: { mode: "acceptEdits" } }],
    ["return", "  nejdřív testy ", { allow: false, extra: { reason: "nejdřív testy" } }],
    ["return", "   ", null],
    ["return", undefined, null],
  ]) assert.deepEqual(planDecide(action, text), want, `${action} ${text}`);
});

test("titulek: z plánu, jinak shrnutí", () => {
  assert.equal(planTitle({ plan: { title: "Oprava", markdown: "# Oprava", file: null, cut: false }, summary: "x" }), "Oprava");
  assert.equal(planTitle({ plan: { title: "", markdown: "", file: null, cut: false }, summary: "shrnutí" }), "shrnutí");
  assert.equal(planTitle({ plan: null, summary: "shrnutí" }), "shrnutí");
});
