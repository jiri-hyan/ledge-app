// Tabulkové testy editoru proužku (`strip.ts`), bez DOM.
//   node --experimental-strip-types --test src/settings/strip.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { STRIP_COLORS, STRIP_ITEM_IDS, STRIP_OPACITY, clipCustom, customLen, monitorOptions, previewSegments, withColor, withQuiet } from "./strip.ts";

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

test("výchozí krytí proužku v UI = Rust (stripconf.rs)", () => {
  const src = readFileSync(new URL("../../src-tauri/src/stripconf.rs", import.meta.url), "utf8");
  const m = src.match(/opacity:\s*([0-9.]+),/);
  assert.ok(m, "Default má opacity");
  assert.equal(STRIP_OPACITY, Number(m[1]));
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

test("volba monitoru: Hlavní / Všechny / připojené, neznámá uložená hodnota navíc", () => {
  const lab = { primary: "Hlavní", all: "Všechny", dev: (m) => `Monitor ${m.n} · ${m.w}×${m.h}`, gone: "Nepřipojený" };
  const list = [
    { id: "dev:A;1920x1080@0,0", n: 1, w: 1920, h: 1080, primary: true },
    { id: "dev:B;2560x1440@1920,0", n: 2, w: 2560, h: 1440, primary: false },
  ];
  const vals = (cur) => monitorOptions(list, cur, lab).map((o) => o.v);
  assert.deepEqual(vals("primary"), ["primary", "all", list[0].id, list[1].id]);
  assert.deepEqual(vals("all"), ["primary", "all", list[0].id, list[1].id]);
  assert.deepEqual(vals(list[1].id), ["primary", "all", list[0].id, list[1].id]);
  assert.deepEqual(vals("dev:Z;800x600@0,0").slice(-1), ["dev:Z;800x600@0,0"], "odpojený monitor zůstane vybraný");
  assert.equal(monitorOptions(list, "2", lab).at(-1).label, "Nepřipojený", "starý index");
  assert.equal(monitorOptions(list, "", lab).length, 4);
  assert.deepEqual(monitorOptions([], "primary", lab).map((o) => o.v), ["primary", "all"], "bez monitorů (IPC selhalo)");
  assert.equal(monitorOptions(list, list[1].id, lab)[3].label, "Monitor 2 · 2560×1440");
});
