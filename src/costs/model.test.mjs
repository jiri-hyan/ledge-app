// Tabulkové testy čisté logiky útraty Claude (`model.ts`): zkrácení tokenů, USD, sloupce, podíl, průběh.
//   node --experimental-strip-types --test src/costs/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { bars, compact, pickRange, progressPct, share, usdDigits } from "./model.ts";

test("zkrácení tokenů", () => {
  const cases = [
    [0, { v: 0, unit: "", digits: 0 }],
    [999, { v: 999, unit: "", digits: 0 }],
    [1_500, { v: 1.5, unit: "k", digits: 1 }],
    [250_000, { v: 250, unit: "k", digits: 0 }],
    [3_400_000, { v: 3.4, unit: "M", digits: 1 }],
    [123_000_000, { v: 123, unit: "M", digits: 0 }],
    [2_000_000_000, { v: 2, unit: "G", digits: 1 }],
  ];
  for (const [n, want] of cases) assert.deepEqual(compact(n), want, String(n));
});

test("desetiny USD", () => {
  assert.equal(usdDigits(0.37), 2);
  assert.equal(usdDigits(99.99), 2);
  assert.equal(usdDigits(100), 0);
});

test("sloupce po dnech", () => {
  const d = (tokens, cost) => ({ date: "2026-10-17", tokens, cost });
  assert.deepEqual(bars([d(10, 1), d(0, null), d(40, 4)]), [0.25, 0, 1], "podle ceny");
  assert.deepEqual(bars([d(10, null), d(40, null)]), [0.25, 1], "bez ceny podle tokenů");
  assert.deepEqual(bars([d(0, null), d(0, null)]), [0, 0], "prázdné okno = nuly, ne NaN");
  assert.deepEqual(bars([]), []);
});

test("podíl řádku", () => {
  const l = (tokens, cost) => ({ name: "x", tokens, cost, unpriced: 0, other: false });
  assert.equal(share(l(1, 5), l(10, 20)), 0.25);
  assert.equal(share(l(5, null), l(10, 20)), 0.5, "řádek bez ceny podle tokenů");
  assert.equal(share(l(5, 1), l(0, 0)), 0);
});

test("průběh a výběr rozsahu", () => {
  assert.equal(progressPct(null), null);
  assert.equal(progressPct(0.999), 99);
  assert.equal(progressPct(1), 100);
  const r = (days) => ({ days, split: {}, total: {}, requests: 0, projects: [], models: [] });
  assert.equal(pickRange([r(1), r(7), r(30)], 7).days, 7);
  assert.equal(pickRange([r(1)], 30).days, 1);
  assert.equal(pickRange([], 7), null);
  assert.equal(pickRange(undefined, 7), null);
});
