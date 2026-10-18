// Čistá logika karet Systém a GitHub (bez DOM): velikosti, rychlost sítě, sparkline, úrovně, výběr disků.
//   node --experimental-strip-types --test src/integrations/format.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CI_TONE, HISTORY, countText, diskLoad, diskOn, diskUsed, fmtPct, fmtRate, fmtSize, isoMs, loadOf, sparkPoints, toggleDisk } from "./format.ts";

const GB = 1024 ** 3;

test("fmtSize: jednotky po 1024, zlomek jen u malých hodnot", () => {
  const cases = [
    [0, "0 B"],
    [-5, "0 B"],
    [NaN, "0 B"],
    [999, "999 B"],
    [1024, "1.0 KB"],
    [1536, "1.5 KB"],
    [10 * 1024, "10 KB"],
    [1024 ** 2, "1.0 MB"],
    [Math.round(11.24 * GB), "11 GB"],
    [Math.round(8.2 * GB), "8.2 GB"],
    [476 * GB, "476 GB"],
    [2 * 1024 * GB, "2.0 TB"],
    // těsně pod hranicí se po zaokrouhlení nesmí ukázat jako 1024 KB
    [1024 * 1024 - 100, "1.0 MB"],
  ];
  for (const [n, want] of cases) assert.equal(fmtSize(n), want, String(n));
  assert.equal(fmtSize(8.2 * GB, (v, d) => v.toFixed(d).replace(".", ",")), "8,2 GB", "formát čísla podle jazyka");
});

test("fmtRate: bajty a bity za sekundu, po 1000", () => {
  const cases = [
    [0, "bytes", "0 B/s"],
    [512, "bytes", "512 B/s"],
    [1_200_000, "bytes", "1.2 MB/s"],
    [45_000, "bytes", "45 KB/s"],
    [999_600, "bytes", "1.0 MB/s"],
    [2_500_000_000, "bytes", "2.5 GB/s"],
    [1_200_000, "bits", "9.6 Mbit/s"],
    [125_000, "bits", "1.0 Mbit/s"],
    [100, "bits", "800 bit/s"],
    [12_500_000, "bits", "100 Mbit/s"],
    [null, "bytes", "–"],
    [undefined, "bits", "–"],
    [NaN, "bytes", "–"],
    [-1, "bytes", "0 B/s"],
  ];
  for (const [n, unit, want] of cases) assert.equal(fmtRate(n, unit), want, `${n} ${unit}`);
});

test("fmtPct: celá čísla, bez vzorku pomlčka", () => {
  assert.equal(fmtPct(23.4), "23 %");
  assert.equal(fmtPct(99.5), "100 %");
  assert.equal(fmtPct(0), "0 %");
  assert.equal(fmtPct(null), "–");
});

test("sparkPoints: nejnovější vpravo, růst zprava, strop osy", () => {
  assert.equal(sparkPoints([], 120, 30, 100), "");
  assert.equal(sparkPoints([50], 120, 30, 100), "", "jeden vzorek nic nenakreslí");
  const p = sparkPoints([0, 100], 118, 30, 100).split(" ");
  assert.equal(p.length, 2);
  assert.equal(p[1], "118,1", "poslední vzorek na pravém okraji, 100 % nahoře (1 px okraj)");
  assert.equal(p[0], `${(118 - 118 / (HISTORY - 1)).toFixed(2).replace(/\.?0+$/, "")},29`, "0 % dole");
  // plný graf začíná na levém okraji
  const full = Array.from({ length: HISTORY }, (_, i) => i);
  assert.equal(sparkPoints(full, 118, 30).split(" ")[0].startsWith("0,"), true);
  // starší než HISTORY se zahodí
  assert.equal(sparkPoints(Array.from({ length: HISTORY + 20 }, () => 1), 118, 30, 1).split(" ").length, HISTORY);
  // automatický strop: největší hodnota, nejméně floor
  assert.equal(sparkPoints([0, 5], 118, 30, undefined, 10).split(" ")[1], "118,15", "5 z floor 10 = půl výšky");
  assert.equal(sparkPoints([0, 500], 118, 30).split(" ")[1], "118,1", "bez stropu je maximum nahoře");
  // hodnota nad stropem se ořízne, NaN se přeskočí
  assert.equal(sparkPoints([0, 250], 118, 30, 100).split(" ")[1], "118,1");
  assert.equal(sparkPoints([10, NaN, 20], 118, 30, 100).split(" ").length, 2);
});

test("úrovně: CPU / paměť od 75 a 90 %, disk podle volného místa", () => {
  const cases = [[null, "ok"], [0, "ok"], [74.9, "ok"], [75, "warm"], [89.9, "warm"], [90, "hot"], [100, "hot"]];
  for (const [v, want] of cases) assert.equal(loadOf(v), want, String(v));
  assert.equal(diskLoad(300 * GB, 476 * GB), "ok");
  assert.equal(diskLoad(80 * GB, 476 * GB), "warm", "17 % volných");
  assert.equal(diskLoad(30 * GB, 476 * GB), "hot", "6 % volných");
  assert.equal(diskLoad(5 * GB, 30 * GB), "hot", "pod 10 GB");
  assert.equal(diskLoad(0, 0), "ok");
  assert.equal(diskUsed(25, 100), 0.75);
  assert.equal(diskUsed(0, 0), 0);
  assert.equal(diskUsed(200, 100), 0, "volné nad celkem se ořízne");
});

test("toggleDisk: prázdný výběr = všechny; plný výběr se uloží jako prázdný", () => {
  const all = ["C", "D", "E"];
  assert.deepEqual(toggleDisk([], "d", all), ["C", "E"], "z „všech“ ubrat D");
  assert.deepEqual(toggleDisk(["C", "E"], "D", all), [], "doplněním D jsou zase všechny");
  assert.deepEqual(toggleDisk(["C", "E"], "C", all), ["E"]);
  assert.deepEqual(toggleDisk(["E"], "E", all), [], "poslední odebraný = prázdné (všechny)");
  assert.deepEqual(toggleDisk(["c"], "E", all), ["C", "E"], "malá písmena z ručně psaného configu");
  assert.equal(diskOn([], "C"), true);
  assert.equal(diskOn(["d"], "D"), true);
  assert.equal(diskOn(["D"], "C"), false);
});

test("CI, čas z GitHubu, počty", () => {
  assert.deepEqual(Object.keys(CI_TONE).sort(), ["failure", "none", "pending", "success"]);
  assert.equal(CI_TONE.success, "green");
  assert.equal(CI_TONE.failure, "red");
  assert.equal(isoMs("2026-10-17T15:26:56Z"), Date.UTC(2026, 9, 17, 15, 26, 56));
  assert.equal(isoMs(""), null);
  assert.equal(isoMs("včera"), null);
  assert.equal(isoMs(undefined), null);
  assert.equal(countText(20, true), "20+");
  assert.equal(countText(3, false), "3");
});
