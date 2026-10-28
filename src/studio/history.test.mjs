// „Platí hned“ (kolo 24): rozdíl verzí, časová osa (slučování, ruční návrat, Vrátit jednu / poslední),
// lidské popisky a kdy smí autosave zapisovat.
//   node --experimental-strip-types --test src/studio/history.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { MERGE_MS, MAX_ENTRIES, canSave, diffPaths, emptyHistory, entryText, isTextEntry, nearestLabel, record, same, setAt, undoEntry, undoLast, valueText } from "./history.ts";

const W = { on: "zapnuto", off: "vypnuto", empty: "nic", items: (n) => `${n} položek`, edited: "upraveno", more: (n) => `a ${n} další` };

test("diffPaths: do objektů sestupuje, pole a hodnoty bere celé, pořadí klíčů nevadí", () => {
  const cases = [
    [{ a: 1 }, { a: 1 }, []],
    [{ a: 1, b: { c: 2 } }, { b: { c: 2 }, a: 1 }, []],
    [{ strip: { look: "classic" } }, { strip: { look: "island" } }, [{ path: "strip.look", before: "classic", after: "island" }]],
    [{ projects: [1, 2] }, { projects: [2, 1] }, [{ path: "projects", before: [1, 2], after: [2, 1] }]],
    [{}, { autostart: true }, [{ path: "autostart", before: undefined, after: true }]],
    [{ x: { y: 1 } }, { x: 5 }, [{ path: "x", before: { y: 1 }, after: 5 }]],
  ];
  for (const [a, b, want] of cases) assert.deepEqual(diffPaths(a, b), want, JSON.stringify([a, b]));
  assert.ok(same({ a: 1, b: 2 }, { b: 2, a: 1 }));
});

test("setAt: kopie, založí chybějící, undefined maže", () => {
  const o = { a: { b: 1 } };
  assert.deepEqual(setAt(o, "a.b", 2), { a: { b: 2 } });
  assert.deepEqual(o, { a: { b: 1 } }, "původní beze změny");
  assert.deepEqual(setAt({}, "x.y.z", true), { x: { y: { z: true } } });
  assert.deepEqual(setAt({ a: 1, b: 2 }, "b", undefined), { a: 1 });
});

test("record: nová položka, slučování do MERGE_MS, ruční návrat smaže, beze změny nic", () => {
  const c0 = { strip: { look: "classic", opacity: 0.8 } };
  const c1 = { strip: { look: "island", opacity: 0.8 } };
  const c2 = { strip: { look: "line", opacity: 0.8 } };
  const h = record(emptyHistory(), c0, c1, 1000);
  assert.equal(h.entries.length, 1);
  assert.equal(record(h, c1, c1, 1100), h, "beze změny");
  const merged = record(h, c1, c2, 1000 + MERGE_MS);
  assert.equal(merged.entries.length, 1);
  assert.deepEqual(merged.entries[0].changes, [{ path: "strip.look", before: "classic", after: "line" }]);
  assert.equal(record(h, c1, c2, 1001 + MERGE_MS).entries.length, 2, "pozdě = nová položka");
  assert.equal(record(h, c1, c0, 99999).entries.length, 0, "ruční návrat poslední změny ji smaže");
  assert.equal(record(h, c1, { strip: { look: "island", opacity: 0.5 } }, 1200).entries.length, 2, "jiná volba = nová položka");
  let big = emptyHistory();
  for (let i = 0; i < MAX_ENTRIES + 5; i++) big = record(big, { n: i }, { n: i + 1, ["k" + (i % 2)]: i }, i * 10000);
  assert.equal(big.entries.length, MAX_ENTRIES);
});

test("undoEntry / undoLast: vrátí jen volby té položky", () => {
  const c0 = { a: 1, b: 1 };
  const c1 = { a: 2, b: 1 };
  const c2 = { a: 2, b: 2 };
  let h = record(emptyHistory(), c0, c1, 0);
  h = record(h, c1, c2, 10000);
  const r = undoEntry(h, h.entries[0].id, c2);
  assert.deepEqual(r.cfg, { a: 1, b: 2 }, "b zůstane");
  assert.equal(r.history.entries.length, 1);
  assert.deepEqual(undoLast(h, c2).cfg, { a: 2, b: 1 });
  assert.equal(undoEntry(h, 999, c2), null);
  assert.equal(undoLast(emptyHistory(), c2), null);
  assert.deepEqual(undoEntry(record(emptyHistory(), {}, { x: true }, 0), 1, { x: true }).cfg, {}, "nový klíč se smaže");
});

test("popisky: hodnota, jedna změna, víc voleb, nejbližší popsaná volba", () => {
  const named = (path, v) => (path === "strip.look" ? ({ classic: "Klasický", island: "Ostrůvek" })[v] ?? null : null);
  const cases = [
    [true, "zapnuto"], [false, "vypnuto"], [null, "nic"], ["", "nic"], [0.825, "0.83"], [[1, 2, 3], "3 položek"], [{ a: 1 }, "upraveno"],
    ["x".repeat(40), `${"x".repeat(27)}…`],
  ];
  for (const [v, want] of cases) assert.equal(valueText(v, W), want, JSON.stringify(v));
  const label = (p) => ({ "strip.look": "Vzhled proužku", a: "A", b: "B" })[p] ?? p;
  assert.equal(entryText({ id: 1, at: 0, changes: [{ path: "strip.look", before: "classic", after: "island" }] }, label, W, named), "Vzhled proužku: Klasický → Ostrůvek");
  assert.equal(entryText({ id: 1, at: 0, changes: [{ path: "a", before: 1, after: 2 }, { path: "b", before: 1, after: 2 }] }, label, W), "A a 1 další");
  assert.equal(entryText({ id: 1, at: 0, changes: [{ path: "layout", before: { p: 1 }, after: { p: 2 } }] }, label, W), "layout");
  const has = (id) => ["appearance.strip", "strip"].includes(id);
  assert.equal(nearestLabel("appearance.strip.size", has), "appearance.strip");
  assert.equal(nearestLabel("strip.custom.text", has), "strip");
  assert.equal(nearestLabel("nic.dalsiho", has), "nic.dalsiho");
});

test("canSave a textová pole", () => {
  const ok = { changed: true, typing: false, invalid: false, conflict: false, saving: false };
  assert.equal(canSave(ok), true);
  for (const k of ["typing", "invalid", "conflict", "saving"]) assert.equal(canSave({ ...ok, [k]: true }), false, k);
  assert.equal(canSave({ ...ok, changed: false }), false);
  const cases = [["INPUT", "text", true], ["INPUT", null, true], ["INPUT", "number", true], ["TEXTAREA", null, true], ["INPUT", "checkbox", false], ["INPUT", "range", false], ["BUTTON", null, false], ["SELECT", null, false]];
  for (const [tag, type, want] of cases) assert.equal(isTextEntry(tag, type), want, `${tag} ${type}`);
});
