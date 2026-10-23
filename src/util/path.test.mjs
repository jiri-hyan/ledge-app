// Tabulkový test porovnání cest (`util/path.ts`).
//   node --experimental-strip-types --test src/util/path.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { normPath, samePath } from "./path.ts";

test("normPath: lomítka, koncové lomítko, velikost písmen, mezery", () => {
  const rows = [
    ["C:/Work/App", "c:\\work\\app"],
    ["C:\\Work\\App\\", "c:\\work\\app"],
    ["  C:\\Work\\App//  ", "c:\\work\\app"],
    ["D:\\", "d:"],
    ["", ""],
  ];
  for (const [inp, want] of rows) assert.equal(normPath(inp), want, JSON.stringify(inp));
});

test("samePath: shodné a rozdílné cesty", () => {
  assert.ok(samePath("C:/Work/App/", "c:\\WORK\\app"));
  assert.ok(samePath("C:\\a\\b.md", "c:/a/b.md "));
  assert.ok(!samePath("C:\\a\\b", "C:\\a\\bc"));
  assert.ok(!samePath("C:\\a\\b", "D:\\a\\b"));
});
