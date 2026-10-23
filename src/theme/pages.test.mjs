// Vzhled záložek a vlastní motivy (`pages.ts`, `customThemes.ts`, `contrast.ts`): normalizace configu,
// přepis vzhledu na záložce, kontrast textu (stejná tabulka jako Rust `contrast_table_matches_frontend`),
// export / import a zrcadlo s `appearance/pages.rs`. Spouští `npm run test:ui`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { THEMES, THEME_IDS } from "./themes.ts";
import { contrast, parseColor } from "./contrast.ts";
import { DEFAULT_APPEARANCE, appearanceVars, normalizeAppearance } from "./appearance.ts";
import { STYLES } from "./looks.ts";
import { withStyle } from "./styles.ts";
import {
  CUSTOM_KEYS, CUSTOM_TEXT_KEYS, LIGHT_LUM, MAX_CUSTOM_THEMES, addCustomTheme, colorsOf, customIdOk, customProblems, exportTheme, importTheme,
  isLightBg, newThemeId, normalizeCustomThemes, resolveCustom, themeOf, updateCustomTheme,
} from "./customThemes.ts";
import { dropThemeRefs, effectiveAppearance, normalizePages, setPageLook } from "./pages.ts";
import { textProblems } from "./contrast.ts";

const rust = readFileSync(new URL("../../src-tauri/src/appearance/pages.rs", import.meta.url), "utf8");
const mine = (id = "my-x", base = "dark", patch = {}) => ({ id, name: "Můj", base, colors: { ...colorsOf(base), ...patch } });

test("kontrast: stejná tabulka jako Rust (hranice 4.5:1)", () => {
  for (const [fg, bg, ok] of [
    ["#f4f5f9", "#101018", true],
    ["#111111", "#ffffff", true],
    ["#808080", "#707070", false],
    ["#767676", "#ffffff", true],
    ["#777777", "#ffffff", false],
  ]) assert.equal(contrast(parseColor(fg), parseColor(bg)) >= 4.5, ok, `${fg} na ${bg}`);
});

test("vlastní motiv ze vestavěného: úplná sada tokenů a čitelný text", () => {
  for (const id of THEME_IDS) {
    const c = mine("my-" + id, id);
    const { vars, light } = resolveCustom(c);
    assert.deepEqual(Object.keys(vars).sort(), Object.keys(THEMES[id].vars).sort(), id);
    assert.equal(light, THEMES[id].light, `${id}: tón sedí`);
    assert.deepEqual(textProblems(vars), [], `${id}: i odvozené barvy čitelné`);
  }
});

test("vlastní motiv: opačný tón pozadí vezme vrstvy z dark / light a zůstane čitelný", () => {
  const dark = mine("my-a", "dark", { bg: "#fbf7ee", raised: "#ffffff", t1: "#1a1a1a", t2: "#3a3a3a", t3: "#505050", green: "#14532d", amber: "#7c4a03", red: "#991b1b", blue: "#1e40af" });
  const r = resolveCustom(dark);
  assert.equal(r.light, true);
  assert.equal(r.vars.card, THEMES.light.vars.card, "světlé vrstvy místo tmavých");
  assert.deepEqual(customProblems(dark), []);
  assert.deepEqual(textProblems(r.vars), []);
});

test("vlastní motiv: nízký kontrast se pozná u klíče a zablokuje", () => {
  const bad = mine("my-b", "dark", { t3: "#202030" });
  assert.deepEqual(customProblems(bad).map((p) => p.key), ["t3"]);
  assert.ok(customProblems(bad)[0].ratio < 4.5);
  assert.deepEqual(customProblems(mine("my-c", "dark", { t3: "#9a9fb0" })), []);
  // text splývá s pozadím i když je pozadí jiné než barvy karet
  assert.ok(customProblems(mine("my-d", "dark", { bg: "#808080" })).length > 0);
});

test("tón motivu podle svítivosti pozadí (zrcadlo Rust `light`)", () => {
  assert.equal(LIGHT_LUM, 0.18);
  for (const [bg, light] of [["#101018", false], ["#262a37", false], ["#707070", false], ["#7a7a7a", true], ["#f8f9fc", true], ["#ffffff", true]]) assert.equal(isLightBg(bg), light, bg);
});

test("normalizace: neplatné záznamy pryč, vadné barvy z základu, max 20", () => {
  const good = { id: "my-one", name: " Moje ", base: "nord", colors: { ...colorsOf("nord"), accent: "#22C55E", t2: "nope" } };
  const out = normalizeCustomThemes([good, good, { id: "nord", colors: {} }, { id: "Velké" }, null, "x", { id: "ok2", base: "zzz" }]);
  assert.deepEqual(out.map((c) => c.id), ["my-one", "ok2"]);
  assert.equal(out[0].name, "Moje");
  assert.equal(out[0].colors.accent, "#22c55e");
  assert.equal(out[0].colors.t2, colorsOf("nord").t2, "vadná barva z motivu base");
  assert.equal(out[1].base, "dark");
  assert.equal(out[1].name, "ok2");
  const many = Array.from({ length: 30 }, (_, i) => ({ id: "m" + i, colors: {} }));
  assert.equal(normalizeCustomThemes(many).length, MAX_CUSTOM_THEMES);
  assert.deepEqual(normalizeCustomThemes(undefined), []);
});

test("jména motivů: tvar, volné id, přidání, úprava", () => {
  for (const [id, ok] of [["my-a", true], ["a", true], ["", false], ["-a", false], ["A", false], ["a_b", false], ["x".repeat(24), true], ["x".repeat(25), false]]) assert.equal(customIdOk(id), ok, id);
  assert.equal(newThemeId([], "Večerní káva"), "my-vecerni-kava");
  assert.equal(newThemeId(["my-vecerni-kava"], "Večerní káva"), "my-vecerni-kava-2");
  assert.equal(newThemeId([], "!!!"), "my-motiv");
  assert.ok(customIdOk(newThemeId([], "x".repeat(60))));
  const a = addCustomTheme([], "  Ráno ", "light", colorsOf("light"));
  assert.equal(a.list[0].name, "Ráno");
  assert.equal(a.id, "my-rano");
  let list = updateCustomTheme(a.list, a.id, { name: "Večer", colors: { accent: "#FF0000", bg: "špatně", t9: "#000000" } });
  assert.equal(list[0].name, "Večer");
  assert.equal(list[0].colors.accent, "#ff0000");
  assert.equal(list[0].colors.bg, colorsOf("light").bg, "vadná barva se ignoruje");
  assert.deepEqual(Object.keys(list[0].colors).sort(), [...CUSTOM_KEYS].sort(), "cizí klíč se nepřidá");
  assert.equal(addCustomTheme(Array.from({ length: 20 }, (_, i) => mine("my-" + i)), "x", "dark", colorsOf("dark")), null);
});

test("export a import motivu (schránka)", () => {
  const c = mine("my-e", "midnight");
  const back = importTheme(exportTheme(c), []);
  assert.ok(back.ok);
  assert.deepEqual([back.theme.name, back.theme.base, back.theme.colors], [c.name, c.base, c.colors]);
  assert.equal(back.theme.id, "my-muj", "id se přidělí nové z názvu");
  assert.equal(importTheme("{", []).error, "json");
  assert.equal(importTheme("[1]", []).error, "format");
  assert.equal(importTheme(JSON.stringify({ ledgeTheme: 1, colors: { bg: "#000000" } }), []).error, "format");
  const lowContrast = exportTheme(mine("my-f", "dark", { t1: "#101018" }));
  const r = importTheme(lowContrast, []);
  assert.equal(r.error, "contrast");
  assert.ok(r.keys.includes("t1"));
  assert.equal(importTheme(exportTheme(c), Array.from({ length: 20 }, (_, i) => mine("my-" + i))).error, "full");
});

test("vzhled s vlastním motivem: tokeny, tón a výběr", () => {
  const c = mine("my-g", "dark", { accent: "#22c55e" });
  const a = normalizeAppearance({ customThemes: [c], customTheme: "my-g" });
  assert.equal(a.customTheme, "my-g");
  assert.equal(themeOf(a).vars.accent, "#22c55e");
  assert.equal(appearanceVars(a).accent, "#22c55e");
  assert.equal(appearanceVars({ ...a, accent: "#ff0000" }).accent, "#ff0000", "vlastní akcent vzhledu je výš");
  assert.equal(normalizeAppearance({ customThemes: [c], customTheme: "neni" }).customTheme, "", "neznámý výběr = vestavěný");
  assert.equal(themeOf(DEFAULT_APPEARANCE).vars, THEMES.dark.vars);
});

test("záložky: normalizace, přepis vzhledu a vrácení na celek", () => {
  const c = mine("my-h", "nord");
  const pages = normalizePages(
    { prace: { style: "terminal", theme: "my-h", accent: "#22C55E" }, osobni: { theme: "neexistuje", style: "Nic", accent: "red" }, x: { theme: "nord" }, "": { style: "glass" }, y: 5 },
    [c],
  );
  assert.deepEqual(pages, { prace: { style: "terminal", theme: "my-h", accent: "#22c55e" }, x: { theme: "nord" } });
  const base = normalizeAppearance({ style: "neon", theme: "neon", accent: "#ff00aa", density: "compact", customThemes: [c], pages });
  assert.equal(effectiveAppearance(base, "osobni"), base, "bez přepisu totéž");
  assert.equal(effectiveAppearance(base, ""), base);
  const e = effectiveAppearance(base, "prace");
  assert.deepEqual([e.style, e.font, e.material, e.layout, e.density], ["terminal", "mono", "frame", "band", "compact"].map((v, i) => (i === 4 ? STYLES.find((s) => s.id === "terminal").parts.density : v)));
  assert.equal(e.customTheme, "my-h", "motiv záložky přebije motiv stylu");
  assert.equal(e.accent, "#22c55e");
  assert.equal(e.strip, base.strip, "proužek zůstává celkový");
  assert.equal(e.width, base.width);
  const onlyTheme = effectiveAppearance(base, "x");
  assert.deepEqual([onlyTheme.style, onlyTheme.theme, onlyTheme.accent, onlyTheme.density], ["neon", "nord", "#ff00aa", "compact"]);
  // styl bez motivu a akcentu = barvy stylu, akcent stylu je prázdný (jako withStyle)
  const s = effectiveAppearance(normalizeAppearance({ pages: { p: { style: "paper" } }, accent: "#00ff00" }), "p");
  assert.deepEqual([s.theme, s.accent, s.font], [withStyle({}, "paper").theme, "", "serif"]);
});

test("setPageLook a dropThemeRefs", () => {
  let a = normalizeAppearance({ customThemes: [mine("my-i")], customTheme: "my-i" });
  a = setPageLook(a, "p1", { style: "glass", theme: "my-i" });
  a = setPageLook(a, "p2", { accent: "#123456" });
  assert.deepEqual(a.pages, { p1: { style: "glass", theme: "my-i" }, p2: { accent: "#123456" } });
  a = setPageLook(a, "p2", { accent: undefined });
  assert.deepEqual(Object.keys(a.pages), ["p1"], "prázdný záznam zmizí");
  a = dropThemeRefs(a, "my-i");
  assert.deepEqual([a.customTheme, a.pages.p1], ["", { style: "glass" }]);
  assert.deepEqual(dropThemeRefs(setPageLook(a, "p1", { style: undefined }), "my-i").pages, {});
});

test("zrcadlo Rustu `appearance/pages.rs`: klíče barev a limity", () => {
  const arr = (name) => rust.match(new RegExp(`${name}: \\[&str; \\d+\\] = \\[([^\\]]+)\\]`))[1].match(/"([^"]+)"/g).map((s) => s.slice(1, -1));
  assert.deepEqual(arr("EDITABLE"), [...CUSTOM_KEYS]);
  assert.deepEqual(arr("TEXT_KEYS"), [...CUSTOM_TEXT_KEYS]);
  assert.equal(+rust.match(/MAX_CUSTOM_THEMES: usize = (\d+)/)[1], MAX_CUSTOM_THEMES);
  assert.equal(+rust.match(/const LIGHT_LUM: f32 = ([\d.]+)/)[1], 0.18);
  assert.equal(+rust.match(/MIN_CONTRAST: f32 = ([\d.]+)/)[1], 4.5);
});
