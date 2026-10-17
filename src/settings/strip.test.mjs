// Tabulkové testy editoru proužku (`strip.ts`), bez DOM.
//   node --experimental-strip-types --test src/settings/strip.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { STRIP_COLORS, STRIP_ITEM_IDS, clipCustom, customLen, previewSegments, withColor, withQuiet } from "./strip.ts";

test("položky a barvy se shodují s Rustem (stripconf.rs)", () => {
  const src = readFileSync(new URL("../../src-tauri/src/stripconf.rs", import.meta.url), "utf8");
  const variants = (name) => {
    const body = src.slice(src.indexOf(`pub enum ${name} {`), src.indexOf("}", src.indexOf(`pub enum ${name} {`)));
    return [...body.matchAll(/^\s*([A-Z][A-Za-z]*),\s*$/gm)].map((m) => m[1][0].toLowerCase() + m[1].slice(1));
  };
  assert.deepEqual(variants("StripItem"), [...STRIP_ITEM_IDS]);
  assert.deepEqual(variants("StripColor"), [...STRIP_COLORS]);
});

test("barva: změna jednoho klíče, null = výchozí", () => {
  const before = { clock: "teal" };
  const cases = [
    [undefined, "custom", "violet", { custom: "violet" }],
    [before, "custom", "red", { clock: "teal", custom: "red" }],
    [before, "clock", null, {}],
    [before, "clock", "amber", { clock: "amber" }],
  ];
  for (const [colors, id, c, want] of cases) assert.deepEqual(withColor(colors, id, c), want, `${id} ${c}`);
  assert.deepEqual(before, { clock: "teal" }, "vstup netknutý");
});

test("v klidu skryté: přidat, odebrat, bez duplicit", () => {
  const cases = [
    [undefined, "clock", true, ["clock"]],
    [["clock"], "clock", true, ["clock"]],
    [["clock", "mode"], "clock", false, ["mode"]],
    [["mode"], "limits", true, ["mode", "limits"]],
    [[], "mode", false, []],
  ];
  for (const [q, id, on, want] of cases) assert.deepEqual(withQuiet(q, id, on), want, `${JSON.stringify(q)} ${id} ${on}`);
});

test("vlastní text: délka ve znacích, emoji = jeden znak", () => {
  assert.equal(customLen("🎯 Fokus"), 7);
  assert.equal(customLen(undefined), 0);
  assert.equal(clipCustom("🎯".repeat(30)), "🎯".repeat(24));
  assert.equal(clipCustom("krátké"), "krátké");
});

test("náhled: pořadí, prázdné vynechá, klid vs. dění", () => {
  const strip = { items: ["mode", "clock", "running", "custom"], colors: { custom: "violet" }, quiet: ["clock"] };
  const sample = { mode: "Práce", clock: "08:31", running: "", custom: "🎯" };
  const calm = previewSegments(strip, sample, false);
  assert.deepEqual(calm.map((s) => [s.id, s.hidden, s.color]), [["mode", false, null], ["clock", true, null], ["custom", false, "violet"]]);
  assert.deepEqual(previewSegments(strip, sample, true).map((s) => s.hidden), [false, false, false]);
  assert.deepEqual(previewSegments({}, sample, false), []);
});
