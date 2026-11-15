// Pomocné funkce „Poslat do {název}“ (`model.ts`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { defaultLines, needsDecision, priorityBand, summarize, toggled } from "./model.ts";

const item = (line, assignee, sent = false) => ({ line, title: `bod ${line}`, assignee, priority: null, sent, requestId: `r${line}` });
const plan = (items) => ({ mode: "tasks", hasTasks: true, noteTitle: "n", noteRequestId: "x", noteSent: false, items, truncated: false });

test("předem se zaškrtnou jen neodeslané body s jasným řešitelem", () => {
  const p = plan([
    item(0, { kind: "unset" }),
    item(1, { kind: "resolved", email: "a@x.cz", name: "A" }),
    item(2, { kind: "ambiguous", query: "Tomáš", names: ["Tomáš A", "Tomáš B"] }),
    item(3, { kind: "unknown", query: "Pepa" }),
    item(4, { kind: "unset" }, true),
  ]);
  assert.deepEqual(defaultLines(p), [0, 1]);
  assert.deepEqual(p.items.map(needsDecision), [false, false, true, true, false]);
});

test("priorita propojené aplikace 1–5 do pásma", () => {
  assert.deepEqual([null, 1, 2, 3, 4, 5].map(priorityBand), [null, "low", "low", "mid", "mid", "high"]);
});

test("souhrn odeslání", () => {
  const ok = (line, duplicate = false) => ({ line, ok: true, id: "t", url: null, error: null, duplicate });
  const bad = (line, error) => ({ line, ok: false, id: null, url: null, error, duplicate: false });
  assert.deepEqual(summarize([]), { ok: 0, failed: 0, duplicate: 0, error: null });
  assert.deepEqual(summarize([ok(0), ok(1, true), bad(2, "Nelze"), bad(3, "Jiná")]), { ok: 2, failed: 2, duplicate: 1, error: "Nelze" });
});

test("přepnutí zaškrtnutí drží pořadí", () => {
  assert.deepEqual(toggled([0, 4], 2), [0, 2, 4]);
  assert.deepEqual(toggled([0, 2, 4], 2), [0, 4]);
});
