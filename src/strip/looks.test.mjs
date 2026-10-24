// Seznam stylů zavřeného Ledge (`looks.ts`) proti Rustu a slovníku, bez DOM.
//   node --experimental-strip-types --test src/strip/looks.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { LOOK_HIDEABLE, LOOK_SHAPES, SHAPEABLE_LOOKS, STRIP_LOOKS, STRIP_LOOK_IDS, lookOf, normShape, withLookShown } from "./looks.ts";

test("id stylů = varianty `Look` v Rustu (looks/mod.rs), ve stejném pořadí", () => {
  const src = readFileSync(new URL("../../src-tauri/src/looks/mod.rs", import.meta.url), "utf8");
  const start = src.indexOf("pub enum Look {");
  const body = src.slice(start, src.indexOf("\n}", start));
  const rust = [...body.matchAll(/^\s*([A-Z][A-Za-z]*),\s*$/gm)].map((m) => m[1][0].toLowerCase() + m[1].slice(1));
  assert.deepEqual(rust, [...STRIP_LOOK_IDS]);
  assert.deepEqual(STRIP_LOOKS.map((l) => l.id), [...STRIP_LOOK_IDS]);
  assert.equal(STRIP_LOOKS[0].id, "classic", "první = výchozí");
});

test("každý styl má název, anglický název a popis; klíče sedí s id", () => {
  const en = readFileSync(new URL("./en.ts", import.meta.url), "utf8");
  for (const l of STRIP_LOOKS) {
    assert.equal(l.key, `appearance.look.${l.id}`);
    assert.equal(l.desc.key, `appearance.look.${l.id}.desc`);
    assert.ok(l.cs && l.en && l.desc.cs, l.id);
    assert.ok(en.includes(`"${l.key}": "${l.en}"`), `en.ts: ${l.key} = ${l.en}`);
    assert.ok(en.includes(`"${l.desc.key}": "`), `en.ts: ${l.desc.key}`);
  }
});

test("lookOf: neznámé id = Klasický", () => {
  assert.equal(lookOf("island").cs, "Ostrůvek");
  for (const bad of [undefined, null, "", "pill"]) assert.equal(lookOf(bad).id, "classic");
});

test("Pokročilé: co styl nemá ukazovat (strip.lookHide)", () => {
  const cases = [
    [undefined, "media", false, ["media"]],
    [["media"], "media", true, []],
    [["clock", "media"], "waiting", false, ["waiting", "media", "clock"]],
    [["media", "media"], "repos", true, ["media"]],
  ];
  for (const [hide, id, on, want] of cases) {
    const before = JSON.stringify(hide);
    assert.deepEqual(withLookShown(hide, id, on), want, `${before} ${id} ${on}`);
    assert.equal(JSON.stringify(hide), before, "vstup netknutý");
  }
  // vypnout jde jen to, co Rust `looks::without` zná
  const src = readFileSync(new URL("../../src-tauri/src/looks/mod.rs", import.meta.url), "utf8");
  for (const id of LOOK_HIDEABLE) {
    const v = id[0].toUpperCase() + id.slice(1);
    assert.ok(src.includes(`StripItem::${v} =>`), v);
  }
});

test("tvar kapsle (strip.lookShape): zrcadlo `Shape` a `Look::shapeable` v Rustu, neznámý = round", () => {
  const src = readFileSync(new URL("../../src-tauri/src/looks/mod.rs", import.meta.url), "utf8");
  const start = src.indexOf("pub enum Shape {");
  const body = src.slice(start, src.indexOf("\n}", start));
  const rust = [...body.matchAll(/^\s*([A-Z][A-Za-z]*),\s*$/gm)].map((m) => m[1].toLowerCase());
  assert.deepEqual(rust, [...LOOK_SHAPES]);
  const at = src.indexOf("pub fn shapeable");
  const fn = src.slice(at, src.indexOf("}", at));
  const ids = [...fn.matchAll(/Look::([A-Z][a-z]+)/g)].map((m) => m[1].toLowerCase());
  assert.deepEqual(ids, [...SHAPEABLE_LOOKS]);
  for (const id of SHAPEABLE_LOOKS) assert.ok(STRIP_LOOK_IDS.includes(id), id);
  for (const [v, want] of [["island", "island"], ["sharp", "sharp"], [undefined, "round"], ["pill", "round"], [3, "round"]]) assert.equal(normShape(v), want, String(v));
});
