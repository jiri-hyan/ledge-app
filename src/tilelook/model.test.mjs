// @vitest-environment node
// Tabulkové testy vzhledu zástupců (`model.ts`, `glyphs.ts`): normalizace, výchozí = beze změny, kontrast
// kresby na každém podkladu, zrcadlo předvoleb s Rustem, změna v configu, přehled a Obnovit vše, ikony.
//   node --experimental-strip-types --test src/tilelook/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  contrast, customizedTiles, inkDark, isPlain, normalizeTileLook, patchLook, resetTileLooks, SHAPES, TILE_COLORS, tileLabel, withTileLook,
} from "./model.ts";
import { GLYPHS, searchGlyphs } from "./glyphs.ts";

test("normalizace: prázdné a nesmysly = žádný vzhled (dlaždice jako dřív)", () => {
  const cases = [
    ["nic", undefined, undefined],
    ["prázdný objekt", {}, undefined],
    ["výchozí tvar", { shape: "rounded" }, undefined],
    ["prázdný popisek", { label: "  ", hideLabel: false }, undefined],
    ["neznámé", { icon: { kind: "svg", value: "x" }, color: "red", shape: "star" }, undefined],
    ["emoji", { icon: { kind: "emoji", value: "🎮" } }, { icon: { kind: "emoji", value: "🎮" } }],
    ["dlouhé emoji", { icon: { kind: "emoji", value: "123456789" } }, undefined],
    ["glyf malými", { icon: { kind: "glyph", value: "e80f" } }, { icon: { kind: "glyph", value: "E80F" } }],
    ["glyf mimo PUA", { icon: { kind: "glyph", value: "0041" } }, undefined],
    ["obrázek = klíč", { icon: { kind: "image", value: "0123456789abcdef" } }, { icon: { kind: "image", value: "0123456789abcdef" } }],
    ["obrázek = cesta", { icon: { kind: "image", value: "C:\\a.png" } }, undefined],
    ["vše", { color: "#2563EB", shape: "glass", label: "Web", hideLabel: true }, { color: "#2563eb", shape: "glass", label: "Web", hideLabel: true }],
    ["popisek přes 40", { label: "x".repeat(41) }, undefined],
  ];
  for (const [name, raw, want] of cases) assert.deepEqual(normalizeTileLook(raw), want, name);
  assert.ok(isPlain(undefined) && isPlain({ shape: "rounded" }) && !isPlain({ shape: "circle" }));
  assert.deepEqual(SHAPES, ["rounded", "circle", "square", "glass"], "nejvýš 4 volby → segment");
});

test("patchLook: undefined / false / prázdné = pole pryč; popisek", () => {
  const l = patchLook(undefined, { color: "#16a34a", label: "A" });
  assert.deepEqual(l, { color: "#16a34a", label: "A" });
  assert.deepEqual(patchLook(l, { label: "" }), { color: "#16a34a" });
  assert.equal(patchLook(l, { color: undefined, label: undefined }), undefined);
  assert.equal(tileLabel("Harbor", undefined), "Harbor");
  assert.equal(tileLabel("Harbor", { label: "Přístav" }), "Přístav");
});

test("kresba na podkladu: černá nebo bílá vždy ≥ 4.5:1 (předvolby i celé RGB)", () => {
  const ink = (hex) => (inkDark(hex) ? "#000000" : "#ffffff");
  for (const c of TILE_COLORS) assert.ok(contrast(c, ink(c)) >= 4.5, `${c}: ${contrast(c, ink(c))}`);
  const h = (n) => n.toString(16).padStart(2, "0");
  for (let r = 0; r <= 255; r += 17) for (let g = 0; g <= 255; g += 17) for (let b = 0; b <= 255; b += 17) {
    const c = `#${h(r)}${h(g)}${h(b)}`;
    assert.ok(contrast(c, ink(c)) >= 4.5, c);
  }
  assert.equal(inkDark("#f59e0b"), true);
  assert.equal(inkDark("#1f2937"), false);
});

test("zrcadlo Rustu: předvolby barev a výpočet kresby jako tilelook.rs", () => {
  const src = readFileSync(new URL("../../src-tauri/src/tilelook.rs", import.meta.url), "utf8");
  const block = src.slice(src.indexOf("pub const TILE_COLORS"), src.indexOf("];", src.indexOf("pub const TILE_COLORS")));
  assert.deepEqual(block.match(/"#[0-9a-f]{6}"/g).map((s) => s.slice(1, -1)), [...TILE_COLORS]);
  assert.match(src, /pub const MAX_LABEL: usize = 40;/);
});

test("config: vzhled jednoho projektu / akce, přehled a Obnovit vše", () => {
  const cfg = { projects: [{ id: "a", name: "A" }, { id: "b", name: "B", look: { shape: "circle" } }], actions: [{ id: "web", label: "Web", kind: "url", target: "https://x.cz" }] };
  const one = withTileLook(cfg, "project", "a", { icon: { kind: "emoji", value: "⚓" } });
  assert.deepEqual(one.projects[0].look, { icon: { kind: "emoji", value: "⚓" } });
  assert.equal(cfg.projects[0].look, undefined, "vstup netknutý");
  const two = withTileLook(one, "action", "web", { color: "#db2777", hideLabel: true });
  assert.deepEqual(customizedTiles(two).map((x) => `${x.kind}:${x.id}:${x.name}`), ["project:a:A", "project:b:B", "action:web:Web"]);
  const off = withTileLook(two, "project", "b", { shape: "rounded" });
  assert.ok(!("look" in off.projects[1]), "výchozí vzhled = pole pryč");
  const reset = resetTileLooks(two);
  assert.deepEqual(customizedTiles(reset), []);
  assert.ok(reset.projects.every((p) => !("look" in p)) && !("look" in reset.actions[0]));
  assert.deepEqual(customizedTiles({}), []);
});

test("systémové ikony: kódy v PUA, id jednou, hledání bez diakritiky", () => {
  assert.ok(GLYPHS.length >= 60);
  assert.equal(new Set(GLYPHS.map((g) => g.id)).size, GLYPHS.length);
  assert.equal(new Set(GLYPHS.map((g) => g.cp)).size, GLYPHS.length);
  for (const g of GLYPHS) {
    const c = parseInt(g.cp, 16);
    assert.ok(/^[0-9A-F]{4}$/.test(g.cp) && c >= 0xe000 && c <= 0xf8ff, g.id);
    assert.equal(g.name[0], `cards.tile.glyph.${g.id}`);
  }
  assert.equal(searchGlyphs("").length, GLYPHS.length);
  assert.deepEqual(searchGlyphs("domu").map((g) => g.id), ["home"]);
  assert.ok(searchGlyphs("hra").some((g) => g.id === "game"));
  assert.ok(searchGlyphs("GAME").some((g) => g.id === "game"), "anglické id");
  assert.deepEqual(searchGlyphs("e80f").map((g) => g.id), ["home"], "kód");
  assert.deepEqual(searchGlyphs("xyzzy"), []);
});
