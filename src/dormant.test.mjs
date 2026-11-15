// Odmontování skrytého šuplíku (`dormant.ts`): kdy ano a kdy ne. node --experimental-strip-types --test src/dormant.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { DORMANT_AFTER_MS, isDormant } from "./dormant.ts";

const base = { view: "hidden", page: "main", editing: false, hiddenLong: true };

test("isDormant: tabulka", () => {
  const rows = [
    [{}, true],
    [{ hiddenLong: false }, false], // skryto krátce
    [{ view: "full" }, false],
    [{ view: "peek" }, false],
    [{ view: "mid" }, false],
    [{ page: "note" }, false], // rozepsaná poznámka
    [{ page: "settings" }, false],
    [{ page: "notes" }, false],
    [{ page: "welcome" }, false],
    [{ editing: true }, false], // rozpracované úpravy rozložení
  ];
  for (const [over, want] of rows) assert.equal(isDormant({ ...base, ...over }), want, JSON.stringify(over));
});

test("lhůta je rozumná (1–10 min)", () => {
  assert.ok(DORMANT_AFTER_MS >= 60_000 && DORMANT_AFTER_MS <= 600_000);
});
