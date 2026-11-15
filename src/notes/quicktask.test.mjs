// Rychlá poznámka → úkol: kdy se uloží řádek úkolu a kdy beze změny.
import { test } from "node:test";
import assert from "node:assert/strict";
import { quickChip, quickTaskLine, worthParsing } from "./quicktask.ts";

const q = (o) => ({ title: "doktor", whenIso: null, priority: 0, assignee: null, line: "- [ ] doktor", ...o });

test("worthParsing: jen jednořádkový neprázdný text, ne seznam", () => {
  const cases = [["v 9:00 doktor !", true], ["", false], ["   ", false], ["a\nb", false], ["a\r\nb", false], ["- [ ] už úkol", false], ["- odrážka", false], ["1. bod", false], ["x".repeat(201), false], ["x".repeat(200), true]];
  for (const [t, want] of cases) assert.equal(worthParsing(t), want, JSON.stringify(t));
});

test("quickTaskLine: čas nebo priorita → řádek, jinak null", () => {
  const timed = q({ whenIso: "2026-11-14 09:00", line: "- [ ] doktor ⏰ 2026-11-14 09:00" });
  const prio = q({ priority: 1, line: "- [ ] doktor !" });
  assert.equal(quickTaskLine("v 9:00 doktor", timed), "- [ ] doktor ⏰ 2026-11-14 09:00");
  assert.equal(quickTaskLine("doktor !", prio), "- [ ] doktor !");
  assert.equal(quickTaskLine("koupit mléko", q({ line: "- [ ] koupit mléko" })), null); // nic nenalezeno = beze změny
  assert.equal(quickTaskLine("v 9:00 doktor", null), null);
  assert.equal(quickTaskLine("v 9:00 doktor", undefined), null);
});

test("quickTaskLine: víceřádkový text a špatný řádek z parseru se nepřevádí", () => {
  const timed = q({ whenIso: "2026-11-14", line: "- [ ] x ⏰ 2026-11-14" });
  assert.equal(quickTaskLine("zítra x\ndruhý řádek", timed), null);
  assert.equal(quickTaskLine("zítra x", q({ whenIso: "2026-11-14", line: "" })), null);
  assert.equal(quickTaskLine("zítra x", q({ whenIso: "2026-11-14", line: "x ⏰" })), null);
  assert.equal(quickTaskLine("zítra x", q({ whenIso: "2026-11-14", line: "- [ ] a\n- [ ] b" })), null);
});

test("quickChip: čas a priorita pod polem, jinak nic", () => {
  assert.deepEqual(quickChip("doktor !!", q({ priority: 2, line: "- [ ] doktor !!" })), { when: null, prio: 2 });
  assert.deepEqual(quickChip("zítra x", q({ whenIso: "2026-11-14", line: "- [ ] x ⏰ 2026-11-14" })), { when: "2026-11-14", prio: 0 });
  assert.equal(quickChip("mléko", q()), null);
});
