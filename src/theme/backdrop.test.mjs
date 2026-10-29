// Testy čisté logiky pozadí šuplíku (kolo 25): `node --experimental-strip-types --test src/theme/`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { THEMES, THEME_IDS } from "./themes.ts";
import { textProblems, contrast, parseColor } from "./contrast.ts";
import { PRESETS, HIDDEN, LIVE_PRESETS, FLAME_COLORS, presetById } from "./backdropPresets.ts";
import {
  DEFAULT_BACKDROP, VEIL_MAX, cardAlpha, cardContrast, cardMode, cardVars, accentContrast, accentFromGradient, addGradient, addStop, backdropVars, colorAt, drawMode, fitAccent, gradientCss,
  gradientOf, isDefaultBackdrop, meshPoints, normalizeBackdrop, normalizeGradient, onAccent, removeStop, sampleColors, veilFor,
} from "./backdrop.ts";
import { DEFAULT_APPEARANCE, appearanceVars, normalizeAppearance } from "./appearance.ts";

const g2 = { kind: "linear", angle: 90, stops: [{ c: "#000000", p: 0 }, { c: "#ffffff", p: 100 }] };
const asG = (p) => ({ kind: p.kind, angle: p.angle, stops: p.stops.map(([c, pos]) => ({ c, p: pos })) });

test("hotových pozadí je aspoň 10 gradientů a 6 živých, id jsou jedinečná", () => {
  assert.ok(PRESETS.filter((p) => p.group === "gradient").length >= 10);
  assert.equal(LIVE_PRESETS.length, 6);
  const ids = [...PRESETS, ...HIDDEN].map((p) => p.id);
  assert.equal(new Set(ids).size, ids.length);
  for (const l of LIVE_PRESETS) assert.ok(presetById(l.base), `základ živého ${l.id}`);
  const kinds = new Set(PRESETS.map((p) => p.kind));
  for (const k of ["linear", "radial", "conic", "mesh"]) assert.ok(kinds.has(k), k);
  for (const p of PRESETS) assert.ok(normalizeGradient({ id: "x", name: "x", ...asG(p) }), p.id);
});

test("normalizace: starý config / neplatné = výchozí, platné projde", () => {
  const cases = [
    ["undefined", undefined, DEFAULT_BACKDROP],
    ["prázdný objekt", {}, DEFAULT_BACKDROP],
    ["neznámé id", { id: "neexistuje" }, DEFAULT_BACKDROP],
    ["neznámé živé", { live: "snow" }, DEFAULT_BACKDROP],
    ["síla mimo", { intensity: 9, speed: 0 }, DEFAULT_BACKDROP],
    ["hotový", { id: "sunset" }, { ...DEFAULT_BACKDROP, id: "sunset" }],
    ["oheň modrý", { live: "fire", flame: "blue", intensity: 0.8 }, { ...DEFAULT_BACKDROP, live: "fire", flame: "blue", intensity: 0.8 }],
  ];
  for (const [name, raw, want] of cases) assert.deepEqual(normalizeBackdrop(raw), want, name);
  assert.ok(isDefaultBackdrop(normalizeBackdrop({ id: "x" })));
  const c = { id: "my-a", name: " Můj ", kind: "conic", angle: 400, stops: g2.stops };
  assert.equal(normalizeBackdrop({ id: "my-a", custom: [c] }).custom[0].angle, 0, "úhel mimo → 0");
  assert.equal(normalizeBackdrop({ id: "my-a", custom: [c] }).id, "my-a");
  assert.equal(normalizeBackdrop({ id: "my-a", custom: [{ ...c, id: "sunset" }] }).custom.length, 0, "id hotového se nepřepíše");
  assert.equal(normalizeBackdrop({ custom: [{ ...c, stops: [g2.stops[0]] }] }).custom.length, 0, "1 zastávka");
  assert.equal(normalizeBackdrop({ custom: Array.from({ length: 30 }, (_, i) => ({ ...c, id: `my-${i}` })) }).custom.length, 20);
});

test("appearance bez backdropu zůstane beze změny a nevydá žádné proměnné pozadí", () => {
  const a = normalizeAppearance({ theme: "oled", material: "solid" });
  assert.equal(a.backdrop, undefined);
  assert.equal(a.accent2, undefined);
  const v = appearanceVars(a);
  assert.ok(!("bd-img" in v) && !("bd-veil" in v) && !("accent-grad" in v));
  assert.deepEqual(appearanceVars(DEFAULT_APPEARANCE), appearanceVars(normalizeAppearance({})));
});

test("colorAt: interpolace zastávek", () => {
  const cases = [[-5, "#000000"], [0, "#000000"], [50, "#808080"], [100, "#ffffff"], [140, "#ffffff"], [25, "#404040"]];
  for (const [p, want] of cases) assert.equal(colorAt(g2.stops, p), want, `p=${p}`);
  assert.equal(colorAt([{ c: "#ff0000", p: 50 }], 10), "#ff0000");
  assert.equal(colorAt([{ c: "#ffffff", p: 100 }, { c: "#000000", p: 0 }], 50), "#808080", "neseřazené");
});

test("gradientCss: čtyři druhy", () => {
  assert.equal(gradientCss(g2), "linear-gradient(90deg, #000000 0%, #ffffff 100%)");
  assert.equal(gradientCss({ ...g2, kind: "radial", angle: 0 }), "radial-gradient(ellipse 140% 140% at 50% 5%, #000000 0%, #ffffff 100%)");
  assert.match(gradientCss({ ...g2, kind: "radial", angle: 180 }), /at 50% 95%/);
  assert.equal(gradientCss({ ...g2, kind: "conic", angle: 45 }), "conic-gradient(from 45deg at 50% 40%, #000000 0%, #ffffff 100%)", "poslední je na 100 → nic se nepřidává");
  assert.match(gradientCss({ ...g2, kind: "conic", angle: 0, stops: [{ c: "#112233", p: 0 }, { c: "#445566", p: 60 }] }), /#445566 60%, #112233 100%\)$/, "kužel se uzavře první barvou");
  const mesh = gradientCss({ kind: "mesh", angle: 0, stops: [{ c: "#ff0000", p: 50 }, { c: "#00ff00", p: 50 }, { c: "#0000ff", p: 50 }] });
  assert.equal((mesh.match(/radial-gradient/g) ?? []).length, 3);
  assert.match(mesh, /linear-gradient\(0deg, #[0-9a-f]{6}, #[0-9a-f]{6}\)$/, "tmavý základ");
  assert.equal(gradientCss({ ...g2, stops: [] }), "none");
});

test("meshPoints: bez otočení jsou na místě, otočení o 180° zrcadlí", () => {
  assert.deepEqual(meshPoints(2, 0), [[14, 18], [86, 14]]);
  assert.deepEqual(meshPoints(1, 180), [[86, 82]]);
  assert.equal(meshPoints(9, 0).length, 5);
});

test("zastávky: přidání do největší mezery, odebrání, minimum 2 a maximum 5", () => {
  let s = addStop(g2.stops);
  assert.deepEqual(s.map((x) => x.p), [0, 50, 100]);
  assert.equal(s[1].c, "#808080");
  s = addStop(addStop(addStop(s)));
  assert.equal(s.length, 5);
  assert.equal(addStop(s).length, 5);
  assert.equal(removeStop(g2.stops, 0).length, 2);
  assert.equal(removeStop(s, 2).length, 4);
});

test("vlastní gradienty: přidání, id bez kolize, strop 20", () => {
  const a = addGradient([], "Můj západ!", g2);
  assert.equal(a.id, "my-muj-zapad");
  const b = addGradient(a.list, "Můj západ!", g2);
  assert.equal(b.id, "my-muj-zapad-2");
  assert.equal(addGradient([], "", g2).list[0].name, "Gradient");
  const full = Array.from({ length: 20 }, (_, i) => normalizeGradient({ id: `my-${i}`, name: "n", ...g2 }));
  assert.equal(addGradient(full, "x", g2), null);
  assert.equal(gradientOf({ ...DEFAULT_BACKDROP, id: a.id, custom: a.list }).kind, "linear");
});

// ---------- čitelnost: závoj ----------
test("závoj: každé hotové pozadí je ve VŠECH motivech čitelné (nebo má nejvyšší závoj)", () => {
  const report = [];
  for (const id of THEME_IDS) {
    const vars = THEMES[id].vars;
    for (const p of [...PRESETS, ...HIDDEN]) {
      const cols = sampleColors(asG(p));
      const veil = veilFor(cols, vars);
      assert.ok(veil >= 0.1 && veil <= VEIL_MAX, `${id}/${p.id} ${veil}`);
      if (veil < VEIL_MAX) {
        for (const c of cols) {
          const bg = parseColor(vars.bg);
          const mixed = [0, 1, 2].map((i) => bg[i] * veil + parseColor(c)[i] * (1 - veil));
          const v = { ...vars, bg: mixed.map(Math.round).join(" ") };
          assert.deepEqual(textProblems(v), [], `${id}/${p.id}/${c} veil=${veil}`);
        }
      }
      report.push(`${id}/${p.id}=${veil}`);
    }
  }
  if (process.env.BD_REPORT) console.log(report.join("\n"));
});

test("závoj: oled (hlavní motiv) drží hotové gradienty výrazné (závoj ≤ 0.55)", () => {
  const vars = THEMES.oled.vars;
  for (const p of PRESETS) {
    const v = veilFor(sampleColors(asG(p)), vars);
    assert.ok(v <= 0.55, `${p.id}=${v}`);
  }
});

test("závoj: bílý gradient na tmavém motivu dostane nejvyšší závoj, černý nejnižší", () => {
  const vars = THEMES.oled.vars;
  assert.ok(veilFor(["#ffffff"], vars) >= 0.7);
  assert.equal(veilFor(["#000000"], vars), 0.1);
  assert.ok(veilFor(["#808080"], vars) > veilFor(["#303030"], vars));
});

test("backdropVars: bez nastavení nic, s gradientem img + závoj, jen živé = závoj bez obrázku", () => {
  const vars = THEMES.oled.vars;
  assert.deepEqual(backdropVars(DEFAULT_BACKDROP, vars), {});
  const v = backdropVars({ ...DEFAULT_BACKDROP, id: "midnight" }, vars);
  assert.match(v["bd-img"], /^linear-gradient/);
  assert.ok(+v["bd-veil"] > 0);
  const live = backdropVars({ ...DEFAULT_BACKDROP, live: "fire" }, vars);
  assert.ok(!("bd-img" in live) && "bd-veil" in live);
});

// ---------- akcent ----------
test("akcent z gradientu: sytá barva rozjasněná do čitelného pásma, šedý gradient nic", () => {
  assert.equal(accentFromGradient(g2), null);
  const [a, b] = accentFromGradient({ kind: "linear", angle: 0, stops: [{ c: "#1c1a4a", p: 0 }, { c: "#a04a12", p: 100 }] });
  assert.match(a, /^#[0-9a-f]{6}$/);
  assert.notEqual(a, b);
  for (const p of PRESETS.filter((x) => x.group === "gradient")) {
    const r = accentFromGradient(asG(p));
    assert.ok(r, p.id);
    for (const hex of r) assert.ok(accentContrast(fitAccent(hex, THEMES.oled.vars), THEMES.oled.vars) >= 4.5, `${p.id} ${hex}`);
  }
});

test("fitAccent: ve všech motivech dá ≥ 4.4:1 na kartách, dobrý akcent nechá", () => {
  for (const id of THEME_IDS) {
    const vars = THEMES[id].vars;
    for (const hex of ["#8b5cf6", "#3b82f6", "#f59e0b", "#14b8a6", "#ef4444", "#22c55e", "#ec4899", "#1d2a5a", "#f4f1ff"]) {
      const f = fitAccent(hex, vars);
      const c = accentContrast(f, vars);
      assert.ok(c >= 4.4, `${id} ${hex}→${f} ${c.toFixed(2)}`);
    }
  }
  assert.equal(fitAccent("#F59E0B", THEMES.oled.vars), "#f59e0b");
});

test("onAccent: bílá na tmavé, černá na světlé", () => {
  const cases = [["#1d2a5a", "#ffffff"], ["#fde68a", "#000000"], ["#4c1d95", "#ffffff"], ["#22c55e", "#000000"]];
  for (const [hex, want] of cases) assert.equal(onAccent(hex), want, hex);
});

test("barvy plamene: čtyři a vzestupně světlé", () => {
  const lum = (h) => contrast(parseColor(h), [0, 0, 0, 1]);
  for (const [k, [a, b, c]] of Object.entries(FLAME_COLORS)) assert.ok(lum(a) < lum(b) && lum(b) < lum(c), k);
  assert.equal(Object.keys(FLAME_COLORS).length, 4);
});

// ---------- kreslit / nekreslit ----------
test("drawMode: kreslí jen otevřené a viditelné, reduced = snímek, zavřené = stop", () => {
  const base = { open: true, hidden: false, reduced: false, live: true };
  const cases = [
    ["otevřené", base, "run"],
    ["zavřené", { ...base, open: false }, "stop"],
    ["okno skryté", { ...base, hidden: true }, "stop"],
    ["reduced-motion", { ...base, reduced: true }, "still"],
    ["reduced a zavřené", { ...base, reduced: true, open: false }, "stop"],
    ["bez živého pozadí", { ...base, live: false }, "stop"],
  ];
  for (const [name, s, want] of cases) assert.equal(drawMode(s), want, name);
});

// ---------- smyčka: zastavení ----------
import { createLoop } from "./backdropLoop.ts";

function fakeClock() {
  let now = 0, id = 0;
  const q = new Map();
  return {
    deps: { now: () => now, request: (cb) => (q.set(++id, cb), id), cancel: (h) => q.delete(h) },
    advance(ms) {
      now += ms;
      const cbs = [...q.values()];
      q.clear();
      cbs.forEach((cb) => cb());
    },
    queued: () => q.size,
  };
}

test("smyčka: kreslí nejvýš 30 fps, po stop() nemá nic naplánováno a nekreslí", () => {
  const clk = fakeClock();
  const calls = [];
  const loop = createLoop({ ...clk.deps, frame: (t, dt) => calls.push([t, dt]) }, 30);
  assert.equal(clk.queued(), 0, "před startem nic");
  loop.start();
  for (let i = 0; i < 60; i++) clk.advance(1000 / 60); // 1 s při 60 Hz
  assert.ok(calls.length >= 28 && calls.length <= 31, `snímků ${calls.length}`);
  loop.stop();
  assert.equal(clk.queued(), 0, "po stop() žádný čekající rAF");
  assert.equal(loop.pending(), false);
  const n = calls.length;
  for (let i = 0; i < 120; i++) clk.advance(16);
  assert.equal(calls.length, n, "zastavená smyčka nekreslí");
});

test("smyčka: nižší fps, dlouhá prodleva nezkreslí dt, opakovaný start/stop je bezpečný", () => {
  const clk = fakeClock();
  const calls = [];
  const loop = createLoop({ ...clk.deps, frame: (t, dt) => calls.push(dt) }, 12);
  loop.start();
  loop.start();
  assert.equal(clk.queued(), 1, "dvojí start = jeden plán");
  for (let i = 0; i < 60; i++) clk.advance(1000 / 60);
  assert.ok(calls.length >= 11 && calls.length <= 13, `snímků ${calls.length}`);
  clk.advance(5000);
  assert.ok(Math.max(...calls) <= 0.1 + 1e-9, "dt omezeno");
  loop.stop();
  loop.stop();
  assert.equal(clk.queued(), 0);
  loop.start();
  assert.equal(clk.queued(), 1, "znovu spustitelná");
  loop.stop();
});

import { moveStop, posFromPointer, removeGradient, updateGradient } from "./backdrop.ts";

test("moveStop: tažení přes sousední zastávku změní index, mez 0–100", () => {
  const s = [{ c: "#111111", p: 0 }, { c: "#222222", p: 50 }, { c: "#333333", p: 100 }];
  const cases = [
    ["jen posun", 1, 60, [0, 60, 100], 1],
    ["přes pravého souseda", 0, 80, [50, 80, 100], 1],
    ["na samý konec", 0, 100, [50, 100, 100], 2],
    ["mimo rozsah", 2, 150, [0, 50, 100], 2],
    ["zaokrouhlení", 1, 33.4, [0, 33, 100], 1],
    ["záporné", 1, -10, [0, 0, 100], 1],
  ];
  for (const [name, i, p, want, idx] of cases) {
    const r = moveStop(s, i, p);
    assert.deepEqual(r.stops.map((x) => x.p), want, name);
    assert.equal(r.index, idx, name);
  }
  assert.equal(moveStop(s, 0, 80).stops[1].c, "#111111", "barva se pohybuje s zastávkou");
});

test("posFromPointer, updateGradient a removeGradient", () => {
  assert.equal(posFromPointer(150, 100, 200), 25);
  assert.equal(posFromPointer(50, 100, 200), 0);
  assert.equal(posFromPointer(500, 100, 200), 100);
  assert.equal(posFromPointer(5, 0, 0), 0);
  const a = addGradient([], "A", g2);
  const up = updateGradient(a.list, a.id, { kind: "conic", angle: 200, name: " Nový " });
  assert.deepEqual([up[0].kind, up[0].angle, up[0].name], ["conic", 200, "Nový"]);
  assert.equal(updateGradient(a.list, a.id, { stops: [g2.stops[0]] })[0].stops.length, 2, "neplatná změna se ignoruje");
  assert.equal(updateGradient(a.list, "jine", { angle: 5 })[0].angle, a.list[0].angle);
  const b = removeGradient({ ...DEFAULT_BACKDROP, id: a.id, custom: a.list }, a.id);
  assert.deepEqual([b.id, b.custom.length], ["", 0]);
  assert.equal(removeGradient({ ...DEFAULT_BACKDROP, id: "sunset", custom: a.list }, a.id).id, "sunset");
});

test("karty nad pozadím: auto = průsvitné jen s pozadím, bez pozadí se nic nemění", () => {
  const cases = [
    ["bez pozadí, auto", { ...DEFAULT_BACKDROP }, "solid"],
    ["bez pozadí, tint", { ...DEFAULT_BACKDROP, cards: "tint" }, "solid"],
    ["gradient, auto", { ...DEFAULT_BACKDROP, id: "sunset" }, "tint"],
    ["živé, auto", { ...DEFAULT_BACKDROP, live: "rain" }, "tint"],
    ["gradient, solid", { ...DEFAULT_BACKDROP, id: "sunset", cards: "solid" }, "solid"],
    ["gradient, glass", { ...DEFAULT_BACKDROP, id: "sunset", cards: "glass" }, "glass"],
  ];
  for (const [n, b, want] of cases) assert.equal(cardMode(b), want, n);
  assert.equal(cardMode(null), "solid");
  assert.equal(normalizeBackdrop({ cards: "foam" }).cards, "auto");
  assert.equal(normalizeBackdrop({ cards: "glass" }).cards, "glass");
  assert.equal(isDefaultBackdrop(normalizeBackdrop({ cards: "glass" })), false);
  assert.deepEqual(cardVars({ ...DEFAULT_BACKDROP }, { bg: "0 0 0", "bd-veil": "0.3" }), {});
});

test("krytí karet je 55–70 % podle závoje", () => {
  for (const [veil, want] of [[0, 0.55], [0.1, 0.55], [0.51, 0.63], [0.92, 0.7], [1, 0.7]]) assert.equal(cardAlpha(veil), want, String(veil));
});

test("text na průsvitné kartě má kontrast ≥ 4.5:1 nad všemi pozadími a motivy", () => {
  const presets = [...PRESETS, ...HIDDEN];
  for (const id of THEME_IDS) {
    const vars = THEMES[id].vars;
    for (const p of presets) {
      const b = { ...DEFAULT_BACKDROP, id: p.id };
      const v = backdropVars(b, vars);
      const veil = Number(v["bd-veil"]);
      for (const c of sampleColors(gradientOf(b))) {
        const k = cardContrast(vars, c, veil);
        if (veil < VEIL_MAX) assert.ok(k >= 4.5, `${id} / ${p.id} / ${c}: ${k.toFixed(2)}`);
      }
    }
  }
});
