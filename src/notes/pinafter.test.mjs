// P8 (audit k34): selhání připnutí po úspěšném vytvoření poznámky nesmí vyhodit výjimku z `saveDraft`
// (koncept by zůstal a další Uložit by založil druhou poznámku).
import { test } from "node:test";
import assert from "node:assert/strict";
import { pinAfterCreate } from "./pinafter.ts";

test("pinAfterCreate: připne, nebo vrátí chybu — nikdy nevyhodí", async () => {
  const calls = [];
  const ok = (p) => (calls.push(p), Promise.resolve());
  const bad = () => Promise.reject(new Error("okno nejde otevřít"));
  const sync = () => {
    throw new Error("sync");
  };
  assert.equal(await pinAfterCreate(false, "a.md", ok), null);
  assert.equal(await pinAfterCreate(true, "", ok), null);
  assert.equal(await pinAfterCreate(true, null, ok), null);
  assert.deepEqual(calls, []);
  assert.equal(await pinAfterCreate(true, "a.md", ok), null);
  assert.deepEqual(calls, ["a.md"]);
  assert.match(await pinAfterCreate(true, "a.md", bad), /okno nejde otevřít/);
  assert.match(await pinAfterCreate(true, "a.md", sync), /sync/);
});
