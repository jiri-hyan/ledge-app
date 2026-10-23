// @vitest-environment node
// Tabulkový test výběru barvy obalu (`tint.ts` `pickTint`).
//   node --experimental-strip-types --test src/media/tint.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { canTint, pickTint } from "./tint.ts";

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
