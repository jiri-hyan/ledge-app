//   node --experimental-strip-types --test src/cards/temp.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { tempIn, tempUnit } from "./temp.ts";

test("tempIn: °C beze změny, °F přepočtem; starý snapshot (undefined) = °C", () => {
  for (const [c, f, want] of [[0, false, 0], [21.5, undefined, 21.5], [0, true, 32], [100, true, 212], [-40, true, -40], [-3.5, true, 25.7]]) {
    assert.ok(Math.abs(tempIn(c, f) - want) < 1e-9, `${c} ${f}`);
  }
  assert.equal(tempUnit(false), "°C");
  assert.equal(tempUnit(undefined), "°C");
  assert.equal(tempUnit(true), "°F");
});
