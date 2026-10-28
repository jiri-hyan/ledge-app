// Tabulkové testy čisté logiky pruhu kontextu session (`ctx.ts`).
//   node --experimental-strip-types --test src/sessions/ctx.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CTX_HIGH, CTX_SCALE, CTX_WARN, ctxFill, ctxTicks, sameCtx, showRatio, showTip } from "./ctx.ts";

const c = (tokens, first, ratio, level) => ({ tokens, first, ratio, level });

test("výplň pruhu", () => {
  const cases = [[0, 0], [-5, 0], [100_000, 0.25], [CTX_SCALE, 1], [900_000, 1]];
  for (const [t, want] of cases) assert.equal(ctxFill(t), want, String(t));
});

test("rysky 250 k a 350 k leží na stupnici", () => {
  assert.deepEqual(ctxTicks(), [0.625, 0.875]);
  assert.ok(CTX_WARN < CTX_HIGH && CTX_HIGH < CTX_SCALE);
});

test("poměr k začátku jen když je vidět", () => {
  const cases = [[c(1, 1, 1, "ok"), false], [c(1, 1, 1.4, "ok"), false], [c(1, 1, 1.5, "ok"), true], [c(1, 1, 9.1, "ok"), true], [c(1, null, null, "ok"), false]];
  for (const [v, want] of cases) assert.equal(showRatio(v), want, JSON.stringify(v));
});

test("tip o předávce od 250 k", () => {
  for (const [lvl, want] of [["ok", false], ["warn", true], ["high", true]]) assert.equal(showTip(c(1, 1, 1, lvl)), want, lvl);
});

test("stejná data se nepřekreslují", () => {
  assert.equal(sameCtx(null, null), true);
  assert.equal(sameCtx(null, c(1, 1, 1, "ok")), false);
  assert.equal(sameCtx(c(5, 1, 5, "ok"), c(5, 1, 5, "ok")), true);
  assert.equal(sameCtx(c(5, 1, 5, "ok"), c(6, 1, 6, "ok")), false);
});
