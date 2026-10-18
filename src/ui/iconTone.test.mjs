// Tón ikony (`iconTone.ts`): tmavá / světlá kresba na průhledném pozadí, ikona s vlastním pozadím, prázdná.
//   node --experimental-strip-types --test src/ui/iconTone.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { toneOf } from "./iconTone.ts";

/** 10×10 obrázek: `cover` pixelů barvy `rgb` s alfou `a`, zbytek průhledný (nebo `bg`). */
function img(cover, rgb, a = 255, bg = null) {
  const px = [];
  for (let i = 0; i < 100; i++) {
    if (i < cover) px.push(...rgb, a);
    else if (bg) px.push(...bg, 255);
    else px.push(0, 0, 0, 0);
  }
  return px;
}

test("tón kresby (tabulka)", () => {
  const cases = [
    ["černý GitHub na průhledném", img(45, [24, 23, 23]), "dark"],
    ["tmavě šedá", img(30, [60, 60, 60]), "dark"],
    ["bílá kresba", img(40, [255, 255, 255]), "light"],
    ["světle šedá", img(40, [220, 220, 220]), "light"],
    ["barevná (modrá)", img(50, [37, 99, 235]), "mid"],
    ["střední šedá", img(50, [128, 128, 128]), "mid"],
    ["vlastní pozadí (plná ikona)", img(10, [0, 0, 0], 255, [255, 255, 255]), "mid"],
    ["skoro prázdná", img(2, [0, 0, 0]), "mid"],
    ["poloprůhledná černá se počítá", img(60, [0, 0, 0], 128), "dark"],
    ["nic", [], "mid"],
  ];
  for (const [what, px, want] of cases) assert.equal(toneOf(px), want, what);
});
