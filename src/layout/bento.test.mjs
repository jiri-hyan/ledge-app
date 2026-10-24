// @vitest-environment node
// Bento (`bento.ts`) a předvolby vzhledu karet (`look.ts` `LOOK_PRESETS`): dlaždice = šířka × výška, šablony jako
// NOVÁ záložka (6–9 dlaždic, největší první, jen existující karty), starý tvar rozložení beze změny.
//   node --experimental-strip-types --test src/layout/bento.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { addTemplatePage, setBento, setTile, templateCards, templateOf, TEMPLATES, TILE_DIM, TILES, tileOf } from "./bento.ts";
import { applyLookPreset, cardLookAttrs, heroable, LOOK_PRESETS, normalizeCardLook, patchCardLook, presetOf } from "./look.ts";
import { CARD_IDS, defaultLayout, MAX_PAGES, normalizeLayout } from "./model.ts";
import { fullLayout } from "./fullLayout.mjs";

const area = (t) => {
  const d = TILE_DIM[t];
  return { s: 1, m: 2, l: 3, xl: 4 }[d.size] * d.h;
};

test("dlaždice: 1×1 / 2×1 / 2×2 = velikost × výška; v bento chybějící výška = 2 řádky", () => {
  assert.deepEqual(TILES, ["1x1", "2x1", "2x2"]);
  const cases = [
    [{ size: "s", h: 1 }, "1x1"],
    [{ size: "m", h: 1 }, "2x1"],
    [{ size: "m", h: 2 }, "2x2"],
    [{ size: "m" }, "2x2"],
    [{ size: "s" }, null],
    [{ size: "l", h: 1 }, null],
    [undefined, null],
  ];
  for (const [c, want] of cases) assert.equal(tileOf(c), want, JSON.stringify(c));
  const l = setTile(fullLayout(), "prace", "claude", "2x2");
  const c = l.pages.find((p) => p.id === "prace").cards.find((x) => x.id === "claude");
  assert.deepEqual([c.size, c.h], ["m", 2]);
});

test("bento na záložce: zapnout / vypnout, normalizace drží jen true", () => {
  const on = setBento(defaultLayout(), "prace", true);
  assert.equal(on.pages[0].bento, true);
  assert.equal(on.pages[1].bento, undefined, "jiná záložka beze změny");
  assert.equal("bento" in setBento(on, "prace", false).pages[0], false);
  assert.equal(normalizeLayout(on).pages[0].bento, true);
  assert.equal("bento" in normalizeLayout({ pages: [{ id: "x", name: "X", cards: [], bento: "ano" }] }).pages[0], false);
});

test("šablony: 6–9 dlaždic, existující karty bez opakování, největší první", () => {
  assert.deepEqual(TEMPLATES.map((t) => t.id), ["work", "minimal", "music"]);
  for (const t of TEMPLATES) {
    assert.ok(t.cards.length >= 6 && t.cards.length <= 9, `${t.id}: ${t.cards.length}`);
    assert.equal(new Set(t.cards.map((c) => c.id)).size, t.cards.length, `${t.id}: bez opakování`);
    for (const c of t.cards) assert.ok(CARD_IDS.includes(c.id), `${t.id}: ${c.id}`);
    const areas = t.cards.map((c) => area(c.tile));
    assert.equal(areas[0], Math.max(...areas), `${t.id}: první = největší`);
    assert.deepEqual([...areas].sort((a, b) => b - a), areas, `${t.id}: velikosti sestupně`);
  }
  // hrdina jen u karet, které ho umí
  for (const t of TEMPLATES) for (const c of templateCards(t)) if (c.look?.hero) assert.ok(heroable(c.id), c.id);
  assert.equal(templateOf("minimal").cards[0].id, "bigclock");
  assert.equal(templateOf("nic"), undefined);
});

test("šablona = nová záložka na konci; ostatní záložky beze změny; plno = null", () => {
  const base = defaultLayout();
  const r = addTemplatePage(base, templateOf("music"), "Hudba");
  assert.equal(r.layout.pages.length, base.pages.length + 1);
  assert.deepEqual(r.layout.pages.slice(0, -1), base.pages);
  const p = r.layout.pages.at(-1);
  assert.deepEqual([p.id, p.name, p.icon, p.bento], [r.id, "Hudba", "🎵", true]);
  assert.deepEqual(p.cards[0], { id: "media", size: "m", h: 2, look: { surface: "glass", grain: true } });
  // projde normalizací beze ztráty (Rust validace: `h` 1–4, karta jednou)
  assert.deepEqual(normalizeLayout(r.layout).pages.at(-1), p);
  let full = base;
  while (full.pages.length < MAX_PAGES) full = addTemplatePage(full, templateOf("work"), `P${full.pages.length}`).layout;
  assert.equal(addTemplatePage(full, templateOf("work"), "Další"), null);
});

test("plocha, zrnění, hrdina: normalizace, atributy, hrdina jen u karet s číslem", () => {
  assert.deepEqual(normalizeCardLook({ surface: "glass", grain: true, hero: true }, "claude"), { surface: "glass", grain: true, hero: true });
  assert.deepEqual(normalizeCardLook({ surface: "layer", hero: true }, "notes"), { surface: "layer" }, "poznámky hrdinu nemají");
  assert.equal(normalizeCardLook({ surface: "kov", grain: "ano" }, "claude"), undefined);
  assert.deepEqual(cardLookAttrs({ surface: "glass", grain: true, hero: true }).attrs, { "data-cl-surface": "glass", "data-cl-grain": "", "data-cl-hero": "" });
  assert.deepEqual(cardLookAttrs(undefined), { attrs: {}, style: {} }, "bez vzhledu žádný atribut");
  assert.equal(patchCardLook({ surface: "glass", grain: true }, "claude", { surface: "style", grain: false }), undefined);
});

test("předvolby galerie: na všechny karty záložky, ostatní pole vzhledu zůstanou, poznání zvolené", () => {
  assert.ok(LOOK_PRESETS.length > 4, "víc než 4 volby = galerie dlaždic");
  const base = fullLayout();
  const pid = base.pages[0].id;
  const withAccent = { ...base, pages: base.pages.map((p, i) => (i ? p : { ...p, cards: p.cards.map((c, j) => (j ? c : { ...c, look: { accent: "teal" } })) })) };
  const hero = applyLookPreset(withAccent, pid, "hero");
  const cards = hero.pages[0].cards;
  assert.equal(cards[0].look.accent, "teal", "akcent zůstal");
  assert.ok(cards.every((c) => c.look?.surface === "layer"));
  assert.ok(cards.every((c) => !!c.look?.hero === heroable(c.id)));
  assert.equal(presetOf(hero, pid), "hero");
  assert.deepEqual(hero.pages.slice(1), withAccent.pages.slice(1), "jiné záložky beze změny");
  const back = applyLookPreset(hero, pid, "style");
  assert.equal(presetOf(back, pid), "style");
  assert.deepEqual(back.pages[0].cards[0].look, { accent: "teal" });
  assert.ok(back.pages[0].cards.slice(1).every((c) => c.look === undefined), "Podle stylu = pole pryč");
  assert.equal(applyLookPreset(base, pid, "neexistuje"), base);
});

test("CSS: bento a plochy jen s atributem (výchozí beze změny), bez animace a filtru", () => {
  const lay = readFileSync(new URL("./layout.css", import.meta.url), "utf8");
  const css = lay.slice(lay.indexOf("/* ---------- plocha karty 2026"), lay.indexOf("/* ---------- režim úprav"));
  assert.ok(css.length > 200, "blok plochy karty");
  for (const rule of css.split("}").map((r) => r.trim()).filter((r) => r.startsWith(".deck"))) assert.match(rule.split("{")[0], /data-cl-/, rule.split("{")[0]);
  const code = css.replace(/\/\*[\s\S]*?\*\//g, "");
  assert.doesNotMatch(code, /@keyframes|animation\s*:|backdrop-filter\s*:|https:\/\//);
  for (const rule of lay.split("}").filter((r) => r.includes("data-bento"))) assert.match(rule, /\[data-bento\]/);
  assert.match(lay, /\.deck\[data-bento\][^{]*\{[^}]*grid-auto-flow: row dense/);
});
