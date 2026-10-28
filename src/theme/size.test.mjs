// @vitest-environment node
// Velikost šuplíku (`size.ts`, kolo 23): starý config = výchozí (žádné pole), normalizace a zápis jako Rust
// `appearance::size`, přichycení k mřížce (Alt = volně), šířka od osy, boky podle pozice, „nevejde se“.
//   node --experimental-strip-types --test src/theme/size.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { clampSize, contentHeight, dragged, dragWidth, fallbackMaxH, heightCss, HEIGHT_LINES, isDefaultSize, normalizeSize, overflowCount, pruneSizePages, switchedSize, setSize, sizeFor, snapTo, widthSides, WIDTH_LINES } from "./size.ts";
import { DEFAULT_APPEARANCE, normalizeAppearance } from "./appearance.ts";

test("starý config: žádná velikost, normalizace pole nepřidá", () => {
  assert.deepEqual(normalizeSize(undefined), {});
  assert.ok(isDefaultSize(normalizeSize({})));
  assert.equal("size" in normalizeAppearance({ theme: "nord" }), false, "výchozí vzhled zůstává bez pole size");
  assert.deepEqual(normalizeAppearance(DEFAULT_APPEARANCE), normalizeAppearance({}));
  assert.deepEqual(sizeFor(undefined, "prace"), { width: 0, height: 0 });
  assert.deepEqual(normalizeAppearance({ size: { width: 1200 } }).size, { width: 1200 });
});

test("normalizace a ořez", () => {
  const cases = [
    [{ width: 500, height: 2000 }, { width: 720, height: 940 }],
    [{ width: 0, height: -5 }, {}],
    [{ height: 5 }, { height: 270 }], // ruční zásah: zrcadlo Rust `for_page_clamps_hand_edited`
    [{ perPage: true, pages: { a: { height: 7 } } }, { perPage: true, pages: { a: { height: 270 } } }],
    [{ width: "1200" }, {}],
    [{ perPage: true, pages: { prace: { width: 1440 }, "Zlé id": { width: 900 }, osobni: {} } }, { perPage: true, pages: { prace: { width: 1440 } } }],
  ];
  for (const [raw, want] of cases) assert.deepEqual(normalizeSize(raw), want, JSON.stringify(raw));
  assert.deepEqual(clampSize({ width: 3000 }), { width: 2400, height: 0 });
});

test("velikost záložky a zápis (zrcadlo SizeConfig::for_page / set)", () => {
  const off = { width: 1200, height: 500, pages: { prace: { width: 1440 }, osobni: { height: 600 } } };
  const on = { ...off, perPage: true };
  const cases = [
    [off, "prace", { width: 1200, height: 500 }],
    [on, "prace", { width: 1440, height: 500 }],
    [on, "osobni", { width: 1200, height: 600 }],
    [on, "app", { width: 1200, height: 500 }],
  ];
  for (const [s, page, want] of cases) assert.deepEqual(sizeFor(s, page), want, `${page} ${!!s.perPage}`);
  assert.deepEqual(setSize({}, "prace", { width: 1300 }), { width: 1300 });
  assert.deepEqual(setSize({ width: 1300, height: 500 }, "prace", { width: 0 }), {}, "bez volby přepíše celek");
  const pp = setSize({ perPage: true }, "osobni", { width: 900, height: 436 });
  assert.deepEqual(pp, { perPage: true, pages: { osobni: { width: 900, height: 436 } } });
  assert.deepEqual(setSize(pp, "osobni", {}), { perPage: true }, "výchozí = záznam pryč");
});

test("přichycení k mřížce, Alt volně", () => {
  assert.ok(WIDTH_LINES.includes(1080) && WIDTH_LINES.includes(1440) && WIDTH_LINES.at(-1) === 2400);
  assert.deepEqual(HEIGHT_LINES, [270, 436, 602, 768, 934]);
  const cases = [
    [1090, WIDTH_LINES, false, 1080],
    [1110, WIDTH_LINES, false, 1110], // mimo dosah čáry
    [1090.4, WIDTH_LINES, true, 1090],
    [450, HEIGHT_LINES, false, 436],
  ];
  for (const [v, lines, free, want] of cases) assert.equal(snapTo(v, lines, free), want, `${v} free=${free}`);
  assert.deepEqual(dragged(600, 2000, false), { width: 720, height: 940 });
});

test("šířka od osy a boky podle pozice", () => {
  const cases = [
    [1080, 30, "right", "center", 1140],
    [1080, -30, "left", "center", 1140], // levý bok doleva = širší
    [1080, 30, "right", "left", 1110],   // u hrany jen o posun
    [1080, 30, "left", "right", 1050],
  ];
  for (const [w, dx, side, pos, want] of cases) assert.equal(dragWidth(w, dx, side, pos), want, `${side} ${pos}`);
  assert.deepEqual(widthSides("center", "classic"), ["left", "right"]);
  assert.deepEqual(widthSides("left", "classic"), ["right"]);
  assert.deepEqual(widthSides("center", "notch"), [], "notch má pevnou šířku");
});

test("nevejde se N karet", () => {
  assert.equal(overflowCount([100, 300, 520, 700], 500), 2);
  assert.equal(overflowCount([100, 500.5], 500), 0, "zaokrouhlení hrany není přetečení");
  assert.equal(overflowCount([], 500), 0);
});

test("úklid size.pages: smazané záložky pryč, jinak tentýž objekt", () => {
  const s = { perPage: true, width: 1200, pages: { prace: { width: 1440 }, smazana: { height: 600 } } };
  const same = pruneSizePages(s, ["prace", "smazana"]);
  assert.equal(same, s, "nic ke smazání = tentýž objekt");
  assert.deepEqual(pruneSizePages(s, ["prace"]), { perPage: true, width: 1200, pages: { prace: { width: 1440 } } });
  assert.equal(pruneSizePages(s, []).pages, undefined, "poslední pryč = pole pages zmizí");
  assert.equal(pruneSizePages(undefined, ["prace"]), undefined);
  assert.deepEqual(pruneSizePages({ width: 1200 }, ["prace"]), { width: 1200 });
});

test("přepnutí záložky v úpravách: nová velikost jen s perPage a jinou velikostí", () => {
  const s = { perPage: true, width: 1200, pages: { prace: { width: 1440 }, osobni: { height: 600 } } };
  const table = [
    [s, "prace", "osobni", { width: 1200, height: 600 }],
    [s, "osobni", "prace", { width: 1440, height: 0 }],
    [s, "prace", "prace", null],
    [s, "", "prace", null],
    [s, "x", "y", null], // obě jen z celku
    [{ ...s, perPage: false }, "prace", "osobni", null],
    [undefined, "prace", "osobni", null],
  ];
  for (const [cfg, a, b, want] of table) assert.deepEqual(switchedSize(cfg, a, b), want, `${a}→${b}`);
});

test("výška hlavní stránky se nikdy neváže na výšku okna (smyčka 0.11.0: šuplík zamrzl na 219 px)", () => {
  assert.equal(heightCss(0), undefined);
  assert.equal(heightCss(934), "min(934px, var(--drawer-max-h, 934px))");
  assert.ok(!heightCss(934).includes("vh"), "fallback nesmí být 100vh");
});

test("záložní strop výšky = Rust drawerfit::limits (80 %, 20 px od spodku, aspoň 240)", () => {
  assert.equal(fallbackMaxH(1032), 825); // 1080 − hlavní panel, jako v logu šéfa
  assert.equal(fallbackMaxH(100), 240);
  assert.equal(fallbackMaxH(0), 240);
  assert.equal(fallbackMaxH(Number.NaN), 240);
});

test("výška pro Rust ignoruje transformaci otevírací animace (uříznutý šuplík 219 px)", () => {
  assert.equal(contentHeight(825, 825), 825);
  assert.equal(contentHeight(824.4, 824), 825, "bez transformace přesná hodnota nahoru");
  assert.equal(contentHeight(825 * 0.28, 825), 825, "scale(0.42, 0.28) na začátku animace");
  assert.equal(contentHeight(825 * 1.03, 825), 825, "překmit pružiny");
});
