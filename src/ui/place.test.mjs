// @vitest-environment node
// Tabulkové testy umístění plovoucích prvků (`place.ts`): pod kotvu, otočení nahoru, posun do okna u okraje
// a v úzkém okně, vyšší než okno; počet sloupců mřížky dlaždic (vyplní šířku, zalomí, nikdy 0).
//   node --experimental-strip-types --test src/ui/place.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { placePopup, pointBox, tileColumns } from "./place.ts";

const view = { w: 1000, h: 600 };
const tile = (left, top, w = 100, h = 100) => ({ left, top, right: left + w, bottom: top + h });

test("popover: pod kotvou, otočení nad ni, posun do okna", () => {
  const cases = [
    ["místo dole", tile(100, 100), { w: 200, h: 150 }, {}, { x: 100, y: 204, flipped: false }],
    ["u spodního okraje → nahoru", tile(100, 450), { w: 200, h: 150 }, {}, { x: 100, y: 296, flipped: true }],
    ["u pravého okraje → posun doleva", tile(950, 100, 40, 40), { w: 200, h: 100 }, {}, { x: 792, y: 144, flipped: false }],
    ["zarovnat na pravý okraj kotvy", tile(500, 100), { w: 200, h: 100 }, { align: "end" }, { x: 400, y: 204, flipped: false }],
    ["u levého okraje (end) → posun doprava", tile(10, 100, 40, 40), { w: 200, h: 100 }, { align: "end" }, { x: 8, y: 144, flipped: false }],
    ["nevejde se ani nahoru → strana s víc místem, uvnitř okna", tile(100, 250, 100, 100), { w: 200, h: 400 }, {}, { x: 100, y: 192, flipped: false }],
    ["vyšší než okno → nahoře, maxH", tile(100, 100), { w: 200, h: 2000 }, {}, { x: 100, y: 8, flipped: false }],
    ["pravý klik u rohu", pointBox(990, 590), { w: 180, h: 120 }, {}, { x: 812, y: 466, flipped: true }],
  ];
  for (const [name, anchor, size, opts, want] of cases) {
    const got = placePopup(anchor, size, view, opts);
    assert.deepEqual({ x: got.x, y: got.y, flipped: got.flipped }, want, name);
    // vždy celé v okně s okrajem
    const h = Math.min(size.h, got.maxH);
    assert.ok(got.x >= 8 && got.x + Math.min(size.w, view.w - 16) <= view.w - 8, `${name}: x v okně`);
    assert.ok(got.y >= 8 && got.y + h <= view.h - 8, `${name}: y v okně`);
  }
});

test("popover v úzkém okně: širší než okno → od okraje, nikdy záporně", () => {
  const narrow = { w: 300, h: 400 };
  const got = placePopup(tile(200, 50, 60, 60), { w: 420, h: 200 }, narrow);
  assert.deepEqual({ x: got.x, y: got.y }, { x: 8, y: 114 });
  assert.equal(placePopup(tile(0, 0), { w: 10, h: 10 }, { w: 0, h: 0 }).maxH, 0);
});

test("sloupce dlaždic: vyplní šířku, nejvýš počet dlaždic, zalomí, aspoň 1", () => {
  const cases = [
    // šířka, min, mezera, dlaždic → sloupců
    ["kompaktní ikony, málo dlaždic = roztáhnout na celou šířku", 800, 52, 4, 5, 5],
    ["kompaktní ikony, hodně = kolik se vejde, zbytek další řádek", 800, 52, 4, 40, 14],
    ["dlaždice M", 500, 118, 8, 3, 3],
    ["dlaždice M plná řada (4 × 118 + 3 × 8 = 496)", 500, 118, 8, 9, 4],
    ["přesně na hraně", 252, 118, 16, 9, 2],
    ["úzká karta", 60, 118, 8, 3, 1],
    ["žádná dlaždice", 500, 118, 8, 0, 1],
    ["nezměřená šířka", 0, 118, 8, 5, 1],
  ];
  for (const [name, w, min, gap, n, want] of cases) assert.equal(tileColumns(w, min, gap, n), want, name);
});
