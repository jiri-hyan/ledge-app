// @vitest-environment node
// Tabulkové testy varianty Bento karty Poznámky (`bento.ts`).
//   node --experimental-strip-types --test src/notes/bento.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { bentoLines, heroOf, TEMPLATES } from "./bento.ts";

const n = (path, done = false) => ({ path, done, title: path });

test("velká poznámka: připnutá > první nesplněná > první > nic", () => {
  const pins = { open: [{ path: "C:/N/c.md", through: false }] };
  const rows = [
    [[n("C:/N/a.md", true), n("C:/N/b.md"), n("C:/N/c.md")], pins, "C:/N/c.md"],
    [[n("C:/N/a.md", true), n("C:/N/b.md")], null, "C:/N/b.md"],
    [[n("C:/N/a.md", true)], undefined, "C:/N/a.md"],
    [[], pins, null],
  ];
  for (const [items, p, want] of rows) assert.equal(heroOf(items, p)?.path ?? null, want);
});

test("náhled těla: nadpis bez #, úkoly označené, prázdné řádky pryč, strop", () => {
  assert.deepEqual(bentoLines("# Nákup\n\n- [ ] mléko\r\n- [x] chléb\ntext\nnavíc", 4), [
    { text: "Nákup", task: false },
    { text: "- [ ] mléko", task: true },
    { text: "- [x] chléb", task: true },
    { text: "text", task: false },
  ]);
  assert.deepEqual(bentoLines(""), []);
  assert.deepEqual(bentoLines(null), []);
});

test("šablony: jedinečná id a výchozí kategorie", () => {
  assert.equal(new Set(TEMPLATES.map((x) => x.id)).size, TEMPLATES.length);
  for (const x of TEMPLATES) assert.ok(["napad", "feedback", "ukol", "info"].includes(x.kind), x.id);
});
