// Hlavička šuplíku (`headModel.ts`): doplnění configu, pořadí dílů, řazení štítků do jedné řady.
//   node --experimental-strip-types --test src/layout/headModel.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { fitCount, fullOrder, handAngles, isDefaultHead, keepChip, movePart, normalizeHead, normalizeTabs, shortDate, visibleParts, withHidden, withOrder, withSeconds } from "./headModel.ts";

test("starý config = dnešní hlavička", () => {
  assert.deepEqual(normalizeHead(undefined), { clock: "big", chips: "auto" });
  assert.deepEqual(normalizeHead({ clock: "nesmysl", order: "x", hidden: [1] }), { clock: "big", chips: "auto" });
  assert.deepEqual(normalizeTabs(undefined), { style: "segment" });
  assert.deepEqual(visibleParts(undefined), ["clock", "chips", "tabs", "edit", "note", "search", "settings", "close"]);
  assert.ok(isDefaultHead({ clock: "big", chips: "auto" }));
});

test("pořadí a vypnuté díly", () => {
  const cases = [
    [{ order: ["tabs", "clock"] }, ["tabs", "clock", "chips", "edit", "note", "search", "settings", "close"]],
    [{ order: ["close", "clock", "clock", "bad"] }, ["clock", "chips", "tabs", "edit", "note", "search", "settings", "close"]],
    [{ hidden: ["note", "settings", "close", "chips"] }, ["clock", "tabs", "edit", "search", "settings", "close"]],
  ];
  for (const [h, want] of cases) assert.deepEqual(visibleParts(normalizeHead(h)), want, JSON.stringify(h));
});

test("přesun dílu: ✕ zůstává vpravo, výchozí pořadí se nezapisuje", () => {
  assert.deepEqual(movePart(undefined, "settings", 1).at(-1), "close");
  assert.equal(movePart(undefined, "close", -1).at(-1), "close");
  assert.deepEqual(movePart(undefined, "tabs", -1).slice(0, 3), ["clock", "tabs", "chips"]);
  const h = withOrder({ clock: "big", chips: "auto" }, movePart(undefined, "tabs", -1));
  assert.ok(h.order);
  assert.equal(withOrder(h, fullOrder(undefined)).order, undefined);
  assert.equal(withHidden({ clock: "big", chips: "auto" }, "close", true).hidden, undefined);
  assert.equal(withHidden(withHidden({ clock: "big", chips: "auto" }, "note", true), "note", false).hidden, undefined);
});

test("štítky v jedné řadě: kolik se vejde a zbytek jde do +N", () => {
  // [šířky, mezera, místo, šířka +N, výsledek]
  const cases = [
    [[], 8, 100, 30, 0],
    [[50, 50], 8, 108, 30, 2],
    [[50, 50], 8, 107, 30, 1],
    [[50, 50, 50], 8, 200, 30, 3],
    [[50, 50, 50], 8, 150, 30, 2],
    [[50, 50, 50], 8, 20, 30, 0],
    [[50, 50, 50], 8, 96, 30, 1],
    [[60, 40, 40], 8, 150, 30, 2],
  ];
  for (const [w, g, a, m, want] of cases) assert.equal(fitCount(w, g, a, m), want, JSON.stringify([w, a]));
});

test("nulové štítky a hodiny", () => {
  assert.equal(keepChip(0, false), false);
  assert.equal(keepChip(0, undefined), false);
  assert.equal(keepChip(0, true), true);
  assert.equal(keepChip(3, false), true);
  assert.deepEqual(handAngles(new Date(2026, 10, 2, 3, 30, 0)), { h: 105, m: 180, s: 0 });
  assert.equal(shortDate("po", new Date(2026, 10, 2)), "po 2. 11.");
  assert.equal(withSeconds("20:02", new Date(2026, 10, 2, 20, 2, 5)), "20:02:05");
});

import { chipLevels, hideByPriority, pickLevel } from "./headModel.ts";

test("auto zhušťuje postupně, ruční styl jen sebe; sloučená se zkracuje místo schování", () => {
  assert.deepEqual(chipLevels("auto").map((l) => `${l.style}${l.shrink}`), ["pills0", "merged0", "merged1", "dots0", "icons0"]);
  assert.deepEqual(chipLevels("pills").map((l) => l.style), ["pills"]);
  assert.deepEqual(chipLevels("merged").map((l) => l.shrink), [0, 1, 2]);
  assert.equal(chipLevels("icons").length, 1);
});

test("pickLevel: první stupeň, kam se vejdou všechny", () => {
  const W = { pills0: [100, 100, 100], merged0: [90, 90, 90], merged1: [60, 60, 60], dots0: [40, 40, 40], icons0: [30, 30, 30] };
  const run = (avail) => {
    const r = pickLevel(chipLevels("auto"), (l) => ({ widths: W[`${l.style}${l.shrink}`], gap: l.style === "merged" ? 0 : 8 }), avail);
    return `${r.level.style}${r.level.shrink}${r.fits ? "" : "!"}`;
  };
  // [místo, výsledek]
  for (const [a, want] of [[400, "pills0"], [300, "merged0"], [200, "merged1"], [150, "dots0"], [110, "icons0"], [50, "icons0!"]]) assert.equal(run(a), want, String(a));
});

test("schovávání do +N podle důležitosti", () => {
  // pořadí: čeká(0), pracuje(2), neuloženo(4), běží(5)
  const w = [60, 60, 60, 60], p = [0, 2, 4, 5];
  const cases = [
    [300, []],
    [250, [3]],
    [200, [2, 3]],
    [140, [1, 2, 3]],
    [20, [0, 1, 2, 3]],
  ];
  for (const [avail, want] of cases) assert.deepEqual([...hideByPriority(w, p, 8, avail, 30)].sort(), want, String(avail));
  // méně důležitý vpředu v DOM se schová dřív než důležitý vzadu
  assert.deepEqual([...hideByPriority([60, 60], [5, 0], 8, 100, 30)], [0]);
});
