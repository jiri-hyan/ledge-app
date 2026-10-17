// Testy čisté logiky vzhledu (bez DOM, bez závislostí): `node --test src/theme/` (npm run test:ui).
// Node 24 načte .ts přímo (odstraní typy). Hlídá: motiv „dark“ = tokeny ze styles.css, úplnost motivů,
// kontrast textů ≥ 4.5:1 ve všech motivech, odvození akcentu, pořadí karet, tvar proužku = Rust.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { THEMES, THEME_IDS, COLOR_KEYS } from "./themes.ts";
import {
  DEFAULT_APPEARANCE, accentPair, appearanceVars, cardOrder, effectiveMotion, hexToHsl, normalizeAppearance, springPreset, stripShape, widenMotion,
} from "./appearance.ts";

const css = readFileSync(new URL("../styles.css", import.meta.url), "utf8");
const rootBlock = css.slice(css.indexOf(":root {"), css.indexOf("\n}\n"));
const cssVar = (k) => rootBlock.match(new RegExp(`\\n\\s*--${k}:\\s*([^;]+);`))?.[1].trim();

// ---------- barvy a kontrast (WCAG) ----------
function parse(c) {
  c = c.trim();
  let m = c.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
  if (m) {
    const h = m[1].length === 3 ? [...m[1]].map((x) => x + x).join("") : m[1];
    const n = parseInt(h, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  m = c.match(/^rgba?\(([^)]+)\)$/);
  if (m) {
    const p = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
    return [p[0], p[1], p[2], p[3] ?? 1];
  }
  m = c.match(/^(\d+) (\d+) (\d+)$/); // --bg „r g b“
  if (m) return [+m[1], +m[2], +m[3], 1];
  throw new Error(`neznámá barva ${c}`);
}
const over = (top, bottom) => {
  const a = top[3];
  return [0, 1, 2].map((i) => top[i] * a + bottom[i] * (1 - a)).concat(1);
};
const lum = ([r, g, b]) => {
  const f = (v) => ((v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
};
const contrast = (a, b) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

test("motiv dark = tokeny ze styles.css (výchozí vzhled se nezmění)", () => {
  for (const k of COLOR_KEYS) {
    if (k === "focus") continue; // v CSS jako var(--blue)
    const v = cssVar(k);
    assert.ok(v, `--${k} chybí v :root`);
    assert.equal(THEMES.dark.vars[k], v, `--${k}`);
  }
  assert.equal(THEMES.dark.vars.focus, cssVar("blue"));
});

test("každý motiv má všechny barevné klíče", () => {
  for (const id of THEME_IDS) {
    for (const k of COLOR_KEYS) assert.ok(THEMES[id].vars[k], `${id}: --${k}`);
    assert.equal(THEMES[id].id, id);
  }
});

test("kontrast textů ≥ 4.5:1 ve všech motivech (karta i hover, nad tmavou i světlou plochou)", () => {
  for (const id of THEME_IDS) {
    const v = THEMES[id].vars;
    const bg = parse(v.bg);
    for (const desk of [[0, 0, 0, 1], [255, 255, 255, 1]]) {
      const shell = over([...bg.slice(0, 3), 0.985], desk);
      for (const layer of ["card", "card-2"]) {
        const surface = over(parse(v[layer]), shell);
        for (const t of ["t1", "t2", "t3", "green", "amber", "red", "blue", "violet", "teal", "amber-ink", "red-ink"]) {
          const k = contrast(parse(v[t]), surface);
          assert.ok(k >= 4.5, `${id}: --${t} na --${layer} = ${k.toFixed(2)}`);
        }
      }
      // tlačítko .btn.pri: text --on-light na ploše --t1
      const k = contrast(parse(v["on-light"]), parse(v.t1));
      assert.ok(k >= 4.5, `${id}: .btn.pri ${k.toFixed(2)}`);
    }
  }
});

test("výchozí vzhled → škály 1 a barvy motivu dark", () => {
  const vars = appearanceVars(DEFAULT_APPEARANCE);
  for (const k of ["fz", "dens", "rr", "glass", "speed"]) assert.equal(vars[k], "1", k);
  assert.equal(vars.accent, "#a78bfa");
  assert.equal(vars.spring, cssVar("spring"));
  assert.equal(vars.ease, cssVar("ease"));
});

test("vlastní akcent přepíše akcent, druhou barvu i fokus", () => {
  const vars = appearanceVars({ ...DEFAULT_APPEARANCE, theme: "light", accent: "#22C55E" });
  assert.equal(vars.accent, "#22c55e");
  assert.equal(vars.focus, "#22c55e");
  const [h1] = hexToHsl("#22c55e");
  const [h2] = hexToHsl(vars["accent-2"]);
  assert.ok(Math.abs(((h1 - h2 + 360) % 360) - 40) < 2, `posun odstínu ${h1} → ${h2}`);
  assert.deepEqual(accentPair("#ff0000")[0], "#ff0000");
});

test("normalizace: neplatné hodnoty → výchozí, chybějící doplněné", () => {
  assert.deepEqual(normalizeAppearance(undefined), DEFAULT_APPEARANCE);
  assert.deepEqual(normalizeAppearance({}), DEFAULT_APPEARANCE);
  const a = normalizeAppearance({ theme: "neon", accent: "red", fontScale: 3, motion: { style: "none" }, strip: { color: "accent" }, cards: ["notes", "x"] });
  assert.equal(a.theme, "dark");
  assert.equal(a.accent, "");
  assert.equal(a.fontScale, 1);
  assert.deepEqual(a.motion, { style: "none", speed: 1 });
  assert.equal(a.strip.color, "accent");
  assert.deepEqual(a.cards, ["notes"]);
});

test("pořadí karet", () => {
  const cases = [
    [[], ["running", "claude", "repos", "notes"]],
    [["notes"], ["notes", "running", "claude", "repos"]],
    [["repos", "repos", "x", "claude"], ["repos", "claude", "running", "notes"]],
  ];
  for (const [inp, want] of cases) assert.deepEqual(cardOrder(inp), want, JSON.stringify(inp));
});

test("tvar proužku = appearance::geom::strip_metrics", () => {
  assert.deepEqual(stripShape("pill", "normal"), { h: 32, r: 14 });
  assert.deepEqual(stripShape("line", "large"), { h: 26 * 1.2, r: 3 * 1.2 });
  assert.deepEqual(stripShape("dot", "small"), { h: 22 * 0.85, r: 11 * 0.85 });
});

test("presety pružin", () => {
  assert.equal(springPreset("none", 240, 25), null);
  assert.deepEqual(springPreset("spring", 240, 25), { stiffness: 240, damping: 25 });
  const smooth = springPreset("smooth", 240, 25);
  assert.ok(smooth.damping >= 2 * Math.sqrt(240), "smooth = kriticky tlumená (bez překmitu)");
  const snappy = springPreset("snappy", 240, 25);
  assert.ok(snappy.stiffness > 240);
  assert.equal(effectiveMotion("spring", true), "none");
  assert.equal(effectiveMotion("snappy", false), "snappy");
});

test("přechod rozšíření Nastavení podle stylu pohybu", () => {
  const ease = (style) => appearanceVars({ ...DEFAULT_APPEARANCE, motion: { style, speed: 1 } }).ease;
  // [styl, rychlost, omezit pohyb, očekávání: null = okamžitě | [zúžení, zmizení, návrat] ms]
  const cases = [
    ["spring", 1, false, [210, 120, 160]],
    ["smooth", 1, false, [210, 120, 160]],
    ["snappy", 2, false, [105, 60, 80]],
    ["spring", 0.5, false, [420, 240, 320]],
    ["spring", 0, false, [210, 120, 160]], // nesmyslná rychlost → 1
    ["none", 1, false, null],
    ["none", 2, true, null],
    ["spring", 1, true, null], // systémové „omezit pohyb“
    ["snappy", 1.5, true, null],
  ];
  for (const [style, speed, reduced, want] of cases) {
    const m = widenMotion(style, speed, reduced);
    if (want === null) {
      assert.equal(m, null, `${style} ${speed} ${reduced}`);
      continue;
    }
    assert.deepEqual([m.shrinkMs, m.fadeMs, m.restoreMs], want, `${style} ${speed}`);
    assert.equal(m.easing, ease(style), `křivka ${style} = --ease stylu`);
  }
});
