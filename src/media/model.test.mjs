// Tabulkové testy čisté logiky záložky Osobní (`model.ts`): zbývající čas baterie, nízká baterie,
// ikona hlasitosti, omezení posuvníku při tahu.
//   node --experimental-strip-types --test src/media/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { batteryLow, remaining, throttle, volumeLevel } from "./model.ts";

test("zbývající čas baterie: hodiny a minuty, neznámo = null", () => {
  const cases = [
    [null, null],
    [undefined, null],
    [0, null],
    [-5, null],
    [Number.NaN, null],
    [59, { h: 0, m: 0 }],
    [60, { h: 0, m: 1 }],
    [45 * 60 + 59, { h: 0, m: 45 }],
    [3600, { h: 1, m: 0 }],
    [2 * 3600 + 10 * 60 + 30, { h: 2, m: 10 }],
    [26 * 3600, { h: 26, m: 0 }],
  ];
  for (const [s, want] of cases) assert.deepEqual(remaining(s), want, String(s));
});

test("nízká baterie: jen na baterii a ≤ 15 %", () => {
  const cases = [
    [15, false, true],
    [16, false, false],
    [3, false, true],
    [3, true, false],
    [100, false, false],
  ];
  for (const [p, plugged, want] of cases) assert.equal(batteryLow(p, plugged), want, `${p} ${plugged}`);
});

test("ikona hlasitosti podle úrovně a ztlumení", () => {
  const cases = [
    [0, false, "mute"],
    [50, true, "mute"],
    [1, false, "low"],
    [33, false, "low"],
    [34, false, "mid"],
    [66, false, "mid"],
    [67, false, "high"],
    [100, false, "high"],
  ];
  for (const [l, m, want] of cases) assert.equal(volumeLevel(l, m), want, `${l} ${m}`);
});

/** Falešné hodiny: `later` jen zapamatuje, `tick(ms)` posune čas a spustí, co dozrálo. */
function fakeClock() {
  let now = 1000;
  let jobs = [];
  return {
    now: () => now,
    later: (fn, ms) => {
      const job = { at: now + ms, fn };
      jobs.push(job);
      return job;
    },
    cancel: (h) => {
      jobs = jobs.filter((j) => j !== h);
    },
    tick(ms) {
      now += ms;
      const due = jobs.filter((j) => j.at <= now);
      jobs = jobs.filter((j) => j.at > now);
      for (const j of due) j.fn();
    },
    pending: () => jobs.length,
  };
}

test("throttle: první hned, při tahu nejvýš 1× za interval, poslední hodnota vždy", () => {
  const c = fakeClock();
  const sent = [];
  const t = throttle((v) => sent.push(v), 50, c);
  t.push(10);
  assert.deepEqual(sent, [10], "první hodnota hned");
  t.push(11);
  t.push(12);
  t.push(13);
  assert.deepEqual(sent, [10], "uvnitř intervalu nic");
  assert.equal(c.pending(), 1, "jeden čekající odklad, ne tři");
  c.tick(50);
  assert.deepEqual(sent, [10, 13], "na konci intervalu jen poslední");
  c.tick(200);
  t.push(20);
  assert.deepEqual(sent, [10, 13, 20], "po klidu zase hned");
  t.push(21);
  t.flush();
  assert.deepEqual(sent, [10, 13, 20, 21], "puštění myši pošle čekající hned");
  assert.equal(c.pending(), 0);
  t.flush();
  assert.deepEqual(sent, [10, 13, 20, 21], "prázdný flush nic neposílá");
});
