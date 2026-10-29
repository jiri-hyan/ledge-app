// Seskupení poznámek: tabulkové případy (nic / projekt / kategorie / datum / připnuté), prázdné skupiny nevznikají.
import { test } from "node:test";
import assert from "node:assert/strict";
import { dateBucket, groupByOf, groupNotes, groupStyleOf } from "./group.ts";

// čtvrtek 29. 10. 2026 12:00
const now = new Date(2026, 9, 29, 12, 0).getTime();
const at = (m, d, h = 9) => new Date(2026, m - 1, d, h).getTime();
const n = (id, o = {}) => ({ path: id, title: id, project: null, kind: "info", createdMs: at(10, 29), done: false, ...o });
const ctx = {
  now,
  project: (id) => (id ? `P:${id}` : "Obecné"),
  kind: (id) => `K:${id}`,
  pinned: (x) => x.path === "b",
  text: { today: "Dnes", week: "Tento týden", older: "Starší", pinned: "Připnuté", rest: "Ostatní" },
};
const shape = (gs) => gs.map((g) => `${g.label}=${g.items.map((x) => x.path).join("")}`).join("|");

test("dateBucket: dnes, pondělí tohoto týdne, starší", () => {
  const cases = [
    [at(10, 29, 0), "today"],
    [at(10, 30), "today"], // budoucí = dnes
    [at(10, 28, 23), "week"],
    [at(10, 26, 0), "week"], // pondělí
    [at(10, 25, 23), "older"], // neděle před týdnem
    [at(1, 1), "older"],
  ];
  for (const [ms, want] of cases) assert.equal(dateBucket(ms, now), want, new Date(ms).toISOString());
});

test("groupNotes: režimy", () => {
  const items = [n("a", { project: "x" }), n("b", { kind: "napad", createdMs: at(10, 27) }), n("c", { project: "x", createdMs: at(9, 1) })];
  const cases = [
    ["none", "=abc"],
    ["project", "P:x=ac|Obecné=b"],
    ["kind", "K:info=ac|K:napad=b"],
    ["date", "Dnes=a|Tento týden=b|Starší=c"],
    ["pinned", "Připnuté=b|Ostatní=ac"],
  ];
  for (const [by, want] of cases) assert.equal(shape(groupNotes(items, by, ctx)), want, by);
  assert.deepEqual(groupNotes([], "date", ctx), []);
  assert.deepEqual(groupNotes([], "none", ctx), []);
});

test("groupNotes: datum řadí Dnes, Týden, Starší i když vstup není po pořádku; prázdné skupiny nejsou", () => {
  const items = [n("c", { createdMs: at(1, 1) }), n("a")];
  assert.equal(shape(groupNotes(items, "date", ctx)), "Dnes=a|Starší=c");
});

test("groupByOf / groupStyleOf: neznámé = výchozí", () => {
  assert.equal(groupByOf("date"), "date");
  assert.equal(groupByOf("zzz"), "none");
  assert.equal(groupByOf(undefined), "none");
  assert.equal(groupStyleOf("cards"), "cards");
  assert.equal(groupStyleOf(3), "line");
});
