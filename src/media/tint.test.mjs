// @vitest-environment node
// Tabulkový test výběru barvy obalu (`tint.ts` `pickTint`).
//   node --experimental-strip-types --test src/media/tint.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { accentKey, canTint, pickTint } from "./tint.ts";

const px = (...cols) => cols.flatMap(([r, g, b, a = 255]) => [r, g, b, a]);

test("barva obalu: nejsytější rozumná, jinak průměr, jinak nic", () => {
  const rows = [
    [px([200, 40, 40], [90, 90, 120]), "#c82828", "sytá červená vyhraje"],
    [px([0, 0, 0], [255, 255, 255], [10, 10, 10]), null, "jen černá a bílá = bez podbarvení"],
    [px([120, 110, 100], [125, 115, 105]), "#7b7167", "málo sytý obal = průměr"],
    [px([200, 40, 40, 0]), null, "průhledné pixely se nepočítají"],
    [[], null, "prázdný obrázek"],
  ];
  for (const [input, want, why] of rows) assert.equal(pickTint(input), want, why);
});

test("zdroj obalu: jen data URL z jádra (jiný původ by canvas znečistil a tiše vrátil null)", () => {
  const rows = [
    ["data:image/png;base64,iVBORw0KGgo=", true],
    ["data:image/jpeg;base64,/9j/4AAQ", true],
    ["http://asset.localhost/C:/covers/a.png", false],
    ["https://example.com/cover.png", false],
    ["data:image/svg+xml;base64,PHN2Zz4=", false],
    ["", false],
    [null, false],
  ];
  for (const [url, want] of rows) assert.equal(canTint(url), want, String(url));
});

import { contrast, withContrast } from "./tint.ts";
test("kontrast barvy obalu proti kartě: ≥ 3:1 a odstín zůstává, bez obalu null", () => {
  const dark = "#1c1c20", light = "#ffffff";
  const rows = [
    ["černý obal", [px([0, 0, 0], [5, 5, 5])], null],
    ["bílý obal", [px([255, 255, 255], [250, 250, 250])], null],
    ["šedý obal", [px([128, 128, 128], [130, 130, 130])], null],
    ["barevný obal", [px([200, 40, 40], [90, 90, 120])], "#c82828"],
    ["žádný obal", [[]], null],
  ];
  for (const [why, [input], want] of rows) assert.equal(pickTint(input), want, why);
  for (const c of ["#c82828", "#808080", "#101040", "#f0f0a0", "#0a0a0a"]) {
    for (const bg of [dark, light]) assert.ok(contrast(withContrast(c, bg), bg) >= 3, `${c} na ${bg}`);
  }
  assert.equal(withContrast("#f0c040", dark), "#f0c040", "dost kontrastní zůstane");
  assert.equal(withContrast("#0a0a0a", dark) !== "#0a0a0a", true, "tmavé na tmavém se zesvětlí");
});

test("cache obalu: klíč zahrnuje pozadí karty, po přepnutí motivu se kontrast přepočítá", () => {
  assert.notEqual(accentKey("cover", "#1c1c20"), accentKey("cover", "#f4f4f6"));
  assert.equal(accentKey("cover", "#1C1C20"), accentKey("cover", "#1c1c20"));
  assert.notEqual(accentKey("a", "#1c1c20"), accentKey("b", "#1c1c20"));
});
