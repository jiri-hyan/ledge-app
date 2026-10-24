// Testy čisté logiky vzhledu (bez DOM, bez závislostí): `node --test src/theme/` (npm run test:ui).
// Node 24 načte .ts přímo (odstraní typy). Hlídá: motiv „dark“ = tokeny ze styles.css, úplnost motivů,
// kontrast textů ≥ 4.5:1 ve všech motivech, odvození akcentu, tvar proužku = Rust, design styly a plány animací.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { THEMES, THEME_IDS, COLOR_KEYS } from "./themes.ts";
import { textProblems } from "./contrast.ts";
import {
  DEFAULT_APPEARANCE, accentPair, appearanceVars, effectiveMotion, hexToHsl, normalizeAppearance, springPreset, stripShape, widenMotion,
} from "./appearance.ts";

const css = readFileSync(new URL("../styles.css", import.meta.url), "utf8");
const rootBlock = css.slice(css.indexOf(":root {"), css.indexOf("\n}\n"));
const cssVar = (k) => rootBlock.match(new RegExp(`\\n\\s*--${k}:\\s*([^;]+);`))?.[1].trim();

// ---------- barvy a kontrast (WCAG): jedna implementace s editorem vlastních motivů (`contrast.ts`) ----------

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
    const bad = textProblems(THEMES[id].vars);
    assert.deepEqual(bad, [], `${id}: ${bad.map((p) => `--${p.key} ${p.ratio.toFixed(2)}`).join(", ")}`);
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
  const a = normalizeAppearance({ theme: "rainbow", accent: "red", fontScale: 3, motion: { style: "none" }, strip: { color: "accent" } });
  assert.equal(a.theme, "dark");
  assert.equal(a.accent, "");
  assert.equal(a.fontScale, 1);
  assert.deepEqual(a.motion, { style: "none", speed: 1 });
  assert.equal(a.strip.color, "accent");
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

// ---------- design styly (vlna 6) ----------
import { CLASSIC, FONTS, LAYOUTS, MATERIALS, STYLES, STYLE_IDS, WIDTHS, lookVars, spans, styleOf } from "./looks.ts";
import { MOTION_STYLES, STRIP_STYLES } from "./appearance.ts";
import { configWithStyle, styleChanges, withStyle } from "./styles.ts";
import { CLOSE_CAP_MS, closePlan, openPlan } from "./openAnim.ts";

const rust = (f) => readFileSync(new URL(`../../src-tauri/src/${f}`, import.meta.url), "utf8");
/** Varianty Rust enumu (camelCase jako serde) — zrcadla TS tabulek. */
function rustEnum(src, name) {
  const body = src.slice(src.indexOf(`pub enum ${name} {`)).split("\n}")[0];
  return [...body.matchAll(/^\s{4}([A-Z]\w*),?\r?$/gm)].map((m) => m[1][0].toLowerCase() + m[1].slice(1));
}
/** Výchozí hodnoty `var(--k, výchozí)` ze styles.css (vyvážené závorky). */
function cssFallbacks() {
  const out = {};
  const re = /var\(--([a-z0-9-]+), /g;
  let m;
  while ((m = re.exec(css))) {
    let depth = 1;
    let i = re.lastIndex;
    for (; i < css.length && depth; i++) depth += css[i] === "(" ? 1 : css[i] === ")" ? -1 : 0;
    (out[m[1]] ??= new Set()).add(css.slice(re.lastIndex, i - 1).trim());
  }
  return out;
}

test("výchozí hodnoty proměnných stylu v styles.css = Klasický styl (dnešní vzhled)", () => {
  const classic = lookVars(DEFAULT_APPEARANCE);
  const fb = cssFallbacks();
  const checked = [];
  for (const [k, vals] of Object.entries(fb)) {
    if (!(k in classic)) continue;
    for (const v of vals) assert.equal(v, classic[k], `--${k}: výchozí v CSS „${v}“ ≠ Klasický „${classic[k]}“`);
    checked.push(k);
  }
  for (const k of ["m-card-bg", "m-card-bd", "m-h-color", "h-font", "h-tt", "m-glow", "deck-gap", "l-pad", "m-pill-r", "m-ctl-w"]) assert.ok(checked.includes(k), `--${k} se v CSS čte`);
  assert.equal(classic.sans, cssVar("sans"));
  assert.equal(classic.display, cssVar("display"));
  assert.equal(classic["deck-cols"], undefined, "Klasický nepřebíjí sloupce (.grid 3, .deck 4)");
});

test("starý config z vlny 5 (bez stylu) vypadá stejně: styl Klasický, stejné proměnné", () => {
  const old = { theme: "nord", accent: "#22c55e", density: "compact", fontScale: 1.1, radius: "round", glass: { opacity: 0.8 }, motion: { style: "smooth", speed: 1.5 }, strip: { style: "line", size: "large", color: "accent", position: "left" } };
  const a = normalizeAppearance(old);
  assert.deepEqual([a.style, a.font, a.material, a.layout, a.width], ["classic", "system", "solid", "classic", "custom"]);
  const { style, font, material, layout, width, pages, customThemes, customTheme, ...rest } = a;
  assert.deepEqual([pages, customThemes, customTheme], [{}, [], ""], "starý config nemá záložky ani vlastní motivy");
  assert.deepEqual(rest, old, "vlastní hodnoty zůstaly");
  const vars = appearanceVars(a);
  for (const [k, v] of Object.entries(lookVars(DEFAULT_APPEARANCE))) assert.equal(vars[k], v, `--${k}`);
  assert.equal(vars.dens, "0.78");
  assert.deepEqual(styleChanges(a, true).sort(), ["colors", "density", "font", "material", "shape", "strip"].sort(), "jeho úpravy proti Klasickému");
});

test("tabulka stylů je úplná a platná", () => {
  assert.equal(STYLES[0].id, CLASSIC);
  assert.equal(new Set(STYLE_IDS).size, STYLES.length);
  for (const s of STYLES) {
    const p = s.parts;
    assert.match(s.id, /^[a-z]{1,24}$/, "jméno projde validací v Rustu (look::style_id_ok)");
    assert.equal(s.name[0], `appearance.style.${s.id}`);
    assert.equal(s.desc[0], `appearance.style.${s.id}.desc`);
    assert.ok(THEME_IDS.includes(p.theme) && p.font in FONTS && p.material in MATERIALS && p.layout in LAYOUTS && p.width in WIDTHS, s.id);
    assert.ok(STRIP_STYLES.includes(p.strip), s.id);
    assert.ok(p.opacity >= 0.25 && p.opacity <= 1 && p.fontScale >= 0.85 && p.fontScale <= 1.25, s.id);
    // výběr stylu = žádná složka „upravená“
    assert.deepEqual(styleChanges(withStyle({}, s.id), p.blur), [], s.id);
  }
  // 6 schválených stylů + Klasický + 3 nové (kolo 13); sestava motiv + rozložení + materiál + písmo je u každého jiná,
  // každé rozložení je aspoň jednou použité
  // + široký notch (kolo 22) = jediný styl s rozložením `notch`
  assert.deepEqual(STYLE_IDS, ["classic", "glass", "terminal", "paper", "neon", "brutal", "pastel", "aurora", "mono", "retro", "fluent", "oled", "warm", "notch"]);
  assert.deepEqual(STYLES.filter((s) => s.parts.layout === "notch").map((s) => s.id), ["notch"]);
  const combo = (s) => [s.parts.theme, s.parts.layout, s.parts.material, s.parts.font].join("|");
  assert.equal(new Set(STYLES.map(combo)).size, STYLES.length);
  assert.deepEqual([...new Set(STYLES.map((s) => s.parts.layout))].sort(), Object.keys(LAYOUTS).sort());
  // nový styl = vlastní motiv (barvy) a statická dekorace v CSS
  for (const id of ["aurora", "mono", "retro", "oled", "warm"]) {
    assert.equal(styleOf(id).parts.theme, id, `${id}: vlastní motiv`);
    assert.ok(css.includes(`html[data-style="${id}"]`), `${id}: blok v styles.css`);
  }
  // Fluent (kolo 18) = tmavý motiv + akryl + vlastní rozložení Panel a blok dekorace
  assert.deepEqual([styleOf("fluent").parts.material, styleOf("fluent").parts.layout, styleOf("fluent").parts.blur], ["acrylic", "panel", true]);
  assert.ok(css.includes('html[data-style="fluent"]'), "fluent: blok v styles.css");
  assert.equal(styleOf("neexistuje").id, CLASSIC);
});

test("výběr stylu přepíše jen svoje složky; pozice proužku, linka a animace zůstanou", () => {
  const before = normalizeAppearance({ theme: "light", accent: "#ff0000", motion: { style: "bounce", speed: 2 }, strip: { position: "right", color: "#00ff00" } });
  const a = withStyle(before, "neon");
  assert.deepEqual([a.style, a.theme, a.accent, a.material, a.layout, a.width, a.strip.style], ["neon", "neon", "", "glow", "panorama", "wide", "glow"]);
  assert.deepEqual([a.strip.position, a.strip.color, a.motion], ["right", "#00ff00", { style: "bounce", speed: 2 }]);
  // úprava detailu je vidět a „Obnovit styl“ (= znovu withStyle) ji vrátí
  const tweaked = { ...a, font: "mono", radius: "sharp" };
  assert.deepEqual(styleChanges(tweaked, false), ["font", "shape"]);
  assert.deepEqual(styleChanges(withStyle(tweaked, tweaked.style), false), []);
  // config z palety: vstup se nemění, rozmazání podle stylu
  const cfg = { drawer: { width: 1300, glass: "acrylic" }, appearance: before };
  const next = configWithStyle(cfg, "terminal");
  assert.equal(cfg.appearance.theme, "light");
  assert.deepEqual([next.drawer.width, next.drawer.glass, next.appearance.width, next.appearance.font], [1300, "solid", "full", "mono"]);
  assert.equal(configWithStyle({}, "glass").appearance.layout, "float");
});

test("proměnné stylu: vždy úplná sada (náhled nezdědí kořen), sloupce a rozpětí karet", () => {
  const keys = (s) => Object.keys(lookVars(withStyle({}, s))).filter((k) => !/^(deck-cols|span-)/.test(k)).sort();
  for (const s of STYLE_IDS) assert.deepEqual(keys(s), keys(CLASSIC), s);
  for (const [id, l] of Object.entries(LAYOUTS)) {
    const v = lookVars({ ...DEFAULT_APPEARANCE, layout: id });
    if (l.cols === null) continue;
    assert.equal(v["deck-cols"], String(l.cols), id);
    for (const k of ["s", "m", "l", "xl"]) assert.ok(+v[`span-${k}`] <= l.cols && +v[`span-${k}`] >= 1, `${id} span-${k}`);
  }
  assert.deepEqual(spans(2), { s: 1, m: 2, l: 2, xl: 2 });
  assert.deepEqual(spans(5), { s: 1, m: 2, l: 3, xl: 5 });
});

test("zrcadla Rustu: motivy, animace, proužek, písmo, materiál, rozložení, šířky", () => {
  const app = rust("appearance.rs");
  const look = rust("appearance/look.rs");
  assert.deepEqual(rustEnum(app, "Theme").sort(), [...THEME_IDS].sort());
  assert.deepEqual(rustEnum(app, "MotionStyle").sort(), [...MOTION_STYLES].sort());
  assert.deepEqual(rustEnum(app, "StripStyle").sort(), [...STRIP_STYLES].sort());
  assert.deepEqual(rustEnum(look, "FontLook").sort(), Object.keys(FONTS).sort());
  assert.deepEqual(rustEnum(look, "Material").sort(), Object.keys(MATERIALS).sort());
  assert.deepEqual(rustEnum(look, "Layout").sort(), Object.keys(LAYOUTS).sort());
  assert.deepEqual(rustEnum(look, "WidthPreset").sort(), Object.keys(WIDTHS).sort());
  for (const [w, { px }] of Object.entries(WIDTHS)) {
    const name = w[0].toUpperCase() + w.slice(1);
    if (px !== null) assert.ok(look.includes(`WidthPreset::${name} => Some(${px})`), `${w} = ${px} px v look.rs`);
  }
  // tvar proužku = geom::strip_metrics
  const geom = rust("appearance/geom.rs");
  for (const s of STRIP_STYLES) {
    const { h, r } = stripShape(s, "normal");
    const name = s[0].toUpperCase() + s.slice(1);
    const row = geom.match(new RegExp(`StripStyle::${name} => \\(([\\d.]+) \\* k, ([\\d.]+)(?: \\* k)?\\)`));
    assert.ok(row, `${s} v strip_metrics`);
    assert.deepEqual([+row[1], +row[2]], [h, r], s);
  }
});

test("plán otevření a zavření podle stylu animace", () => {
  const g = { collapsed: "C", band: "B", full: "F" };
  assert.equal(openPlan("none", 1, g), null);
  assert.equal(closePlan("none", 1, g), null);
  for (const s of ["spring", "smooth", "snappy"]) {
    const p = openPlan(s, 1, g);
    assert.deepEqual(p.frames.map((f) => f.clipPath), ["C", "F"], s);
    assert.equal(p.start.clipPath, "C");
    assert.ok(p.breath && p.duration > 0, s);
  }
  assert.ok(openPlan("smooth", 1, g).easing !== openPlan("spring", 1, g).easing, "jiná pružina");
  const notch = openPlan("notch", 1, g);
  assert.deepEqual(notch.frames.map((f) => f.clipPath), ["C", "B", "F"], "nejdřív do šířky, pak dolů");
  const bounce = openPlan("bounce", 1, g);
  assert.equal(bounce.start.transform, "translateY(-100%)");
  assert.equal(bounce.frames.at(-1).transform, "translateY(0%)");
  assert.ok(bounce.frames.filter((f) => f.transform === "translateY(0%)").length >= 3, "dopadne a dvakrát odskočí");
  const fade = openPlan("fade", 1, g);
  assert.deepEqual([fade.start.opacity, fade.frames[0].opacity, fade.frames.at(-1).opacity], ["0", 0, 1]);
  // rychlost dělí délku; z viditelného pohledu (peek) vždy morf tvaru
  assert.equal(openPlan("bounce", 2, g).duration, bounce.duration / 2);
  assert.deepEqual(openPlan("fade", 1, g, true).frames.map((f) => f.clipPath), ["C", "F"]);
  // zavření se vždy vejde do pojistky Rustu (600 ms), i při rychlosti 0,5
  for (const s of MOTION_STYLES) {
    for (const speed of [0.5, 1, 2]) {
      const p = closePlan(s, speed, g);
      if (p) assert.ok(p.duration + p.delay <= CLOSE_CAP_MS && CLOSE_CAP_MS < 600, `${s} ${speed}`);
    }
  }
  // pád: jen transform + opacity, jeden odraz nahoru a usazení; zavření ho vrací nahoru
  const drop = openPlan("drop", 1, g);
  assert.equal(drop.start.transform, "translateY(-60%)");
  assert.equal(drop.frames.at(-1).transform, "translateY(0%)");
  assert.ok(drop.frames.some((f) => f.transform === "translateY(-2.5%)"), "jeden odraz");
  assert.equal(drop.frames.filter((f) => f.transform === "translateY(0%)").length, 2, "dopad a usazení, ne dvojí odskok");
  assert.ok(drop.frames.every((f) => Object.keys(f).every((k) => ["transform", "opacity", "offset", "easing"].includes(k))), "žádný filtr ani clip");
  assert.equal(closePlan("drop", 1, g).frames.at(-1).opacity, 0);
  // rozvinutí: tvar shora dolů bez překmitu, karty po 20–30 ms
  const unfold = openPlan("unfold", 1, g);
  assert.deepEqual(unfold.frames.map((f) => f.clipPath), ["C", "B", "F"]);
  assert.ok(unfold.reveal.step >= 20 && unfold.reveal.step <= 30 && unfold.reveal.blur === false, "odstup karet");
  assert.equal(unfold.breath, false);
  assert.equal(openPlan("notch", 1, g).reveal, undefined, "ostatní styly mají výchozí naskakování");
  // tekutá kapsle (kolo 18): ze středu horní hrany, přestřel, jen transform + opacity; zavření zpět do kapsle
  const liquid = openPlan("liquid", 1, g);
  assert.equal(liquid.start.transform, "scale(0.42, 0.28)");
  assert.equal(liquid.frames.at(-1).transform, "scale(1, 1)");
  assert.ok(liquid.frames.some((f) => f.transform.includes("1.04")), "přestřel");
  assert.ok(liquid.frames.every((f) => Object.keys(f).every((k) => ["transform", "transformOrigin", "opacity", "offset", "easing"].includes(k))), "žádný filtr ani clip");
  assert.equal(closePlan("liquid", 1, g).frames.at(-1).transform, "scale(0.42, 0.28)");
  // dnešní stažení beze změny při rychlosti 1
  const c = closePlan("spring", 1, g);
  assert.deepEqual([c.duration, c.delay, c.frames.map((f) => f.clipPath)], [240, 40, ["F", "C"]]);
});

test("výkon (software render WebView2): bloky stylů ve styles.css bez filtru, rozmazání a animace", () => {
  const blocks = css.slice(css.indexOf("/* ---------- Styly (vlna 6)"), css.indexOf("/* CSS animace a přechody pryč")).replace(/\/\*[\s\S]*?\*\//g, "");
  assert.ok(blocks.includes('html[data-style="retro"]'), "našel jsem bloky stylů");
  for (const bad of ["backdrop-filter", "filter:", "animation", "@keyframes", "infinite"]) assert.ok(!blocks.includes(bad), `v blocích stylů je ${bad}`);
  // přechod barev při přepnutí záložky: jen barvy, nikdy transform / filtr, a jen po dobu atributu
  const shift = css.match(/html\[data-look-shift\][^{]*\{[^}]*\}/)[0];
  assert.ok(/background-color[^;]*border-color[^;]*color/.test(shift) && !/transform|filter|box-shadow/.test(shift), shift);
});
