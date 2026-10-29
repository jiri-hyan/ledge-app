// Vzhledy karty Systém (`systemLooks.ts`): textové pruhy, historie, nejzaplněnější disk, sloupky zátěže.
import test from "node:test";
import assert from "node:assert/strict";
import { ageSec, fullestDisk, histStats, lastN, loadBars, termCells, windowSec } from "./systemLooks.ts";

test("termCells: zaokrouhlení, aspoň jedna buňka, ořez, nečíslo", () => {
  const cases = [
    [0, 10, "▯▯▯▯▯▯▯▯▯▯"],
    [1, 10, "▮▯▯▯▯▯▯▯▯▯"],
    [50, 10, "▮▮▮▮▮▯▯▯▯▯"],
    [96, 10, "▮▮▮▮▮▮▮▮▮▮"],
    [100, 4, "▮▮▮▮"],
    [150, 4, "▮▮▮▮"],
    [-5, 4, "▯▯▯▯"],
    [NaN, 3, "▯▯▯"],
    [null, 3, "▯▯▯"],
    [50, 0, ""],
  ];
  for (const [p, n, want] of cases) assert.equal(termCells(p, n), want, `${p}/${n}`);
});

test("lastN a histStats", () => {
  assert.deepEqual(lastN([1, 2, NaN, 3, 4], 3), [2, 3, 4].slice(-3));
  assert.deepEqual(lastN(undefined, 3), []);
  assert.deepEqual(lastN([1, 2], 0), []);
  assert.equal(histStats([]), null);
  assert.equal(histStats(undefined), null);
  assert.deepEqual(histStats([10, 30, 50]), { avg: 30, peak: 50, n: 3 });
});

test("ageSec a windowSec: poslední vzorek je teď, každý starší o 2 s", () => {
  assert.deepEqual([0, 1, 4].map((i) => ageSec(i, 5)), [8, 6, 0]);
  assert.equal(windowSec(60), 120);
  assert.equal(windowSec(20), 40);
});

test("fullestDisk: největší zaplnění, disk bez velikosti se přeskočí", () => {
  const G = 1024 ** 3;
  const disks = [{ letter: "C", total: 500 * G, free: 100 * G }, { letter: "D", total: 1000 * G, free: 100 * G }, { letter: "E", total: 0, free: 0 }];
  const r = fullestDisk(disks);
  assert.equal(r.disk.letter, "D");
  assert.equal(r.used, 0.9);
  assert.equal(fullestDisk([]), null);
  assert.equal(fullestDisk(undefined), null);
  assert.equal(fullestDisk([{ letter: "E", total: 0, free: 0 }]), null);
});

test("loadBars: výška a úroveň podle zátěže", () => {
  const b = loadBars([10, 80, 95, 120], 3);
  assert.deepEqual(b.map((x) => [x.pct, x.h, x.load, x.age]), [[80, 0.8, "warm", 4], [95, 0.95, "hot", 2], [120, 1, "hot", 0]]);
  assert.deepEqual(loadBars([], 5), []);
});
