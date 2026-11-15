// @vitest-environment node
import { test } from "node:test";
import assert from "node:assert/strict";
import { radioTarget, tabStop } from "./radioKeys.ts";

test("radioTarget: šipky kruhově, Home/End, cizí klávesy a prázdná skupina", () => {
  const rows = [
    ["ArrowRight", 0, 3, 1], ["ArrowDown", 2, 3, 0], ["ArrowLeft", 0, 3, 2], ["ArrowUp", 1, 3, 0],
    ["Home", 2, 3, 0], ["End", 0, 3, 2], ["ArrowRight", -1, 3, 0], ["ArrowLeft", -1, 3, 2],
    ["Enter", 0, 3, null], ["a", 1, 3, null], ["ArrowRight", 0, 0, null], ["ArrowRight", 0, 1, 0],
  ];
  for (const [k, cur, n, want] of rows) assert.equal(radioTarget(k, cur, n), want, `${k} ${cur}/${n}`);
});

test("tabStop: zastávka na vybraném, bez výběru na prvním; právě jedna na skupinu", () => {
  for (const [checked, want] of [[2, [-1, -1, 0, -1]], [-1, [0, -1, -1, -1]], [0, [0, -1, -1, -1]]]) {
    assert.deepEqual([0, 1, 2, 3].map((i) => tabStop(i, checked)), want, `vybrané ${checked}`);
  }
});
