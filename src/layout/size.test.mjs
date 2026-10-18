// Tabulkové testy mechaniky mřížky (`size.ts`): rozpětí karet, řada bez děr, tah za okraj (šířka na
// sloupce, výška na řádky).
//   node --experimental-strip-types --test src/layout/size.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { deckCols, fitSize, heightPx, nextSizeFor, packSpans, resizeTo, rowsForPx, sizeForSpan, sizeOutcome, spanOf, spanPx, stepHeight, stepSize, usableSizes } from "./size.ts";

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

// ---------- vlna 9: rozložení, které vždy poslechne (diagnóza → test → oprava) ----------

test("případ 1: volné místo v řadě nedostane menší karta — zvolené S zůstane užší než soused", () => {
  // dřív: [M, S] ve 4 → S dostala volný sloupec = vypadala jako M a přepnutí S → M nic neudělalo
  const cases = [
    ["střední + malá ve 4: volno dostane střední", ["m", "s"], 4, [3, 1]],
    ["malá + střední ve 4", ["s", "m"], 4, [1, 3]],
    ["dvě malé ve 4: půl na půl", ["s", "s"], 4, [2, 2]],
    ["velká sama ve 4", ["l"], 4, [4]],
    ["střední + malá v 5", ["m", "s"], 5, [4, 1]],
  ];
  for (const [name, sizes, cols, want] of cases) assert.deepEqual(packSpans(sizes, cols), want, name);
  // vlastnost: v každé řadě má karta s nejmenší zvolenou velikostí nejvýš tolik sloupců jako ostatní
  // a karta s menší volbou nikdy není širší než karta s větší volbou
  const all = ["s", "m", "l", "xl"];
  for (const cols of [3, 4, 5, 6]) {
    for (const a of all) for (const b of all) for (const c of all) {
      const sizes = [a, b, c];
      const got = packSpans(sizes, cols);
      for (let i = 0; i < 3; i++) for (let j = 0; j < 3; j++) {
        if (spanOf(sizes[i], cols) < spanOf(sizes[j], cols) && sameRow(got, cols, i, j)) assert.ok(got[i] <= got[j], `${cols}: ${sizes} → ${got}`);
      }
    }
  }
});

/** jsou karty i a j ve stejné řadě (rozpětí po zabalení)? */
function sameRow(spans, cols, i, j) {
  const row = [];
  let r = 0;
  let used = 0;
  for (const s of spans) {
    if (used + s > cols) (r++, (used = 0));
    row.push(r);
    used += s;
  }
  return row[i] === row[j];
}

test("případ 2: volba velikosti bez viditelného efektu se ohlásí a vybere nejbližší možnou", () => {
  // (popis, velikosti záložky, index, chci, sloupce, uložená velikost, nápověda)
  const cases = [
    ["normální změna", ["m", "m"], 0, "s", 4, "s", null],
    ["karta v řadě sama: vyplní šířku", ["xl", "m", "xl"], 1, "s", 4, "s", "alone"],
    ["S → M u karty, která už volno vyplňuje", ["s", "s", "s"], 2, "m", 4, "m", "fill"],
    ["2 sloupce: Velká = celá šířka", ["s", "s"], 0, "l", 2, "xl", "cols"],
    ["3 sloupce: Velká = celá šířka", ["m", "s"], 1, "l", 3, "xl", "cols"],
    ["3 sloupce: Střední jde", ["s", "s", "s"], 0, "m", 3, "m", null],
    ["stejná velikost = nic, bez nápovědy", ["m", "m"], 0, "m", 4, "m", null],
  ];
  for (const [name, sizes, i, want, cols, size, note] of cases) {
    const got = sizeOutcome(sizes, i, want, cols);
    assert.equal(got.size, size, `${name}: velikost`);
    assert.equal(got.note, note, `${name}: nápověda`);
    assert.equal(got.span, packSpans(sizes.map((s, k) => (k === i ? got.size : s)), cols)[i], `${name}: rozpětí`);
  }
  // nabízené velikosti podle sloupců: vždy aspoň S a XL, jen ty, které vypadají jinak
  assert.deepEqual(usableSizes(4), ["s", "m", "l", "xl"]);
  assert.deepEqual(usableSizes(3), ["s", "m", "xl"]);
  assert.deepEqual(usableSizes(2), ["s", "xl"]);
  assert.deepEqual(usableSizes(1), ["s", "xl"]);
  assert.deepEqual(["s", "m", "l", "xl"].map((s) => fitSize(s, 2)), ["s", "xl", "xl", "xl"]);
});

test("případ 3: ⇔ a klávesy +/- jdou jen přes velikosti, které vypadají jinak", () => {
  // dřív: ve 2 sloupcích ⇔ třikrát po sobě nic (M, L i XL = celá šířka)
  assert.deepEqual(["s", "m", "l", "xl"].map((s) => nextSizeFor(s, 4)), ["m", "l", "xl", "s"]);
  assert.deepEqual(["s", "m", "l", "xl"].map((s) => nextSizeFor(s, 2)), ["xl", "s", "s", "s"]);
  assert.deepEqual(["s", "m", "xl"].map((s) => nextSizeFor(s, 3)), ["m", "xl", "s"]);
  // +/- bez kruhu: na kraji zůstane
  assert.equal(stepSize("s", -1, 4), "s");
  assert.equal(stepSize("s", 1, 4), "m");
  assert.equal(stepSize("xl", 1, 4), "xl");
  assert.equal(stepSize("xl", -1, 4), "l");
  assert.equal(stepSize("xl", -1, 2), "s");
  assert.equal(stepSize("l", -1, 3), "m");
});

test("případ 12: výška z klávesnice o krok (PageUp nižší, PageDown vyšší, za 4 = podle obsahu)", () => {
  // dřív: výška šla z klávesnice jen kruhem ↕ (4 → podle obsahu → 1 = nečekaný skok)
  const cases = [
    [undefined, 1, undefined],
    [undefined, -1, 4],
    [4, 1, undefined],
    [1, -1, 1],
    [2, 1, 3],
    [3, -1, 2],
  ];
  for (const [h, dir, want] of cases) assert.equal(stepHeight(h, dir), want, `${h} ${dir}`);
});
