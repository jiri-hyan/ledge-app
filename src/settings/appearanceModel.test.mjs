// @vitest-environment node
// Panel Vzhled (vlna 7) bez DOM: kroky a kotvy (každá volba panelu Vzhled z labels.ts leží v kroku, skok
// z hledání přepne krok a rozbalí Pokročilé), šířka šuplíku = zrcadlo Rust `geom::preset_width` (stejná
// tabulka), monitor ve skutečném poměru, sloupce a ukázkové karty.
//   node --experimental-strip-types --test src/settings/appearanceModel.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  ADVANCED, ANCHOR_STEP, DENSITY_STEPS, STEPS, WIDTH_STEPS, deckCols, drawerLeft, drawerWidth, isAdvanced, monitorModel, sampleSpans, stepOf,
} from "./appearanceModel.ts";

const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");
const vzhled = () => [...read("labels.ts").matchAll(/^\s*"?([\w.]+)"?: \{ panel: "vzhled"/gm)].map((m) => m[1]);

test("každá volba panelu Vzhled leží v kroku (nebo vědomě mimo kroky) a nic navíc", () => {
  const ids = vzhled();
  assert.ok(ids.length >= 15, "labels.ts se načetl");
  assert.deepEqual(ids.filter((id) => !(id in ANCHOR_STEP)), [], "volba bez kroku");
  assert.deepEqual(Object.keys(ANCHOR_STEP).filter((id) => !ids.includes(id)), [], "krok pro neexistující volbu");
  for (const id of ADVANCED) assert.ok(stepOf(id), `${id} v Pokročilé má krok`);
});

test("skok z hledání: krok a Pokročilé", () => {
  const cases = [
    ["strip.look", "closed", false],
    ["strip.items", "closed", false],
    ["strip.monitor", "closed", false],
    ["drawer.hover", "hover", false],
    ["drawer.hoverExpandMs", "hover", true],
    ["appearance.style", "open", false],
    ["appearance.density", "open", false],
    ["drawer.width", "open", true],
    ["drawer.glass", "open", true],
    ["appearance.motion", "motion", false],
    ["ui.lang", undefined, false],
    ["neznamo", undefined, false],
    [null, undefined, false],
  ];
  for (const [id, step, adv] of cases) {
    assert.equal(stepOf(id), step, String(id));
    assert.equal(isAdvanced(id), adv, String(id));
  }
  assert.deepEqual([...STEPS], ["closed", "hover", "open", "motion"]);
});

test("šířka šuplíku = Rust preset_width (stejná tabulka, monitor v logických px)", () => {
  // (předvolba, drawer.width, monitor logických px, očekávaná šířka) — geom.rs `preset_width_table`
  const cases = [
    ["custom", 1080, 1920, 1080],
    ["custom", 1300, 1920, 1300],
    ["narrow", 1300, 1920, 820],
    ["medium", 2000, 1920, 1080],
    ["wide", 1080, 1920, 1440],
    ["wide", 1080, 1366, 1297],
    ["full", 1080, 1920, 1920],
    ["full", 1080, 1920 /* 3840 @ 200 % */, 1920],
    ["full", 1080, 0, 2400],
    ["custom", 9000, 0, 2400],
    ["custom", 100, 1920, 720],
  ];
  for (const [p, custom, mon, want] of cases) assert.equal(drawerWidth(p, custom, mon), want, `${p} ${custom} na ${mon}`);
});

test("monitor ve skutečném poměru: rozměr ze screen, jinak 16:9 bez čísel", () => {
  const C = { position: "center" };
  const m = monitorModel({ width: "wide", layout: "float", strip: C }, 1080, { w: 2560, h: 1440 });
  assert.deepEqual(m, { monW: 2560, monH: 1440, known: true, drawerW: 1440, left: 560, share: 1440 / 2560, cols: 3 });
  const u = monitorModel({ width: "full", layout: "classic", strip: C }, 1080, null);
  assert.equal(u.known, false);
  assert.equal(u.monW / u.monH, 16 / 9);
  assert.equal(u.share, 1, "celá šířka i bez známého monitoru = celý náhled");
  assert.equal(monitorModel({ width: "narrow", layout: "classic", strip: C }, 1080, { w: 0, h: 0 }).known, false);
  // široký notch (kolo 22): vlastní šířka 720 px bez ohledu na předvolbu, 2 sloupce pod lištou
  assert.deepEqual(monitorModel({ width: "wide", layout: "notch", strip: C }, 1300, { w: 1920, h: 1080 }), { monW: 1920, monH: 1080, known: true, drawerW: 720, left: 600, share: 720 / 1920, cols: 2 });
  // pozice u hrany = geom::anchor_c2 (odstup 24 px; celá šířka přiléhá k hraně)
  for (const [pos, mon, w, want] of [["center", 1920, 1080, 420], ["left", 1920, 1080, 24], ["right", 1920, 1080, 816], ["left", 1920, 1920, 0], ["right", 1920, 1920, 0], ["center", 1921, 1080, 420]]) {
    assert.equal(drawerLeft(pos, mon, w), want, `${pos} ${w} na ${mon}`);
  }
});

test("sloupce = rozložení stylu (bez vlastních = 4); ukázkové karty vyplní dvě řady", () => {
  assert.equal(deckCols("classic"), 4);
  assert.equal(deckCols("band"), 5);
  assert.equal(deckCols("sheet"), 2);
  assert.equal(deckCols("nic"), 4);
  for (const c of [1, 2, 3, 4, 5]) {
    const s = sampleSpans(c);
    assert.equal(s.reduce((a, b) => a + b, 0), 2 * c, `${c} sloupců`);
    assert.ok(s.every((x) => x >= 1 && x <= c));
  }
});

test("přepínače v panelu mají nejvýš 4 volby", () => {
  assert.ok(WIDTH_STEPS.length <= 4 && DENSITY_STEPS.length <= 4);
  assert.ok(!WIDTH_STEPS.includes("custom"), "vlastní šířka je v Pokročilé");
});
