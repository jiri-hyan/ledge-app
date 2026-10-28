// Tabulkové testy karty Rychlé dlaždice (`model.ts`): bez DOM, bez registru a zvuku.
//   node --experimental-strip-types --test src/winquick/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { TILE_IDS, addTile, baseName, cleanTiles, missingTiles, moveTile, pinFolder, radioOf, removeTile } from "./model.ts";

test("cleanTiles: neznámé a duplicity pryč, pořadí zůstane", () => {
  const cases = [
    [["out", "mic"], ["out", "mic"]],
    [["mic", "mic", "x", "shot"], ["mic", "shot"]],
    [[], []],
  ];
  for (const [inp, want] of cases) assert.deepEqual(cleanTiles(inp), want);
});

test("add / remove / missing", () => {
  assert.deepEqual(missingTiles(["mic", "out"]), TILE_IDS.filter((t) => t !== "mic" && t !== "out"));
  assert.deepEqual(addTile(["mic"], "out"), ["mic", "out"]);
  assert.deepEqual(addTile(["mic"], "mic"), ["mic"]);
  assert.deepEqual(addTile(["mic"], "nope"), ["mic"]);
  assert.deepEqual(removeTile(["mic", "out"], "mic"), ["out"]);
  assert.deepEqual(missingTiles(TILE_IDS), []);
});

test("moveTile: okraje beze změny", () => {
  assert.deepEqual(moveTile(["mic", "out", "dir"], "out", -1), ["out", "mic", "dir"]);
  assert.deepEqual(moveTile(["mic", "out", "dir"], "out", 1), ["mic", "dir", "out"]);
  assert.deepEqual(moveTile(["mic", "out"], "mic", -1), ["mic", "out"]);
  assert.deepEqual(moveTile(["mic", "out"], "out", 1), ["mic", "out"]);
  assert.deepEqual(moveTile(["mic"], "dir", 1), ["mic"]);
});

test("radioOf: bez rádia null", () => {
  const r = [{ kind: "bluetooth", name: "BT", on: true }];
  assert.equal(radioOf(r, "wifi"), null);
  assert.equal(radioOf(r, "bt")?.on, true);
  assert.equal(radioOf([], "bt"), null);
});

test("pinFolder: duplicity, strop, prázdné", () => {
  assert.deepEqual(pinFolder([], " C:\\Work\\app "), ["C:\\Work\\app"]);
  assert.deepEqual(pinFolder(["C:\\Work\\app"], "c:\\work\\APP\\"), ["C:\\Work\\app"]);
  assert.deepEqual(pinFolder(["a"], ""), ["a"]);
  assert.deepEqual(pinFolder(["a", "b"], "c", 2), ["a", "b"]);
});

test("baseName", () => {
  assert.equal(baseName("C:\\Work\\app-v2\\"), "app-v2");
  assert.equal(baseName("C:\\"), "C:");
});
