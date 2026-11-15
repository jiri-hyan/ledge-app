// Motiv „Podle Windows“ (`auto.ts`, zrcadlo `appearance/auto.rs`): tabulka případů, normalizace, starý config beze změny.
// `node --experimental-strip-types --test src/theme/auto.test.mjs`
import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_AUTO, NO_WIN, isDefaultAuto, normalizeAuto, resolveAuto } from "./auto.ts";
import { DEFAULT_APPEARANCE, appearanceVars, normalizeAppearance } from "./appearance.ts";
import { THEMES } from "./themes.ts";
import { contrast, parseColor } from "./contrast.ts";

const base = (auto, extra = {}) => normalizeAppearance({ theme: "nord", customTheme: "", auto, ...extra });

test("resolveAuto: tabulka (zapnuto / akcent × stav Windows)", () => {
  const win = (light, accent) => ({ light, accent });
  const pair = { light: "paper", dark: "nord" };
  // [auto, Windows, očekávaný motiv, akcent je z Windows]
  const cases = [
    [{ on: false, ...pair, accent: false }, win(true, "#0078d4"), "nord", false],
    [{ on: true, ...pair, accent: false }, win(true, "#0078d4"), "paper", false],
    [{ on: true, ...pair, accent: false }, win(false, null), "nord", false],
    [{ on: true, ...pair, accent: false }, win(null, null), "nord", false],
    [{ on: false, ...pair, accent: true }, win(true, "#0078d4"), "nord", true],
    [{ on: true, ...pair, accent: true }, win(true, "#0078d4"), "paper", true],
    [{ on: true, ...pair, accent: true }, win(false, null), "nord", false],
  ];
  for (const [i, [auto, w, theme, acc]] of cases.entries()) {
    const out = resolveAuto(base(auto), w);
    assert.equal(out.theme, theme, `případ ${i}: motiv`);
    assert.equal(!!out.accent, acc, `případ ${i}: akcent`);
  }
});

test("resolveAuto: vlastní motiv se přeskočí, akcent Windows se doladí na čitelnost, gradient pryč", () => {
  const a = { ...base({ on: true, light: "light", dark: "dark", accent: true }), customTheme: "x", accent2: "#000000", accentFromBackdrop: true };
  const out = resolveAuto({ ...a, customTheme: "x" }, { light: true, accent: "#ffd400" });
  assert.equal(out.customTheme, "");
  assert.equal(out.accent2, undefined);
  assert.equal(out.accentFromBackdrop, undefined);
  const bg = THEMES.light.vars;
  const [r, g, b] = bg.bg.split(" ").map(Number);
  assert.ok(contrast(parseColor(out.accent), [r, g, b, 1]) >= 3, "žlutá na světlém se ztmaví");
});

test("bez auto nebo bez údajů Windows se vzhled nemění (tentýž objekt); starý config nemá pole auto", () => {
  const old = normalizeAppearance({ theme: "nord", accent: "#22c55e" });
  assert.equal("auto" in old, false);
  assert.equal(resolveAuto(old, { light: false, accent: "#ffffff" }), old);
  assert.equal(resolveAuto(base({ on: true }), NO_WIN).theme, "nord");
  assert.deepEqual(appearanceVars(resolveAuto(DEFAULT_APPEARANCE, NO_WIN)), appearanceVars(DEFAULT_APPEARANCE));
});

test("normalizeAuto: neplatné hodnoty → výchozí, výchozí se nezapisuje", () => {
  assert.deepEqual(normalizeAuto(undefined), DEFAULT_AUTO);
  assert.deepEqual(normalizeAuto({ on: "ano", light: "nic", dark: 3, accent: 1 }), DEFAULT_AUTO);
  assert.ok(isDefaultAuto(normalizeAuto({})));
  const a = normalizeAppearance({ auto: { on: true, light: "mint", dark: "sunset", accent: true } });
  assert.deepEqual(a.auto, { on: true, light: "mint", dark: "sunset", accent: true });
});
