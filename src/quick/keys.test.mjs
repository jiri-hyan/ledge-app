// Tabulkové testy klávesnice v šuplíku (`keys.ts`) + hlídač, že selektor zástupců sedí s kartami (`quick.ts`).
//   node --experimental-strip-types --test src/quick/keys.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { badges, keyPlan, neighbor } from "./keys.ts";

const k = (key, mods = {}) => ({ key, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, ...mods });
// mřížka 3 × 2: řada 1 (y 0), řada 2 (y 100); sloupce x 0, 100, 200; dlaždice 80 × 80
const GRID = [0, 1, 2, 3, 4, 5].map((i) => ({ x: (i % 3) * 100, y: Math.floor(i / 3) * 100, w: 80, h: 80 }));
const ctx = (o = {}) => ({ enabled: true, page: "main", editing: false, paletteOpen: false, editable: false, onBody: true, rects: GRID, focused: -1, ...o });

test("Alt+číslo spustí N-tého zástupce", () => {
  const alt = (key) => k(key, { altKey: true });
  const cases = [
    ["Alt+1", alt("1"), ctx(), { do: "launch", index: 0 }],
    ["Alt+6", alt("6"), ctx(), { do: "launch", index: 5 }],
    ["Alt+7 — není sedmý", alt("7"), ctx(), null],
    ["Alt+0", alt("0"), ctx(), null],
    ["Alt+a", alt("a"), ctx(), null],
    ["Alt+Shift+1", k("1", { altKey: true, shiftKey: true }), ctx(), null],
    ["Ctrl+Alt+1", k("1", { altKey: true, ctrlKey: true }), ctx(), null],
    ["bez Alt: číslo je záložka (App)", k("1"), ctx(), null],
    ["vypnuto volbou", alt("1"), ctx({ enabled: false }), null],
    ["jiná stránka než hlavní", alt("1"), ctx({ page: "settings" }), null],
    ["v úpravách šuplíku", alt("1"), ctx({ editing: true }), null],
    ["s otevřenou paletou", alt("1"), ctx({ paletteOpen: true }), null],
    ["v poli pro psaní", alt("1"), ctx({ editable: true }), null],
    ["žádní zástupci", alt("1"), ctx({ rects: [] }), null],
  ];
  for (const [name, key, c, want] of cases) assert.deepEqual(keyPlan(key, c), want, name);
});

test("Alt+9 s víc než devíti zástupci míří na devátého, desátý číslo nemá", () => {
  const rects = Array.from({ length: 12 }, (_, i) => ({ x: i * 100, y: 0, w: 80, h: 80 }));
  assert.deepEqual(keyPlan(k("9", { altKey: true }), ctx({ rects })), { do: "launch", index: 8 });
  assert.deepEqual(badges(12), [1, 2, 3, 4, 5, 6, 7, 8, 9]);
  assert.deepEqual(badges(3), [1, 2, 3]);
  assert.deepEqual(badges(0), []);
});

test("šipky: první šipka vybere prvního, dál soused podle rozmístění", () => {
  const on = (focused) => ctx({ focused, onBody: false });
  const cases = [
    ["bez fokusu → první", k("ArrowRight"), ctx(), { do: "focus", index: 0 }],
    ["↓ bez fokusu → první", k("ArrowDown"), ctx(), { do: "focus", index: 0 }],
    ["→ z 0", k("ArrowRight"), on(0), { do: "focus", index: 1 }],
    ["← z 1", k("ArrowLeft"), on(1), { do: "focus", index: 0 }],
    ["← z 0 — okraj", k("ArrowLeft"), on(0), null],
    ["→ z posledního — okraj", k("ArrowRight"), on(5), null],
    ["↓ z 1 → 4", k("ArrowDown"), on(1), { do: "focus", index: 4 }],
    ["↑ z 5 → 2", k("ArrowUp"), on(5), { do: "focus", index: 2 }],
    ["↑ z horní řady — okraj", k("ArrowUp"), on(1), null],
    ["↓ z dolní řady — okraj", k("ArrowDown"), on(4), null],
    ["Home", k("Home"), on(4), { do: "focus", index: 0 }],
    ["End", k("End"), on(1), { do: "focus", index: 5 }],
    ["Home bez fokusu se nechytá", k("Home"), ctx(), null],
    ["fokus jinde (tlačítko mimo zástupce)", k("ArrowRight"), ctx({ onBody: false, focused: -1 }), null],
    ["v poli pro psaní", k("ArrowRight"), ctx({ editable: true }), null],
    ["s Ctrl", k("ArrowRight", { ctrlKey: true }), ctx(), null],
    ["se Shift", k("ArrowRight", { shiftKey: true }), ctx(), null],
    ["Alt+šipka patří záložkám a řazení", k("ArrowRight", { altKey: true }), on(0), null],
    ["vypnuto", k("ArrowRight"), ctx({ enabled: false }), null],
    ["Enter nechat tlačítku", k("Enter"), on(0), null],
  ];
  for (const [name, key, c, want] of cases) assert.deepEqual(keyPlan(key, c), want, name);
});

test("neighbor: nerovná mřížka (poslední řada kratší)", () => {
  // řada 1: dvě dlaždice, řada 2: jedna uprostřed
  const rs = [
    { x: 0, y: 0, w: 100, h: 80 },
    { x: 200, y: 0, w: 100, h: 80 },
    { x: 100, y: 100, w: 100, h: 80 },
  ];
  assert.equal(neighbor(rs, 0, "ArrowDown"), 2);
  assert.equal(neighbor(rs, 1, "ArrowDown"), 2);
  assert.equal(neighbor(rs, 2, "ArrowUp"), 0); // shodná vzdálenost → dřívější v pořadí
  assert.equal(neighbor(rs, 9, "ArrowDown"), -1);
  assert.equal(neighbor([], 0, "ArrowRight"), -1);
});

test("selektor zástupců sedí s kartami: třídy v komponentách existují", () => {
  const read = (f) => fs.readFileSync(new URL(f, import.meta.url), "utf8");
  const sel = read("./quick.ts").match(/QUICK_SELECTOR = "([^"]+)"/)[1];
  const classes = [...sel.matchAll(/\.([a-z-]+)/g)].map((m) => m[1]).filter((c) => c !== "deck");
  assert.deepEqual([...classes].sort(), ["act-tile", "link-row", "proj-hit"]);
  const files = { "proj-hit": "../cards/ProjectTile.tsx", "act-tile": "../cards/ActionsCard.tsx", "link-row": "../cards/LinksCard.tsx" };
  for (const c of classes) assert.ok(read(files[c]).includes(c), c);
  assert.ok(read("../layout/Deck.tsx").includes('"deck'), "mřížka karet je .deck");
});

test("zapojení: jen window šuplíku, žádný časovač ani volání jádra, číslování jen přes atribut", () => {
  const src = fs.readFileSync(new URL("./quick.ts", import.meta.url), "utf8");
  assert.ok(!/setInterval|setTimeout|requestAnimationFrame/.test(src), "žádný časovač ani rAF");
  assert.ok(!/api\./.test(src), "žádné volání jádra (nic globálního)");
  assert.ok(src.includes('"data-qn"'));
  const css = fs.readFileSync(new URL("./quick.css", import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  assert.ok(!/(^|[^\w-])\.[a-z]/m.test(css), "quick.css nesmí mít třídy (kolize CSS tříd)");
});
