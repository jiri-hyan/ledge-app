// Tabulkové testy karty plánu (`plan.ts`): co pošlou tři akce do `decide`.
//   node --experimental-strip-types --test src/sessions/plan.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { addonsMissing, planDecide, planSecondary, planTitle } from "./plan.ts";

test("akce plánu → decide", () => {
  for (const [action, text, want] of [
    ["approve", "", { allow: true }],
    ["approveEdits", "", { allow: true, extra: { mode: "acceptEdits" } }],
    ["inClaude", "", { allow: true, extra: { inClaude: true } }],
    ["return", "  nejdřív testy ", { allow: false, extra: { reason: "nejdřív testy" } }],
    ["return", "   ", null],
    ["return", undefined, null],
  ]) assert.deepEqual(planDecide(action, text), want, `${action} ${text}`);
});

test("vedlejší akce podle cesty hooku", () => {
  for (const [via, want] of [
    ["permission", "approveEdits"],
    ["preToolUse", "inClaude"],
    [undefined, "approveEdits"],
  ]) assert.equal(planSecondary({ via }), want, String(via));
});

test("titulek: z plánu, jinak shrnutí", () => {
  assert.equal(planTitle({ plan: { title: "Oprava", markdown: "# Oprava", file: null, cut: false }, summary: "x" }), "Oprava");
  assert.equal(planTitle({ plan: { title: "", markdown: "", file: null, cut: false }, summary: "shrnutí" }), "shrnutí");
  assert.equal(planTitle({ plan: null, summary: "shrnutí" }), "shrnutí");
});

test("doplnit hooky jedním klikem jen při chybějících doplňcích", () => {
  for (const [state, missing, want] of [
    ["partial", ["PreToolUse"], true],
    ["partial", ["PreToolUse(ExitPlanMode)"], true],
    ["partial", ["PreToolUse", "PreToolUse(ExitPlanMode)"], true],
    ["partial", ["Stop", "PreToolUse"], false],
    ["partial", [], false],
    ["none", ["PreToolUse"], false],
    ["full", [], false],
  ]) assert.equal(addonsMissing({ state, missing }), want, `${state} ${missing}`);
});
