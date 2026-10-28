// Tabulkové testy tempa čerpání limitů (`pace.ts`).
//   node --experimental-strip-types --test src/limits/pace.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { DAY_MS, WEEK_MS, WIN5_MS, eta5h, overSeg, paceLevel, paceOf, paceTip, planPct, rate5h, weekToday } from "./pace.ts";

const NOW = 1_792_250_979_530;
const HOUR = 3_600_000;
const near = (a, b) => assert.ok(Math.abs(a - b) < 1e-6, `${a} ≈ ${b}`);
const tr = (_k, fb, v = {}) => fb.replace(/\{(\w+)\}/g, (_, x) => String(v[x]));
const clock = (ms) => `T+${Math.round((ms - NOW) / 60000)}`;

test("plán týdne: začátek, střed, konec, mimo", () => {
  const R = NOW + WEEK_MS;
  near(planPct(R, WEEK_MS, NOW), 0);
  near(planPct(R, WEEK_MS, NOW + WEEK_MS / 2), 50);
  near(planPct(R, WEEK_MS, NOW + 6 * DAY_MS), 600 / 7);
  near(planPct(R, WEEK_MS, R - 1), 100 - 100 / WEEK_MS);
  assert.equal(planPct(R, WEEK_MS, R), null);
  assert.equal(planPct(null, WEEK_MS, NOW), null);
  assert.equal(planPct(undefined, WIN5_MS, NOW), null);
});

test("úroveň a segment přes plán", () => {
  for (const [d, lv] of [[-3, "ok"], [0, "ok"], [0.5, "ok"], [0.6, "warn"], [5, "warn"], [5.1, "bad"]]) assert.equal(paceLevel(d), lv, String(d));
  assert.equal(overSeg(30, 40), null);
  assert.equal(overSeg(40, 40), null);
  assert.deepEqual(overSeg(75, 71), { from: 71, to: 75, lv: "warn" });
  assert.deepEqual(overSeg(80, 71), { from: 71, to: 80, lv: "bad" });
});

test("dnes zbývá: hranice dne a přečerpání", () => {
  const R = NOW + WEEK_MS;
  near(weekToday(0, R, NOW), 100 / 7);
  near(weekToday(10, R, NOW + DAY_MS - 1), 100 / 7 - 10);
  near(weekToday(10, R, NOW + DAY_MS), 200 / 7 - 10);
  near(weekToday(100, R, R - 1), 0);
  assert.ok(weekToday(20, R, NOW + 1000) < 0);
});

test("tempo 5h: historie 30 min, krátká historie → průměr", () => {
  const R = NOW + 3 * HOUR; // začátek okna NOW − 2 h
  const r = rate5h([[NOW - 20 * 60_000, 30], [NOW, 40]], 40, R, NOW);
  assert.equal(r.src, "recent");
  near(r.perH, 30);
  const short = rate5h([[NOW - 5 * 60_000, 38], [NOW, 40]], 40, R, NOW);
  assert.equal(short.src, "avg");
  near(short.perH, 20);
  assert.equal(rate5h(undefined, 40, R, NOW).src, "avg");
  assert.equal(rate5h([[NOW - 50 * 60_000, 10], [NOW, 40]], 40, R, NOW).src, "avg");
  assert.equal(rate5h([], 40, NOW + WIN5_MS, NOW), null);
});

test("odhad 5h: vydrží / dojde / tempo 0", () => {
  const R = NOW + 3 * HOUR;
  assert.deepEqual(eta5h({ perH: 10, src: "avg" }, 40, R, NOW), { kind: "lasts", until: R });
  const out = eta5h({ perH: 60, src: "recent" }, 40, R, NOW);
  assert.equal(out.kind, "runs-out");
  near(out.at, NOW + HOUR);
  near(out.slowTo, 20);
  assert.equal(eta5h({ perH: 20, src: "avg" }, 40, R, NOW).kind, "lasts");
  assert.equal(eta5h({ perH: 0, src: "recent" }, 40, R, NOW).kind, "idle");
  assert.equal(eta5h({ perH: -1, src: "recent" }, 40, R, NOW).kind, "idle");
});

test("paceOf a texty tooltipu", () => {
  // týden: uplynulo přesně 3 d → plán 3/7 ≈ 42,9 %
  const R = NOW + 4 * DAY_MS;
  const w = paceOf("week", 47, R, undefined, NOW);
  assert.equal(w.over.lv, "warn");
  assert.equal(paceTip("week", w, clock, tr), "plán 42.9 % · +4.1 b. nad tempem · dnes zbývá 10.1 %");
  const under = paceOf("week", 30, R, undefined, NOW);
  assert.equal(under.over, null);
  assert.equal(paceTip("week", under, clock, tr), "plán 42.9 % · 12.9 b. pod tempem · dnes zbývá 27.1 %");
  // 5h
  const R5 = NOW + 3 * HOUR;
  const a = paceOf("5h", 40, R5, [[NOW - 20 * 60_000, 20], [NOW, 40]], NOW); // 60 %/h → dojde za 1 h
  assert.equal(a.fillLv, "warn");
  assert.equal(paceTip("5h", a, clock, tr), "tempo 60 %/h (posl. 30 min) · dojde T+60, obnova T+180 · zpomal na ≤ 20 %/h");
  const b = paceOf("5h", 40, R5, undefined, NOW);
  assert.equal(b.fillLv, null);
  assert.equal(paceTip("5h", b, clock, tr), "tempo 20 %/h (průměr okna) · vydrží do obnovy T+180");
  const c = paceOf("5h", 0, R5, undefined, NOW);
  assert.equal(paceTip("5h", c, clock, tr), "bez čerpání");
});

test("bez dat: chybí resetsAt / po resetu = null", () => {
  assert.equal(paceOf("week", 10, null, undefined, NOW), null);
  assert.equal(paceOf("5h", 10, NOW - 1, undefined, NOW), null);
});
