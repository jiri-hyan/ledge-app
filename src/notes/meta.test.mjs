// Lístky 2.0 — barva a štítky (`meta.ts`) a vazby při psaní (`wiki.ts`): tabulkové testy, zrcadlo Rust testů.
import { test } from "node:test";
import assert from "node:assert/strict";
import { colorHex, frontTags, inlineTags, noteColor, normTag, withoutTag, withTag } from "./meta.ts";
import { applyWiki, wikiQuery } from "./wiki.ts";

test("colorHex: paleta, hex, nesmysl", () => {
  const cases = [["yellow", "#f5c542"], ["Teal", "#2dd4bf"], ["#AABBCC", "#aabbcc"], ["#abc", null], ["neon", null], ["", null], [null, null]];
  for (const [c, want] of cases) assert.equal(colorHex(c), want, String(c));
});

test("noteColor: vlastní barva, pak barva podle štítku", () => {
  const tc = { qa: "red", web: "blue" };
  const cases = [
    [{ color: "green", tags: ["qa"] }, "#4ade80"],
    [{ tags: ["nic", "web", "qa"] }, "#60a5fa"],
    [{ color: "neon", tags: ["qa"] }, "#ef6b6b"],
    [{ tags: [] }, null],
    [null, null],
  ];
  for (const [n, want] of cases) assert.equal(noteColor(n, tc), want, JSON.stringify(n));
  assert.equal(noteColor({ tags: ["qa"] }, null), null);
});

test("normTag: jako Rust meta::norm_tag", () => {
  const cases = [["#Práce", "práce"], ["  qa ", "qa"], ["123", null], ["2026q4", "2026q4"], ["a b", null], ["", null], ["#", null], ["x<y", null], ["ok/sub", "ok/sub"], ["a".repeat(41), null]];
  for (const [raw, want] of cases) assert.equal(normTag(raw), want, raw);
});

test("inlineTags: #štítky v textu bez kódu a nadpisů", () => {
  const body = "# Nadpis\nÚkol #qa a #Web, znovu #qa\n`#kod` [[Nákup]] issue#5\n```\n#ne\n```\n- [ ] #tým (#a/b) #c/ #1 https://x.cz/#kotva";
  assert.deepEqual(inlineTags(body), ["qa", "web", "tým", "a/b", "c"]);
  assert.deepEqual(inlineTags(""), []);
});

test("frontTags / withTag / withoutTag: odebrat jde jen štítek z frontmatteru", () => {
  assert.deepEqual(frontTags(["qa", "web", "tým"], "text #tým"), ["qa", "web"]);
  assert.deepEqual(frontTags(undefined, ""), []);
  const cases = [
    [["qa"], "#Web", ["qa", "web"]],
    [["qa"], "QA", null],
    [["qa"], "a b", null],
    [[], "  ", null],
  ];
  for (const [front, raw, want] of cases) assert.deepEqual(withTag(front, raw), want, raw);
  assert.deepEqual(withoutTag(["qa", "web"], "qa"), ["web"]);
});

test("wikiQuery: rozepsané [[ před kurzorem", () => {
  const cases = [
    ["viz [[Ná", 9, { start: 4, query: "Ná" }],
    ["[[", 2, { start: 0, query: "" }],
    ["[[hotovo]] a", 12, null],
    ["[[a\nb", 5, null],
    ["[[a|b", 5, null],
    ["bez", 3, null],
    ["[[x]] [[y", 9, { start: 6, query: "y" }],
  ];
  for (const [text, caret, want] of cases) assert.deepEqual(wikiQuery(text, caret), want, text);
});

test("applyWiki: vloží [[titulek]] a kurzor za něj", () => {
  assert.deepEqual(applyWiki("viz [[Ná a dál", 4, 8, "Nákup"), { text: "viz [[Nákup]] a dál", caret: 13 });
  // už napsané ]] za kurzorem se nezdvojí
  assert.deepEqual(applyWiki("[[Ná]]", 0, 4, "Nákup"), { text: "[[Nákup]]", caret: 9 });
  // závorky / svislítko v titulku by vazbu rozbily
  assert.deepEqual(applyWiki("[[", 0, 2, "a [b] | c"), { text: "[[a b c]]", caret: 9 });
});
