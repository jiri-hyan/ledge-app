// @vitest-environment node
// Návrat z rychlé poznámky v rozložení Notch (0.11.x): memo `notch` v MainPage se vyhodnotí hned při vzniku
// stránky a přes `facts()` sahá na `todayRows` — deklarace za ním = TDZ pád a stránka zůstala na poznámce.
//   node --experimental-strip-types --test src/notch/order.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const src = readFileSync(new URL("../MainPage.tsx", import.meta.url), "utf8");

test("MainPage: všechno, co čte memo notch (facts → todayRows, has), je deklarované před ním", () => {
  const memo = src.indexOf("const notch = createMemo(");
  assert.ok(memo > 0, "memo notch nenalezeno");
  for (const name of ["todayRows", "has", "facts", "shown", "on"]) {
    const at = src.search(new RegExp(`const ${name} = `));
    assert.ok(at > 0 && at < memo, `const ${name} musí být před memem notch`);
  }
});
