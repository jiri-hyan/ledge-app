// Hlídač (zdrojový, `App.tsx` nejde v Node spustit): otevření šuplíku bez plánu morfu musí před `revealContent` zrušit staré
// animace přes `preHide` — `concealContent` (zavření) nechává `fill: "forwards"`, takže by obsah po naskočení zůstal průhledný.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const app = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

test("prepare: větev bez plánu volá preHide před revealContent", () => {
  const i = app.indexOf("revealContent(content, 40)");
  assert.ok(i > 0, "větev bez plánu existuje");
  const before = app.slice(Math.max(0, i - 400), i);
  assert.ok(before.includes("preHide(content)"), "preHide(content) před revealContent(content, 40)");
});

test("concealContent zůstává forwards (proto je preHide nutný)", () => {
  const motion = readFileSync(new URL("./motion.ts", import.meta.url), "utf8");
  const fn = motion.slice(motion.indexOf("export function concealContent"));
  assert.ok(fn.includes('fill: "forwards"'));
});
