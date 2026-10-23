// Test sdíleného debounce (`util/time.ts`) s falešnými časovači.
//   node --experimental-strip-types --test src/util/time.test.mjs
import { test, mock } from "node:test";
import assert from "node:assert/strict";
import { debounce } from "./time.ts";

test("debounce: volá až po pauze, s posledními argumenty, čekání začíná znovu", () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  const calls = [];
  const d = debounce((x) => calls.push(x), 100);
  d(1);
  mock.timers.tick(60);
  d(2);
  mock.timers.tick(60);
  assert.deepEqual(calls, [], "druhé volání posunulo čekání");
  assert.equal(d.pending, true);
  mock.timers.tick(40);
  assert.deepEqual(calls, [2]);
  assert.equal(d.pending, false);
  mock.timers.reset();
});

test("debounce: flush provede čekající volání hned, jinak nic; cancel ho zahodí", () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  const calls = [];
  const d = debounce((x) => calls.push(x), 100);
  d.flush();
  assert.deepEqual(calls, [], "nic nečeká");
  d("a");
  d.flush();
  assert.deepEqual(calls, ["a"]);
  mock.timers.tick(500);
  assert.deepEqual(calls, ["a"], "po flush už časovač nevolá podruhé");
  d("b");
  d.cancel();
  mock.timers.tick(500);
  assert.deepEqual(calls, ["a"]);
  assert.equal(d.pending, false);
  mock.timers.reset();
});
