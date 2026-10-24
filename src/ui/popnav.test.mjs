// Tabulkové testy logiky menu (`popnav.ts`): kruhové šipky, Home / End, prázdné menu, kdo smí zavřít a navigovat.
//   node --experimental-strip-types --test src/ui/popnav.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { menuKey, navTarget } from "./popnav.ts";

test("navTarget: šipky kruhově, Home / End, fokus mimo položky", () => {
  const rows = [
    ["ArrowDown", -1, 3, 0],
    ["ArrowDown", 0, 3, 1],
    ["ArrowDown", 2, 3, 0],
    ["ArrowUp", -1, 3, 2],
    ["ArrowUp", 0, 3, 2],
    ["ArrowUp", 2, 3, 1],
    ["Home", 2, 3, 0],
    ["End", 0, 3, 2],
    ["ArrowDown", 0, 1, 0],
    ["ArrowUp", 0, 1, 0],
  ];
  for (const [key, cur, count, want] of rows) assert.equal(navTarget(key, cur, count), want, `${key} ${cur}/${count}`);
});

test("navTarget: prázdné menu a cizí klávesy nedělají nic", () => {
  assert.equal(navTarget("ArrowDown", -1, 0), null);
  assert.equal(navTarget("Home", 0, 0), null);
  assert.equal(navTarget("a", 0, 3), null);
  assert.equal(navTarget("Enter", 0, 3), null);
});

test("menuKey: Esc a Tab zavírají vždy, šipky jen když patří menu", () => {
  const f = (inside, free, typing) => ({ inside, free, typing });
  assert.equal(menuKey("Escape", f(false, false, true)), "close");
  assert.equal(menuKey("Tab", f(true, false, false)), "close");
  assert.equal(menuKey("ArrowDown", f(true, false, false)), "nav");
  assert.equal(menuKey("ArrowUp", f(false, true, false)), "nav");
  // fokus v jiném poli (editor poznámky): šipky patří jemu
  assert.equal(menuKey("ArrowDown", f(false, false, false)), null);
  // jezdec / pole uvnitř menu si šipky nechá
  assert.equal(menuKey("ArrowDown", f(true, false, true)), null);
  assert.equal(menuKey("Enter", f(true, false, false)), null);
});

test("zapojení: všechna menu jedou na PopMenu, žádné vlastní posluchače Esc / pointerdown", async () => {
  const { readFileSync } = await import("node:fs");
  const files = ["../cards/CardMenu.tsx", "../notes/NoteMenu.tsx", "../notes/RowMenu.tsx", "../tilelook/TileMenu.tsx", "../media/VariantPicker.tsx", "../pin/PinMenu.tsx"];
  for (const f of files) {
    const s = readFileSync(new URL(f, import.meta.url), "utf8");
    assert.ok(s.includes("PopMenu"), `${f}: menu přes PopMenu`);
    assert.ok(!s.includes('"pointerdown"'), `${f}: klik mimo řeší PopMenu`);
  }
});
