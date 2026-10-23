// @vitest-environment node
// Složky zástupců (`folders.ts`): vytvoření tahem dlaždice na jinou, přidání do složky, vyjmutí, smazání
// prázdné, uspořádání dlaždic na kartě, normalizace a průchod volbami dlaždic v rozložení.
//   node --experimental-strip-types --test src/layout/folders.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { addToFolder, arrange, dropOnTile, folderIdFor, folderOf, normalizeFolders, removeFolder, renameFolder, takeOut } from "./folders.ts";
import { normalizeTiles, pageOf, setTiles, tilesOf } from "./model.ts";
import { fullLayout } from "./fullLayout.mjs";

const base = { size: "tile", label: true, git: true, hidden: [] };
const P = ["a", "b", "c", "d", "e"].map((id) => ({ id }));
const shape = (es) => es.map((e) => (e.kind === "project" ? e.p.id : `[${e.f.id}:${e.items.map((x) => x.id).join(",")}]`));

test("vytvoření: dlaždice na dlaždici = nová složka (cíl první), na místě cíle", () => {
  const r = dropOnTile(base, "d", "b", "Weby");
  assert.equal(r.folder, "slozka");
  assert.deepEqual(r.tiles.folders, [{ id: "slozka", name: "Weby", items: ["b", "d"] }]);
  assert.deepEqual(shape(arrange(P, r.tiles)), ["a", "[slozka:b,d]", "c", "e"]);
  assert.deepEqual(base, { size: "tile", label: true, git: true, hidden: [] }, "vstup netknutý");
  assert.equal(dropOnTile(base, "a", "a").folder, null, "sama na sebe nic");
  // druhá složka dostane další id
  const r2 = dropOnTile(r.tiles, "e", "c");
  assert.equal(r2.folder, "slozka-2");
  assert.equal(folderIdFor(r2.tiles), "slozka-3");
});

test("na dlaždici ve složce / na složku = do ní; z jiné složky se přesune", () => {
  const one = dropOnTile(base, "b", "a").tiles;
  const into = dropOnTile(one, "c", "b");
  assert.equal(into.folder, "slozka");
  assert.deepEqual(into.tiles.folders[0].items, ["a", "b", "c"]);
  const two = dropOnTile(into.tiles, "e", "d").tiles;
  const moved = addToFolder(two, "slozka", "e");
  assert.deepEqual(moved.folders.map((f) => f.items), [["a", "b", "c", "e"], ["d"]]);
  assert.equal(folderOf(moved, "e").id, "slozka");
  assert.equal(addToFolder(two, "neni", "e"), two, "neznámá složka = beze změny");
});

test("vyjmutí a smazání prázdné", () => {
  const f = dropOnTile(base, "b", "a").tiles;
  const out1 = takeOut(f, "b");
  assert.deepEqual(out1.folders, [{ id: "slozka", name: "", items: ["a"] }], "jedna zůstane");
  const out2 = takeOut(out1, "a");
  assert.ok(!("folders" in out2), "prázdná složka zmizí i s polem");
  assert.deepEqual(shape(arrange(P, out2)), ["a", "b", "c", "d", "e"]);
  const gone = removeFolder(dropOnTile(base, "c", "a").tiles, "slozka");
  assert.ok(!("folders" in gone));
  assert.deepEqual(renameFolder(f, "slozka", "  Hry  ").folders[0].name, "Hry");
  assert.equal([...renameFolder(f, "slozka", "x".repeat(50)).folders[0].name].length, 32);
});

test("uspořádání: skryté a neexistující projekty se ignorují, složka bez viditelných se neukáže", () => {
  const t = { ...base, folders: [{ id: "slozka", name: "", items: ["zmizel", "c"] }, { id: "jina", name: "", items: ["zmizel2"] }] };
  assert.deepEqual(shape(arrange(P, t)), ["a", "b", "[slozka:c]", "d", "e"]);
  assert.deepEqual(shape(arrange(P.filter((p) => p.id !== "c"), t)), ["a", "b", "d", "e"]);
});

test("normalizace: špatná id, duplicity, projekt ve dvou složkách, prázdné", () => {
  const raw = [
    { id: "Spatne", name: "x", items: ["a"] },
    { id: "ok", name: 5, items: ["a", "a", 3, ""] },
    { id: "ok", name: "dvakrát", items: ["b"] },
    { id: "druha", name: "D", items: ["a", "b"] },
    { id: "prazdna", name: "P", items: [] },
  ];
  assert.deepEqual(normalizeFolders(raw), [{ id: "ok", name: "", items: ["a"] }, { id: "druha", name: "D", items: ["b"] }]);
  assert.deepEqual(normalizeFolders(undefined), []);
});

test("rozložení: složky jdou do `tiles` a zpět; bez složek se pole nepíše", () => {
  assert.ok(!("folders" in normalizeTiles(base)));
  const t = dropOnTile(tilesOf(undefined), "b", "a").tiles;
  const l = setTiles(fullLayout(), "osobni", { folders: t.folders });
  assert.deepEqual(tilesOf(pageOf(l, "osobni").cards[0]).folders, [{ id: "slozka", name: "", items: ["a", "b"] }]);
  const back = setTiles(l, "osobni", { folders: [] });
  assert.equal(pageOf(back, "osobni").cards[0].tiles, undefined, "bez složek a s výchozími volbami = pole `tiles` pryč");
  assert.deepEqual(normalizeTiles(JSON.parse(JSON.stringify(tilesOf(pageOf(l, "osobni").cards[0])))), tilesOf(pageOf(l, "osobni").cards[0]));
});
