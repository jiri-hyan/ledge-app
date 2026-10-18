// Tabulkové testy mechaniky mřížky (`size.ts`): rozpětí karet, řada bez děr, tah za okraj (šířka na
// sloupce, výška na řádky).
//   node --experimental-strip-types --test src/layout/size.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { deckCols, heightPx, packSpans, resizeTo, rowsForPx, sizeForSpan, spanOf, spanPx } from "./size.ts";

test("rozpětí velikostí = `spans` ve vzhledu (theme/looks.ts)", () => {
  for (const cols of [1, 2, 3, 4, 5, 6]) {
    assert.deepEqual(["s", "m", "l", "xl"].map((s) => spanOf(s, cols)), [1, Math.min(2, cols), Math.min(3, cols), cols], `${cols}`);
  }
  const looks = readFileSync(new URL("../theme/looks.ts", import.meta.url), "utf8");
  assert.ok(looks.includes("s: 1, m: Math.min(2, cols), l: Math.min(3, cols), xl: cols"), "zrcadlo `spans`");
  const cases = [["", 4], [" 5 ", 5], ["abc", 4], ["0", 4], ["99", 12], [null, 4]];
  for (const [raw, want] of cases) assert.equal(deckCols(raw), want, String(raw));
});

test("bez děr: volné sloupce řady dostanou její karty, pořadí se nemění", () => {
  // (popis, velikosti, sloupce, rozpětí)
  const cases = [
    ["screenshot 0.2.16: Claude + Repa vedle sebe v 6 sloupcích, pak Poznámky", ["xl", "m", "m", "xl"], 6, [6, 3, 3, 6]],
    ["Claude + Repa ve 4 = beze změny", ["xl", "m", "m", "xl"], 4, [4, 2, 2, 4]],
    ["Claude + Repa v 5 (Hustý pruh)", ["m", "m", "xl"], 5, [2, 3, 5]],
    ["Claude + Repa ve 3: zalomí se, každá řada plná", ["m", "m"], 3, [3, 3]],
    ["čtyři malé ve 4", ["s", "s", "s", "s"], 4, [1, 1, 1, 1]],
    ["tři malé ve 4: poslední širší", ["s", "s", "s"], 4, [1, 1, 2]],
    ["poslední řada taky plná", ["m", "m", "s"], 4, [2, 2, 4]],
    ["malá + velká ve 4", ["s", "l"], 4, [1, 3]],
    ["velká + střední ve 4: střední na další řadu přes celou", ["l", "m"], 4, [4, 4]],
    ["jeden sloupec", ["s", "m", "xl"], 1, [1, 1, 1]],
    ["nic", [], 4, []],
  ];
  for (const [name, sizes, cols, want] of cases) {
    const got = packSpans(sizes, cols);
    assert.deepEqual(got, want, name);
    // každá řada je plná a žádná karta není širší než mřížka
    let used = 0;
    for (const s of got) {
      assert.ok(s >= 1 && s <= cols, `${name}: ${s}`);
      if (used + s > cols) {
        assert.equal(used, cols, `${name}: díra v řadě`);
        used = 0;
      }
      used += s;
    }
    if (got.length) assert.equal(used, cols, `${name}: díra v poslední řadě`);
  }
});

test("tah za okraj: šířka se přichytí na sloupce, výška na řádky", () => {
  // 4 sloupce po 100 px, mezera 10, řádek 100; karta začíná na (0, 0)
  const m = { cols: 4, colW: 100, gap: 10, row: 100, left: 0, top: 0 };
  const start = { size: "m", h: undefined };
  const cases = [
    ["pravý okraj na 1 sloupec", "e", 90, 999, { size: "s", h: undefined }],
    ["pravý okraj na 3 sloupce", "e", 320, 999, { size: "l", h: undefined }],
    ["pravý okraj za mřížku = celá řada", "e", 2000, 0, { size: "xl", h: undefined }],
    ["pravý okraj vlevo od karty = nejmenší", "e", -50, 0, { size: "s", h: undefined }],
    ["spodní okraj na 3 řádky", "s", 999, 310, { size: "m", h: 3 }],
    ["spodní okraj nahoru = 1 řádek", "s", 0, 5, { size: "m", h: 1 }],
    ["spodní okraj hodně dolů = 4", "s", 0, 5000, { size: "m", h: 4 }],
    ["roh = obojí", "se", 210, 200, { size: "m", h: 2 }],
  ];
  for (const [name, edge, x, y, want] of cases) assert.deepEqual(resizeTo(m, edge, start, x, y), want, name);
  // 5 sloupců (Hustý pruh): 4 sloupce nejsou velikost → nejbližší menší; 5 = celá řada
  assert.equal(sizeForSpan(4, 5), "l");
  assert.equal(sizeForSpan(5, 5), "xl");
  assert.equal(sizeForSpan(2, 2), "xl");
  assert.equal(sizeForSpan(1, 3), "s");
  assert.deepEqual([1, 2, 3, 4].map((h) => heightPx(h, 100, 10)), [100, 210, 320, 430]);
  assert.deepEqual([0, 100, 160, 210, 9999].map((px) => rowsForPx(px, 100, 10)), [1, 1, 2, 2, 4]);
  assert.equal(spanPx(2, 100, 10), 210);
});
