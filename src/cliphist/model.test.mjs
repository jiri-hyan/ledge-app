// Tabulkové testy historie schránky v paletě (`model.ts`): prefix, řádky (vypnuto / prázdno /
// položky / vymazat / hledání), podtitul, výběr po smazání.
//   node --experimental-strip-types --test src/cliphist/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CLIP_PREFIX, clampSel, clipQuery, clipRows, clipSub } from "./model.ts";

const NOW = 1_000_000_000;
const e = (id, preview, o = {}) => ({ id, preview, chars: preview.length, lines: 1, atMs: NOW - 5 * 60_000, pinned: false, ...o });

test("prefix v:", () => {
  const cases = [
    ["v:", ""],
    ["v:adresa", "adresa"],
    ["  V:git", "git"],
    ["v: mezera", " mezera"],
    ["v", null],
    ["v adresa", null],
    ["vv:", null],
    [":srdce", null],
    ["", null],
  ];
  for (const [q, want] of cases) assert.equal(clipQuery(q), want, q);
  assert.equal(CLIP_PREFIX, "v:");
});

test("řádky: načítání, vypnuto, prázdno", () => {
  assert.deepEqual(clipRows(undefined, "", NOW), []);
  const off = clipRows({ enabled: false, items: [] }, "", NOW);
  assert.deepEqual(off.map((r) => r.item.kind), ["off", "settings"], "zapnout jde přímo z palety");
  const empty = clipRows({ enabled: true, items: [] }, "x", NOW);
  assert.deepEqual(empty.map((r) => r.item.kind), ["empty", "settings"]);
});

test("řádky: pořadí jádra + vymazat jen s nepřipnutými", () => {
  const list = { enabled: true, items: [e(3, "pin", { pinned: true }), e(2, "druhý"), e(1, "první")] };
  const rows = clipRows(list, "", NOW);
  assert.deepEqual(rows.map((r) => r.item.id), ["clip:3", "clip:2", "clip:1", "clip:clear", "clip:settings"]);
  assert.equal(rows[0].item.entry.id, 3);
  const onlyPinned = clipRows({ enabled: true, items: [e(3, "pin", { pinned: true })] }, "", NOW);
  assert.deepEqual(onlyPinned.map((r) => r.item.kind), ["entry", "settings"], "nic ke smazání = bez řádku Vymazat");
});

test("řádky: hledání bez diakritiky, bez řádku Vymazat", () => {
  const list = { enabled: true, items: [e(1, "Příčná 12, Praha"), e(2, "git push origin main"), e(3, "npm run dev")] };
  const cases = [
    ["pricna", [1]],
    ["git", [2]],
    ["zzz", []],
  ];
  for (const [q, want] of cases) assert.deepEqual(clipRows(list, q, NOW).map((r) => r.item.entry.id), want, q);
});

test("podtitul: stáří, řádky, připnuto", () => {
  assert.match(clipSub(e(1, "a"), NOW), /^před 5 min$/);
  assert.equal(clipSub(e(1, "a", { lines: 3, pinned: true }), NOW), "před 5 min · 3 řádky · připnuto");
  assert.equal(clipSub(e(1, "a", { lines: 7, atMs: NOW }), NOW), "teď · 7 řádků");
});

test("výběr po smazání", () => {
  const cases = [
    [0, 3, 0],
    [2, 3, 2],
    [3, 3, 2],
    [5, 1, 0],
    [1, 0, 0],
  ];
  for (const [sel, n, want] of cases) assert.equal(clampSel(sel, n), want, `${sel}/${n}`);
});
