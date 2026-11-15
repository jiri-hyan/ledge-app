// Odškrtnout úkol v paletě (`tasks.ts`): prefix `x:`, výběr úkolů z indexu jádra (`note_tasks`), kontrola řádku, řádky palety.
//   node --experimental-strip-types --test src/palette/tasks.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { lineStillTask, matchTasks, taskItems, taskQuery } from "./tasks.ts";

test("taskQuery: jen prefix x:", () => {
  assert.equal(taskQuery("x:mléko"), "mléko");
  assert.equal(taskQuery("  X: koupit  mléko "), "koupit  mléko");
  assert.equal(taskQuery("x:"), "");
  for (const q of ["", "x", "xx:a", "mléko x:", "nákup"]) assert.equal(taskQuery(q), null, q);
});

const row = (path, line, text, o = {}) => ({ path, line, text, state: "open", dueMs: null, allDay: false, prio: 0, project: null, noteTitle: path, kind: "", forwarded: false, ...o });
const rows = [
  row("n.md", 2, "mléko", { noteTitle: "Nákup" }),
  row("n.md", 4, "Vejce a máslo", { noteTitle: "Nákup" }),
  row("n.md", 5, "káva", { noteTitle: "Nákup" }),
  row("n.md", 6, "navržený", { state: "proposed", noteTitle: "Nákup" }),
  row("n.md", 7, "odeslaný", { forwarded: true, noteTitle: "Nákup" }),
];

test("matchTasks: jen otevřené a neodeslané, bez diakritiky, všechna slova, titulek poznámky", () => {
  assert.deepEqual(matchTasks(rows, "").map((r) => r.line), [2, 4, 5]);
  assert.deepEqual(matchTasks(rows, "mleko").map((r) => r.line), [2]);
  assert.deepEqual(matchTasks(rows, "vejce maslo").map((r) => r.line), [4]);
  assert.deepEqual(matchTasks(rows, "nakup kava").map((r) => r.line), [5], "titulek + text");
  assert.deepEqual(matchTasks(rows, "navrzeny"), [], "navržený ne");
  assert.deepEqual(matchTasks(rows, "odeslany"), [], "odeslaný ne");
});

test("lineStillTask: otevřený bod téhož textu (bez termínu, priority a důkazu), jinak ne", () => {
  const body = "a\n- [ ] mléko ⏰ 2026-11-14 !\n- [x] hotovo\n1. [ ] káva\r\n- [ ] " + "d".repeat(10);
  assert.equal(lineStillTask(body, 1, "mléko"), true);
  assert.equal(lineStillTask(body, 1, "chleba"), false, "jiný text");
  assert.equal(lineStillTask(body, 2, "hotovo"), false, "hotový");
  assert.equal(lineStillTask(body, 3, "káva"), true, "číslovaný + CR");
  assert.equal(lineStillTask(body, 4, "ddd…"), true, "oříznutý text indexu");
  assert.equal(lineStillTask(body, 0, "a"), false);
  assert.equal(lineStillTask(body, 99, "a"), false);
});

test("taskItems: řádek na úkol, strop na poznámku i celkem, run předá řádek indexu", () => {
  const many = (p) => Array.from({ length: 9 }, (_, i) => row(p, i, `úkol ${i}`));
  const got = [];
  const out = taskItems([...many("a.md"), ...many("b.md"), ...many("c.md")], "ukol", (r) => got.push([r.path, r.line]));
  assert.equal(out.length, 12, "celkový strop");
  assert.equal(out.filter((i) => i.sub === "a.md").length, 5, "strop na poznámku");
  assert.equal(out[0].title, "Odškrtnout: úkol 0");
  out[1].run();
  assert.deepEqual(got, [["a.md", 1]]);
  assert.deepEqual(taskItems([], "x", () => {}), []);
});
