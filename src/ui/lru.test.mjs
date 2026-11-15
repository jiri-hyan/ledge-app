// Stropy cache (`lru.ts`). node --experimental-strip-types --test src/ui/lru.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { lruGet, lruSet, memoLru, trimRecord } from "./lru.ts";

test("lruSet: nad strop zahodí nejstarší, lruGet osvěží", () => {
  const m = new Map();
  lruSet(m, "a", 1, 2);
  lruSet(m, "b", 2, 2);
  assert.equal(lruGet(m, "a"), 1); // a je teď nejnovější
  assert.deepEqual(lruSet(m, "c", 3, 2), ["b"]);
  assert.deepEqual([...m.keys()], ["a", "c"]);
  assert.equal(lruGet(m, "zzz"), undefined);
});

test("lruSet: přepsání klíče nezvětší mapu", () => {
  const m = new Map();
  lruSet(m, "a", 1, 2);
  assert.deepEqual(lruSet(m, "a", 9, 2), []);
  assert.equal(m.size, 1);
});

test("memoLru: jedno volání na klíč, chyba se nepamatuje, strop drží", async () => {
  const m = new Map();
  let calls = 0;
  const load = (k) => (calls++, k === "bad" ? Promise.reject(new Error("x")) : Promise.resolve(k + "!"));
  assert.equal(await memoLru(m, "a", load, 2), "a!");
  assert.equal(await memoLru(m, "a", load, 2), "a!");
  assert.equal(calls, 1);
  await assert.rejects(memoLru(m, "bad", load, 2));
  await Promise.resolve();
  assert.equal(m.has("bad"), false);
  await memoLru(m, "b", load, 2);
  await memoLru(m, "c", load, 2);
  assert.ok(m.size <= 2);
});

test("trimRecord: nechá nejnovější klíče", () => {
  const r = { a: 1, b: 2, c: 3, d: 4 };
  assert.deepEqual(trimRecord(r, 2), ["a", "b"]);
  assert.deepEqual(Object.keys(r), ["c", "d"]);
  assert.deepEqual(trimRecord({ a: 1 }, 5), []);
});
