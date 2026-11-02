// @vitest-environment node
// Barevné povrchy karet (`surfaces.ts`): kontrast textu nad všemi stopami přechodů, sady, proměnné.
//   node --experimental-strip-types --test src/layout/surfaces.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { gradientStops, GRADIENT_TEXT, PALETTE_COLORS, PALETTES, normalizePalette, paletteStyle, contrast, mixHex } from "./surfaces.ts";

const THEME_ACCENTS = ["#a78bfa", "#60a5fa", "#4ade80", "#fbbf24", "#f87171", "#2dd4bf", "#fb923c", "#f472b6", "#e5e7eb"];

test("text na přechodu má kontrast ≥ 4,5 : 1 nad každou stopou každé sady", () => {
  for (const p of PALETTES) {
    const accents = p === "card" ? THEME_ACCENTS : [undefined];
    for (const ac of accents) {
      for (const stop of gradientStops(p === "card" ? undefined : p, ac)) {
        for (const [k, txt] of Object.entries(GRADIENT_TEXT)) {
          assert.ok(contrast(txt, stop) >= 4.5, `${p} ${ac ?? ""} ${stop} ${k}: ${contrast(txt, stop).toFixed(2)}`);
        }
      }
    }
  }
});

test("sady: normalizace a proměnné", () => {
  assert.equal(normalizePalette("card"), undefined);
  assert.equal(normalizePalette("neexistuje"), undefined);
  assert.equal(normalizePalette("toString"), undefined);
  assert.equal(normalizePalette("ocean"), "ocean");
  assert.deepEqual(paletteStyle("gradient", undefined), {});
  assert.deepEqual(paletteStyle("glass", "ocean"), {});
  assert.deepEqual(Object.keys(paletteStyle("flow", "sunset")), ["--sf-a", "--sf-b", "--sf-e"]);
  for (const c of Object.values(PALETTE_COLORS)) for (const h of Object.values(c)) assert.match(h, /^#[0-9a-f]{6}$/);
});

test("mixHex", () => {
  assert.equal(mixHex("#ffffff", "#000000", 0.5), "#808080");
});
