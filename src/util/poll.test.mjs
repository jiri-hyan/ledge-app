// Test sdíleného vzorkovače (`util/poll.ts`) s falešnými časovači: jedna smyčka pro víc odběratelů, zastavení po
// posledním odhlášení, žádný výsledek ze zrušené smyčky.
//   node --experimental-strip-types --test src/util/poll.test.mjs
import { test, mock } from "node:test";
import assert from "node:assert/strict";
import { sharedPoll } from "./poll.ts";

const flush = () => new Promise((r) => setImmediate(r));

test("sharedPoll: dva odběratelé = jeden dotaz za periodu, po posledním odhlášení stojí", async () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  let calls = 0;
  const got = [];
  const p = sharedPoll(async () => ++calls, (v) => got.push(v), 2000);
  const a = p.acquire();
  const b = p.acquire();
  assert.equal(p.subscribers, 2);
  await flush();
  assert.equal(calls, 1, "druhý odběratel nespouští druhou smyčku");
  mock.timers.tick(2000);
  await flush();
  assert.equal(calls, 2);
  a();
  a(); // opakované odhlášení nic nemění
  assert.equal(p.subscribers, 1);
  mock.timers.tick(2000);
  await flush();
  assert.equal(calls, 3, "smyčka běží, dokud je odběratel");
  b();
  mock.timers.tick(10_000);
  await flush();
  assert.equal(calls, 3, "bez odběratele žádný další dotaz");
  assert.deepEqual(got, [1, 2, 3]);
  mock.timers.reset();
});

test("sharedPoll: chyba a prázdný výsledek smyčku nezastaví ani nepřepíšou vzorek; znovupřihlášení začne hned", async () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  const seq = [Promise.reject(new Error("x")), Promise.resolve(null), Promise.resolve(7)];
  let i = 0;
  const got = [];
  const p = sharedPoll(() => seq[Math.min(i++, seq.length - 1)], (v) => got.push(v), 1000);
  const off = p.acquire();
  for (let k = 0; k < 3; k++) {
    await flush();
    mock.timers.tick(1000);
  }
  await flush();
  assert.equal(got[0], 7);
  off();
  const calls0 = i;
  p.acquire();
  await flush();
  assert.equal(i, calls0 + 1, "nový odběr = okamžitý dotaz");
  mock.timers.reset();
});
