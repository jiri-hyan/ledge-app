// Gramatika úkolu (`taskline.ts`): tytéž tabulky jako Rust `taskline_tests.rs` (`taskline_cases.json`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { forwardedAt, parseTail, renderTail, splitEvidence, taskLineOf } from "./taskline.ts";

const cases = JSON.parse(readFileSync(new URL("../../src-tauri/src/notes/taskline_cases.json", import.meta.url), "utf8"));

test("parseTail: sdílená tabulka s Rustem + round-trip", () => {
  for (const c of cases) {
    const t = parseTail(c.in);
    assert.equal(t.text, c.text, `text: ${c.in}`);
    assert.equal(t.due, c.due, `due: ${c.in}`);
    assert.equal(t.prio, c.prio, `prio: ${c.in}`);
    assert.equal(splitEvidence(c.in)[1], c.ev, `důkaz: ${c.in}`);
    assert.equal(t.cal, !!c.cal, `cal: ${c.in}`);
    assert.equal(renderTail(t), c.in, `round-trip: ${c.in}`);
  }
});

test("splitEvidence: staré texty beze změny", () => {
  assert.deepEqual(splitEvidence("Dokončit login — důkaz: commit abc123"), ["Dokončit login", "commit abc123"]);
  assert.deepEqual(splitEvidence("bez důkazu - a pak  "), ["bez důkazu - a pak", null]);
  assert.deepEqual(splitEvidence("x - dukaz: y"), ["x", "y"]);
  assert.deepEqual(splitEvidence("x — důkaz:"), ["x", null]);
});

test("round-trip na náhodných složeninách", () => {
  const bits = ["a", "Doktor", " ", " ", "  ", "⏰", "📅", "2026-11-14", "2026-02-30", "09:00", "25:00", "!", "!!", "!!!", "Pozor!", "—", "-", "důkaz:", " — důkaz: ", "é", "🙂"];
  let seed = 12345;
  const rnd = (n) => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed % n; };
  for (let i = 0; i < 20000; i++) {
    let s = "";
    for (let k = rnd(13); k > 0; k--) s += bits[rnd(bits.length)];
    const t = parseTail(s);
    assert.equal(renderTail(t), s, JSON.stringify(s));
    assert.equal(t.head.length + t.evidenceRaw.length + t.trail.length, s.length, JSON.stringify(s));
  }
});

const itemCases = JSON.parse(readFileSync(new URL("../../src-tauri/src/notes/taskitem_cases.json", import.meta.url), "utf8"));

test("taskLineOf: sdílená tabulka s Rustem (md::list_item + task_state)", () => {
  for (const c of itemCases) {
    const t = taskLineOf(c.in);
    if (!c.task) {
      assert.equal(t, null, `není úkol: ${JSON.stringify(c.in)}`);
      continue;
    }
    assert.deepEqual(t, { indent: c.indent, marker: c.marker, state: c.state, rest: c.rest }, JSON.stringify(c.in));
  }
});

test("taskLineOf: CR na konci řádku se ořízne", () => {
  assert.deepEqual(taskLineOf("- [ ] a\r"), { indent: 0, marker: "-", state: "open", rest: "a" });
});

const markCases = JSON.parse(readFileSync(new URL("../../src-tauri/src/notes/sent_mark_cases.json", import.meta.url), "utf8"));

test("forwardedAt: tabulka značky odeslání sdílená s Rustem (sent_mark_cases.json)", () => {
  for (const l of markCases.marked) assert.equal(forwardedAt(["- [ ] a", l], 0), true, `marked: ${JSON.stringify(l)}`);
  for (const l of markCases.plain) assert.equal(forwardedAt(["- [ ] a", l], 0), false, `plain: ${JSON.stringify(l)}`);
  for (const b of markCases.bodies) {
    const lines = b.body.split("\n");
    const got = lines.flatMap((l, i) => (taskLineOf(l)?.state === "open" ? [forwardedAt(lines, i)] : []));
    assert.deepEqual(got, b.sent, JSON.stringify(b.body));
  }
});
