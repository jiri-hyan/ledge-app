// Tabulkové testy čisté logiky limitů Claude (`model.ts`): stav okna, odpočet, pruh.
//   node --experimental-strip-types --test src/limits/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { bar, left, level, otherDay, pctText, winState } from "./model.ts";

const NOW = 1_792_250_979_530;
const MIN = 60_000;
const STALE = 15 * MIN;

test("stav okna: čerstvé / staré / po resetu / žádná data", () => {
  const w = (pct, resetsAt) => ({ pct, resetsAt });
  const cases = [
    [w(42, NOW + MIN), NOW - 5 * MIN, { kind: "fresh", pct: 42 }],
    [w(42, null), NOW - STALE, { kind: "fresh", pct: 42 }],
    [w(42, NOW + MIN), NOW - 47 * MIN - 1, { kind: "stale", pct: 42, ageMin: 47 }],
    [w(100, NOW), NOW, { kind: "reset" }],
    [w(100, NOW - 1), NOW - 99 * MIN, { kind: "reset" }],
    [null, NOW, { kind: "none" }],
    [undefined, NOW, { kind: "none" }],
    [w(1, null), null, { kind: "none" }],
  ];
  for (const [win, at, want] of cases) assert.deepEqual(winState(win, at, NOW, STALE), want, JSON.stringify([win, at]));
});

test("odpočet do resetu zaokrouhlený nahoru na minutu", () => {
  const cases = [
    [NOW + 130 * MIN, { d: 0, h: 2, m: 10 }],
    [NOW + 1, { d: 0, h: 0, m: 1 }],
    [NOW + 76 * 60 * MIN + 5 * MIN, { d: 3, h: 4, m: 5 }],
    [NOW, null],
    [NOW - MIN, null],
    [null, null],
  ];
  for (const [r, want] of cases) assert.deepEqual(left(r, NOW), want, String(r));
});

test("pruh, barva a text procent", () => {
  assert.deepEqual([bar(-3), bar(42.5), bar(130)], [0, 42.5, 100]);
  assert.deepEqual([level(0), level(69.9), level(70), level(89.9), level(90), level(100)], ["ok", "ok", "warn", "warn", "bad", "bad"]);
  assert.deepEqual([pctText(99.6), pctText(0), pctText(23.5)], ["99", "0", "23"]);
  assert.equal(otherDay(NOW, NOW + MIN), false);
  assert.equal(otherDay(NOW + 3 * 24 * 60 * MIN, NOW), true);
});
