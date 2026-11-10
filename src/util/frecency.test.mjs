// Testy pro frecency modul (scoreOf, recordEntry, sortByFrecency, recordUse, toggleFrecencySort).
// Spuštění: node --test src/util/frecency.test.mjs
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import {
  FRECENCY_KEY,
  FRECENCY_SORT_KEY,
  scoreOf,
  recordEntry,
  sortByFrecency,
  recordUse,
  frecencyMap,
  isFrecencySort,
  toggleFrecencySort,
} from "./frecency.ts";

// Nastavení mock localStorage pro Node.js prostředí
const store = new Map();
globalThis.localStorage = {
  getItem: (key) => (store.has(key) ? store.get(key) : null),
  setItem: (key, value) => store.set(key, String(value)),
  removeItem: (key) => store.delete(key),
  clear: () => store.clear(),
};

beforeEach(() => {
  store.clear();
});

test("scoreOf: vrací 0 pro neexistující záznam nebo neplatný počet", () => {
  const now = 1_000_000;
  assert.equal(scoreOf(undefined, now), 0);
  assert.equal(scoreOf({ count: 0, lastMs: now }, now), 0);
  assert.equal(scoreOf({ count: -1, lastMs: now }, now), 0);
});

test("scoreOf: čerstvé použití (věk 0) má trojnásobnou váhu (1 + 2 * 1)", () => {
  const now = 10_000_000;
  const score = scoreOf({ count: 5, lastMs: now }, now);
  assert.equal(score, 15);
});

test("scoreOf: poločas rozpadu 72 hodin sníží bonus na polovinu (1 + 2 * 0.5 = 2)", () => {
  const now = 100_000_000;
  const seventyTwoHoursMs = 72 * 3600 * 1000;
  const score = scoreOf({ count: 4, lastMs: now - seventyTwoHoursMs }, now);
  assert.equal(score, 8); // 4 * (1 + 2 * 0.5) = 8
});

test("scoreOf: hodiny v budoucnosti (posun hodin) jsou ošetřeny jako nulový věk", () => {
  const now = 10_000_000;
  const score = scoreOf({ count: 2, lastMs: now + 50_000 }, now);
  assert.equal(score, 6);
});

test("recordEntry: vytvoří nový záznam a nezmění původní mapu", () => {
  const map = {};
  const now = 5000;
  const next = recordEntry(map, "proj:a", now);

  assert.deepEqual(map, {});
  assert.deepEqual(next["proj:a"], { count: 1, lastMs: 5000 });
});

test("recordEntry: zvýší počet a aktualizuje čas existujícího záznamu", () => {
  const map = { "act:x": { count: 3, lastMs: 1000 } };
  const now = 8000;
  const next = recordEntry(map, "act:x", now);

  assert.equal(next["act:x"].count, 4);
  assert.equal(next["act:x"].lastMs, 8000);
});

test("recordEntry: omezí velikost na 200 nejnovějších položek", () => {
  let map = {};
  for (let i = 1; i <= 205; i++) {
    map = recordEntry(map, `item:${i}`, i * 1000);
  }

  const keys = Object.keys(map);
  assert.equal(keys.length, 200);
  // item:1 až item:5 by měly být prořezány, protože mají nejmenší lastMs
  assert.equal(map["item:1"], undefined);
  assert.equal(map["item:5"], undefined);
  assert.ok(map["item:6"]);
  assert.ok(map["item:205"]);
});

test("sortByFrecency: stabilní řazení, položky se score > 0 jdou na začátek", () => {
  const now = 10_000_000;
  const map = {
    "p:active": { count: 10, lastMs: now },
    "p:moderate": { count: 2, lastMs: now },
  };

  const items = [
    { id: "p:unopened1", name: "U1" },
    { id: "p:moderate", name: "Mod" },
    { id: "p:unopened2", name: "U2" },
    { id: "p:active", name: "Act" },
    { id: "p:unopened3", name: "U3" },
  ];

  const sorted = sortByFrecency(items, (x) => x.id, map, now);
  assert.deepEqual(
    sorted.map((x) => x.id),
    ["p:active", "p:moderate", "p:unopened1", "p:unopened2", "p:unopened3"],
    "Aktivní položky sestupně podle skóre, nepoužité v původním pořadí"
  );
});

test("sortByFrecency: položky se stejným skóre zachovávají relativní pořadí", () => {
  const now = 10_000_000;
  const map = {
    "a:1": { count: 2, lastMs: now },
    "a:2": { count: 2, lastMs: now },
  };

  const items = [
    { id: "a:1" },
    { id: "a:2" },
  ];

  const sorted = sortByFrecency(items, (x) => x.id, map, now);
  assert.deepEqual(sorted.map((x) => x.id), ["a:1", "a:2"]);
});

test("recordUse a frecencyMap: reaktivní signál a perzistence do localStorage", () => {
  const now = 20_000;
  recordUse("proj:alpha", now);

  const current = frecencyMap();
  assert.deepEqual(current["proj:alpha"], { count: 1, lastMs: now });

  const stored = JSON.parse(globalThis.localStorage.getItem(FRECENCY_KEY));
  assert.deepEqual(stored["proj:alpha"], { count: 1, lastMs: now });
});

test("isFrecencySort a toggleFrecencySort: přepínání a perzistence", () => {
  const initialProj = isFrecencySort("proj");
  const initialAct = isFrecencySort("act");

  const toggledProj = toggleFrecencySort("proj");
  assert.equal(toggledProj, !initialProj);
  assert.equal(isFrecencySort("proj"), toggledProj);
  assert.equal(isFrecencySort("act"), initialAct, "Přepnutí proj neovlivní act");

  const stored = JSON.parse(globalThis.localStorage.getItem(FRECENCY_SORT_KEY));
  assert.equal(stored.proj, toggledProj);

  // Vrátit zpět
  const revertedProj = toggleFrecencySort("proj");
  assert.equal(revertedProj, initialProj);
  assert.equal(isFrecencySort("proj"), initialProj);
});
