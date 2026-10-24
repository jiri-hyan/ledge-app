// @vitest-environment node
// Geometrie vlastních SVG grafů (`geom.ts`): tabulky vstup → výstup, poctivý prázdný stav (< 2 čísla = nic),
// vynechání nekonečných čísel, ořez rozsahu.
//   node --experimental-strip-types --test src/charts/geom.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { bars, extent, finite, heat, heatLevel, ring, sparkPoints, sparkline, trend } from "./geom.ts";

test("prázdný stav: méně než 2 konečná čísla = nic", () => {
  const empty = [[], [5], [NaN, 3], [null, undefined, 4], [Infinity, -Infinity]];
  for (const v of empty) {
    assert.equal(sparkline(v, 100, 20), null, JSON.stringify(v));
    assert.equal(sparkPoints(v, 100, 20), null);
    assert.deepEqual(bars(v, 100, 20), []);
    assert.deepEqual(heat(v, 100, 20), []);
    assert.equal(trend(v), null);
  }
  // nesmyslné rozměry = nic
  assert.equal(sparkline([1, 2], 0, 20), null);
  assert.deepEqual(bars([1, 2], 100, -1), []);
});

test("finite a extent", () => {
  assert.deepEqual(finite([1, null, NaN, 2, undefined, Infinity, 3]), [1, 2, 3]);
  const cases = [
    [[1, 5, 3], {}, { lo: 1, hi: 5 }],
    [[2, 2], {}, { lo: 2, hi: 3 }],
    [[1, 5], { min: 0, max: 100 }, { lo: 0, hi: 100 }],
    [[7, 7], { min: 7, max: 7 }, { lo: 7, hi: 8 }],
  ];
  for (const [v, o, want] of cases) assert.deepEqual(extent(v, o), want, JSON.stringify([v, o]));
});

test("sparkline: body přes celou šířku, poslední bod vpravo, plocha uzavřená dole", () => {
  const s = sparkline([0, 10, 5], 100, 20, { pad: 0 });
  assert.deepEqual(s.points, [{ x: 0, y: 20 }, { x: 50, y: 0 }, { x: 100, y: 10 }]);
  assert.equal(s.line, "M0 20 L50 0 L100 10");
  assert.equal(s.area, "M0 20 L50 0 L100 10 L100 20 L0 20 Z");
  assert.deepEqual(s.last, { x: 100, y: 10 });
  // výchozí okraj 1.5 px: čára nepřeteče viewBox
  const p = sparkPoints([1, 2], 50, 10);
  assert.deepEqual(p, [{ x: 1.5, y: 8.5 }, { x: 48.5, y: 1.5 }]);
  // pevný rozsah (procenta) a ořez mimo něj
  assert.deepEqual(sparkPoints([50, 150], 10, 10, { min: 0, max: 100, pad: 0 }), [{ x: 0, y: 5 }, { x: 10, y: 0 }]);
  // plochá data = vodorovná čára dole (rozsah 1)
  assert.deepEqual(sparkPoints([3, 3, 3], 10, 10, { pad: 0 }).map((q) => q.y), [10, 10, 10]);
  // chybějící čísla se vynechají, ne nahradí nulou
  assert.deepEqual(sparkPoints([0, null, 10], 10, 10, { pad: 0 }), [{ x: 0, y: 10 }, { x: 10, y: 0 }]);
});

test("prstenec: podíl 0–1, ořez, nečíslo = nic", () => {
  const r = ring(0.25, 40, 4);
  assert.equal(r.r, 18);
  assert.equal(r.c, 113.1);
  assert.equal(r.dash, "28.28 113.1");
  assert.equal(r.frac, 0.25);
  assert.equal(ring(1.7, 40, 4).frac, 1);
  assert.equal(ring(-1, 40, 4).dash, "0 113.1");
  for (const bad of [null, undefined, NaN, Infinity]) assert.equal(ring(bad, 40, 4), null, String(bad));
  assert.equal(ring(0.5, 40, 20), null, "tloušťka větší než poloměr");
  assert.equal(ring(0.5, 0, 2), null);
});

test("sloupce: výška podle maxima, mezery, záporné = 0, minH", () => {
  assert.deepEqual(bars([1, 2], 10, 10, { gap: 2 }), [
    { x: 0, y: 5, w: 4, h: 5 },
    { x: 6, y: 0, w: 4, h: 10 },
  ]);
  assert.deepEqual(bars([0, -3, 4], 32, 8, { gap: 1, minH: 1 }).map((b) => b.h), [1, 1, 8]);
  assert.deepEqual(bars([5, 10], 10, 10, { gap: 0, max: 20 }).map((b) => b.h), [2.5, 5]);
  assert.deepEqual(bars([0, 0], 10, 10).map((b) => b.h), [0, 0], "samé nuly = nulové sloupce, ne vymyšlené");
  assert.deepEqual(bars([1, 2, 3], 2, 10, { gap: 2 }), [], "nevejde se = nic");
});

test("pás: úrovně a mřížka po sloupcích", () => {
  const levels = [
    [0, 10, 4, 0],
    [1, 10, 4, 1],
    [2.5, 10, 4, 1],
    [2.6, 10, 4, 2],
    [10, 10, 4, 4],
    [20, 10, 4, 4],
    [-1, 10, 4, 0],
    [5, 0, 4, 0],
  ];
  for (const [v, max, n, want] of levels) assert.equal(heatLevel(v, max, n), want, JSON.stringify([v, max, n]));
  const row = heat([0, 5, 10], 32, 8, { gap: 1 });
  assert.deepEqual(row.map((c) => [c.x, c.y, c.w, c.h, c.level]), [
    [0, 0, 10, 8, 0],
    [11, 0, 10, 8, 2],
    [22, 0, 10, 8, 4],
  ]);
  // 7 řádků = týdny: 8 hodnot → 2 sloupce, 8. buňka nahoře ve 2. sloupci
  const weeks = heat([1, 1, 1, 1, 1, 1, 1, 2], 20, 70, { rows: 7, gap: 0 });
  assert.equal(weeks.length, 8);
  assert.deepEqual([weeks[6].x, weeks[6].y], [0, 60]);
  assert.deepEqual([weeks[7].x, weeks[7].y, weeks[7].level], [10, 0, 4]);
});

test("trend", () => {
  assert.deepEqual(trend([1, 3]), { dir: 1, delta: 2 });
  assert.deepEqual(trend([3, 1]), { dir: -1, delta: -2 });
  assert.deepEqual(trend([2, 2, 2]), { dir: 0, delta: 0 });
  assert.deepEqual(trend([1, NaN, 4]), { dir: 1, delta: 3 });
});

test("komponenty: žádná knihovna, CDN, časovač ani animace; kreslí jen viditelné", () => {
  for (const f of ["Sparkline.tsx", "Ring.tsx", "Bars.tsx", "Heat.tsx", "Stat.tsx", "visible.ts", "charts.css"]) {
    const src = readFileSync(new URL(`./${f}`, import.meta.url), "utf8");
    assert.doesNotMatch(src, /https?:\/\//, `${f}: žádná síť / CDN`);
    assert.doesNotMatch(src, /setInterval|setTimeout|requestAnimationFrame/, `${f}: žádný časovač`);
    assert.doesNotMatch(src, /@keyframes|animation\s*:|<animate/, `${f}: žádná animace v klidu`);
    assert.doesNotMatch(src, /from "(d3|chart\.js|recharts|echarts)/, `${f}: žádná knihovna grafů`);
  }
  for (const f of ["Sparkline.tsx", "Ring.tsx", "Bars.tsx", "Heat.tsx"]) {
    const src = readFileSync(new URL(`./${f}`, import.meta.url), "utf8");
    assert.match(src, /useVisible/, `${f}: kreslí jen viditelné`);
    assert.match(src, /from "\.\/geom"/, `${f}: geometrie z geom.ts`);
  }
});
