// Tabulkové testy čisté logiky limitů Claude (`model.ts`): stav okna, odpočet, pruh.
//   node --experimental-strip-types --test src/limits/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { HINT_FROM_PCT, bar, left, level, otherDay, pctText, resetHint, winState } from "./model.ts";

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

test("zvýraznění resetu: jen při 5h od 80 %, čerstvém vzorku a pracující session", () => {
  const w = (pct, resetsAt = NOW + 90 * MIN) => ({ pct, resetsAt });
  const R = NOW + 90 * MIN;
  const cases = [
    ["79 %", w(79), NOW - MIN, true, null],
    ["80 %", w(80), NOW - MIN, true, R],
    ["99,5 %", w(99.5), NOW - MIN, true, R],
    ["nikdo nepracuje", w(95), NOW - MIN, false, null],
    ["staré", w(95), NOW - 20 * MIN, true, null],
    ["po resetu", w(95, NOW - 1), NOW - MIN, true, null],
    ["bez času resetu", w(95, null), NOW - MIN, true, null],
    ["bez okna", null, NOW - MIN, true, null],
    ["bez vzorku", w(95), null, true, null],
  ];
  assert.equal(HINT_FROM_PCT, 80);
  for (const [name, win, at, working, want] of cases) assert.equal(resetHint(win, at, NOW, STALE, working), want, name);
});
