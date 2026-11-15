// Psaní otevře paletu (`typeto.ts`): kdy znak paletu otevře a kdy ne.
//   node --experimental-strip-types --test src/palette/typeto.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { isTextTarget, typeToPalette } from "./typeto.ts";

const ev = (key, o = {}) => ({ key, ctrlKey: false, altKey: false, metaKey: false, ...o });
const body = { tagName: "BODY" };

test("holé tělo + tisknutelný znak → znak", () => {
  for (const [k, want] of [["a", "a"], ["Ž", "Ž"], ["?", "?"], [":", ":"], ["0", "0"], ["é", "é"]]) assert.equal(typeToPalette(ev(k), body), want, k);
  assert.equal(typeToPalette(ev("x"), null), "x", "bez cíle = tělo");
});

test("modifikátory, speciální klávesy, mezera a 1–9 paletu neotevřou", () => {
  for (const e of [ev("a", { ctrlKey: true }), ev("a", { altKey: true }), ev("a", { metaKey: true }), ev("a", { isComposing: true }),
    ev("Enter"), ev("Escape"), ev("Tab"), ev("F5"), ev("Dead"), ev("ArrowDown"), ev(" "), ev("1"), ev("9")]) assert.equal(typeToPalette(e, body), null, e.key);
});

test("pole, výběr, contenteditable a ovládací prvky psaní nechají být", () => {
  for (const t of [{ tagName: "INPUT" }, { tagName: "textarea" }, { tagName: "SELECT" }, { tagName: "DIV", isContentEditable: true },
    { tagName: "SPAN", closest: () => ({}) }, { tagName: "BUTTON" }, { tagName: "A" }, { tagName: "DIV" }]) assert.equal(typeToPalette(ev("a"), t), null, t.tagName);
});

test("isTextTarget", () => {
  assert.equal(isTextTarget(null), false);
  assert.equal(isTextTarget(body), false);
  assert.equal(isTextTarget({ tagName: "INPUT" }), true);
  assert.equal(isTextTarget({ tagName: "P", isContentEditable: true }), true);
  assert.equal(isTextTarget({ tagName: "P", closest: () => null }), false);
});
