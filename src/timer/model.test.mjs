// Tabulkové testy logiky karty Časovač (`model.ts`), bez DOM.
//   node --experimental-strip-types --test src/timer/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { elapsedMs, fmtDuration, leftMs, parseMinutes, presetsOf, progress } from "./model.ts";

const T = 1_760_000_000_000;

test("zbývá a naměřeno: běží, pauza, doběhl", () => {
  const cases = [
    [{ state: "running", end: T + 90_000, total: 300_000 }, T, 90_000],
    [{ state: "running", end: T + 90_000, total: 300_000 }, T + 100_000, 0],
    [{ state: "paused", left: 5_000, total: 9_000 }, T + 1e9, 5_000],
    [{ state: "done", at: T, total: 60_000 }, T, 0],
  ];
  for (const [c, now, want] of cases) assert.equal(leftMs(c, now), want, JSON.stringify(c));
  assert.equal(elapsedMs({ state: "running", start: T - 42_000 }, T), 42_000);
  assert.equal(elapsedMs({ state: "running", start: T + 5 }, T), 0, "start v budoucnu (posun hodin) = 0");
  assert.equal(elapsedMs({ state: "paused", elapsed: 7 }, T + 99), 7);
});

test("průběh 0–1", () => {
  const cases = [
    [{ state: "running", end: T + 300_000, total: 300_000 }, 0],
    [{ state: "running", end: T + 150_000, total: 300_000 }, 0.5],
    [{ state: "paused", left: 75_000, total: 300_000 }, 0.75],
    [{ state: "done", at: T, total: 300_000 }, 1],
    [{ state: "running", end: T, total: 0 }, 1],
  ];
  for (const [c, want] of cases) assert.equal(progress(c, T), want, JSON.stringify(c));
});

test("formát se sekundami", () => {
  const cases = [
    [0, false, "0:00"],
    [59_999, false, "0:59"],
    [400, true, "0:01"],
    [25 * 60_000, true, "25:00"],
    [3_600_000 + 5 * 60_000 + 7_000, false, "1:05:07"],
    [-5, true, "0:00"],
  ];
  for (const [ms, up, want] of cases) assert.equal(fmtDuration(ms, up), want, `${ms} ${up}`);
});

test("vlastní minuty", () => {
  const cases = [["25", 25], [" 7 ", 7], ["1,5", 2], ["0", null], ["601", null], ["600", 600], ["abc", null], ["", null], ["-3", null], ["5min", null]];
  for (const [raw, want] of cases) assert.equal(parseMinutes(raw), want, raw);
});

test("předvolby: starý snapshot bez timer = výchozí", () => {
  assert.deepEqual(presetsOf(undefined), [5, 15, 25, 50]);
  assert.deepEqual(presetsOf({ countdown: null, stopwatch: null, presets: [] }), [5, 15, 25, 50]);
  assert.deepEqual(presetsOf({ countdown: null, stopwatch: null, presets: [10] }), [10]);
});

test("předvolby z pole Nastavení", async () => {
  const { parsePresets } = await import("./model.ts");
  const cases = [["5, 15, 25, 50", [5, 15, 25, 50]], ["10;10 20", [10, 20]], ["", []], ["0, 700, x, 3", [3]], ["1 2 3 4 5 6 7", [1, 2, 3, 4, 5, 6]]];
  for (const [raw, want] of cases) assert.deepEqual(parsePresets(raw), want, raw);
});
