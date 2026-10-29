// Geometrie budíků a prstenců (`dial.ts`).
import test from "node:test";
import assert from "node:assert/strict";
import { concentric, dialDeg, dialPath, frac01, needleEnd, polar, ringDash, toneOf } from "./dial.ts";

test("frac01: ořez a nečíslo", () => {
  const cases = [[0.4, 0.4], [-1, 0], [7, 1], [NaN, null], [Infinity, null], [null, null], [undefined, null]];
  for (const [i, o] of cases) assert.equal(frac01(i), o, String(i));
});

test("polar a úhel budíku: 0 vlevo, 0,5 nahoře, 1 vpravo", () => {
  assert.deepEqual(polar(50, 50, 10, 0), { x: 50, y: 40 });
  assert.deepEqual(polar(50, 50, 10, 90), { x: 60, y: 50 });
  assert.deepEqual([dialDeg(0), dialDeg(0.5), dialDeg(1)], [-90, 0, 90]);
  assert.deepEqual(needleEnd(50, 50, 10, 0), { x: 40, y: 50 });
  assert.deepEqual(needleEnd(50, 50, 10, 0.5), { x: 50, y: 40 });
  assert.deepEqual(needleEnd(50, 50, 10, 1), { x: 60, y: 50 });
  assert.deepEqual(needleEnd(50, 50, 10, NaN), { x: 40, y: 50 }, "bez údaje stojí vlevo");
});

test("dialPath: celý půlkruh, část, prázdné", () => {
  assert.equal(dialPath(50, 50, 10, 0, 1), "M40 50 A10 10 0 0 1 60 50");
  assert.equal(dialPath(50, 50, 10, 0, 0.5), "M40 50 A10 10 0 0 1 50 40");
  for (const [a, b] of [[0.5, 0.5], [0.8, 0.2], [NaN, 1], [0, NaN]]) assert.equal(dialPath(50, 50, 10, a, b), "", `${a}-${b}`);
});

test("ringDash: podíl na obvodu", () => {
  const d = ringDash(10, 0.25);
  assert.equal(d.c, 62.83);
  assert.equal(d.dash, "15.71 62.83");
  assert.equal(ringDash(10, 0).dash, "0 62.83");
  assert.equal(ringDash(10, 3).dash, "62.83 62.83");
  assert.equal(ringDash(10, null), null);
});

test("concentric: vnější první, vejde se jen co se vejde", () => {
  assert.deepEqual(concentric(3, 100, 8, 3), [46, 35, 24]);
  assert.deepEqual(concentric(5, 20, 8, 3), [6], "úzké pole: další prstence se nevejdou");
});

test("toneOf: meze", () => {
  const cases = [[null, "ok"], [69, "ok"], [70, "warn"], [89.9, "warn"], [90, "bad"], [100, "bad"]];
  for (const [p, o] of cases) assert.equal(toneOf(p, 70, 90), o, String(p));
});
