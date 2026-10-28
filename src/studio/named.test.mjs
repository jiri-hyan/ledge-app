// Popisky hodnot v časové ose (kolo 24).
//   node --experimental-strip-types --test src/studio/named.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { namedValue } from "./named.ts";

test("známé hodnoty mají český název, neznámé null", () => {
  for (const [p, v, cs] of [["appearance.theme", "dark", "Tmavý"], ["appearance.layout", "float", "Plovoucí panel"], ["appearance.material", "glass", "Sklo"], ["appearance.width", "narrow", "Úzký"], ["appearance.font", "system", "Segoe (výchozí)"]]) {
    assert.equal(namedValue(p, v)?.[1], cs, `${p}=${v}`);
  }
  assert.ok(namedValue("appearance.style", "classic"));
  for (const [p, v] of [["appearance.theme", "nonexistent"], ["jina.cesta", "x"], ["appearance.layout", ""]]) assert.equal(namedValue(p, v), null, `${p}=${v}`);
});
