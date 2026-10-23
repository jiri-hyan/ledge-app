// Tabulkové testy UI aktualizací (`model.ts`): bezpečné poznámky, poctivý průběh, akce panelu.
//   node --experimental-strip-types --test src/update/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { action, inline, mb, parseNotes, progressPct } from "./model.ts";

test("inline: kód, tučně, odkaz jen jako text, HTML pryč", () => {
  const cases = [
    ["prosté", [{ k: "t", s: "prosté" }]],
    ["oprava `update.rs` hotová", [{ k: "t", s: "oprava " }, { k: "code", s: "update.rs" }, { k: "t", s: " hotová" }]],
    ["**nové** volby", [{ k: "b", s: "nové" }, { k: "t", s: " volby" }]],
    ["viz [web](https://example.com/x)", [{ k: "t", s: "viz web" }]],
    ["<script>alert(1)</script>ahoj", [{ k: "t", s: "alert(1)ahoj" }]],
    ["<img src=x onerror=alert(1)>", []],
    ["a < b > c", [{ k: "t", s: "a  c" }]],
    ["`<b>`", [{ k: "t", s: "``" }]],
  ];
  for (const [src, want] of cases) assert.deepEqual(inline(src), want, src);
});

test("parseNotes: nadpisy, odrážky, odstavce; prázdné a HTML řádky zmizí", () => {
  const md = "Změny od v0.2.14:\r\n\n## Nové\n- feat(core): tiché stažení\n* fix: `x`\n1. první\n<div></div>\n\n";
  assert.deepEqual(parseNotes(md), [
    { k: "p", parts: [{ k: "t", s: "Změny od v0.2.14:" }] },
    { k: "h", parts: [{ k: "t", s: "Nové" }] },
    { k: "li", parts: [{ k: "t", s: "feat(core): tiché stažení" }] },
    { k: "li", parts: [{ k: "t", s: "fix: " }, { k: "code", s: "x" }] },
    { k: "li", parts: [{ k: "t", s: "první" }] },
  ]);
  assert.deepEqual(parseNotes(""), []);
  assert.equal(parseNotes("- x\n".repeat(500)).length, 200);
});

test("průběh jen ze skutečných bajtů; bez velikosti neurčitý", () => {
  const cases = [
    [{ state: "downloading", got: 0, total: 1000 }, 0],
    [{ state: "downloading", got: 505, total: 1000 }, 50],
    [{ state: "downloading", got: 1200, total: 1000 }, 100],
    [{ state: "downloading", got: 500, total: null }, null],
    [{ state: "downloading", got: 500, total: 0 }, null],
    [{ state: "ready" }, null],
    [{ state: "idle" }, null],
  ];
  for (const [f, want] of cases) assert.equal(progressPct(f), want, JSON.stringify(f));
  assert.equal(mb(3 * 1024 * 1024 + 300 * 1024), "3.3");
  assert.equal(mb(-1), "0.0");
});

test("akce panelu podle stavu stažení", () => {
  const cases = [
    [{ state: "ready" }, "restart"],
    [{ state: "downloading", got: 1, total: null }, "downloading"],
    [{ state: "failed", duvod: "offline" }, "retry"],
    [{ state: "idle" }, "install"],
  ];
  for (const [f, want] of cases) assert.equal(action(f), want, f.state);
});
