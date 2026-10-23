// @vitest-environment node
// Vzhled karty (`look.ts`): karta bez vzhledu = žádný atribut (přesně jako dřív), normalizace, varianty
// jen u karet, které je mají, změna v rozložení, přehled a Obnovit vše, zrcadlo akcentů s Rustem a
// průchod `normalizeLayout` (starý tvar beze změny, nový se neztratí).
//   node --experimental-strip-types --test src/layout/look.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  accentCss, BGS, CARD_ACCENTS, cardLookAttrs, customizedCards, DENSITIES, normalizeCardLook, patchCardLook, resetCardLooks, setCardLook, VIEW_CARDS, VIEWS, viewOf, viewsOf,
} from "./look.ts";
import { clone, normalizeLayout, pageOf, sameLayout } from "./model.ts";
import { fullLayout } from "./fullLayout.mjs";

test("karta bez vzhledu = žádné atributy ani proměnné", () => {
  for (const raw of [undefined, null, {}, { bg: "style" }, { accent: "pink" }, { view: "list" }, { hideHead: false }]) {
    const l = normalizeCardLook(raw, "repos");
    assert.equal(l, undefined, JSON.stringify(raw));
    assert.deepEqual(cardLookAttrs(l), { attrs: {}, style: {} });
  }
});

test("normalizace a atributy", () => {
  assert.deepEqual(normalizeCardLook({ accent: "teal", bg: "tint", hideHead: true, density: "airy", view: "compact", junk: 1 }, "repos"), { accent: "teal", bg: "tint", hideHead: true, density: "airy", view: "compact" });
  assert.equal(normalizeCardLook({ accent: "TEAL" }, "repos"), undefined, "token jen malými");
  assert.deepEqual(normalizeCardLook({ view: "list" }, "repos"), undefined, "výchozí varianta = pole pryč");
  assert.deepEqual(normalizeCardLook({ view: "list" }, "clock"), undefined, "karta bez variant");
  assert.deepEqual(normalizeCardLook({ view: "list" }, "projects"), { view: "list" });
  assert.deepEqual(normalizeCardLook({ accent: "#AABBCC" }, "clock"), { accent: "#aabbcc" });
  const a = cardLookAttrs({ accent: "teal", bg: "clear", hideHead: true, density: "compact" });
  assert.deepEqual(a.attrs, { "data-cl-accent": "", "data-cl-bg": "clear", "data-cl-head": "off", "data-cl-dens": "compact" });
  assert.deepEqual(a.style, { "--accent": "var(--teal)" });
  assert.equal(accentCss("accent"), "var(--accent)");
  assert.equal(accentCss("#123456"), "#123456");
  assert.ok(BGS.length <= 4 && DENSITIES.length <= 4 && VIEWS.length <= 4, "segment nejvýš 4 volby");
});

test("varianty zobrazení: Projekty, Repa, Poznámky, Akce; výchozí = dnešní", () => {
  assert.deepEqual(Object.keys(VIEW_CARDS).sort(), ["actions", "notes", "projects", "repos"]);
  assert.equal(viewOf("projects", undefined), "tiles");
  assert.equal(viewOf("repos", undefined), "list");
  assert.equal(viewOf("repos", { view: "tiles" }), "tiles");
  assert.equal(viewOf("clock", { view: "tiles" }), undefined);
});

test("patch a změna v rozložení; Obnovit podle stylu; přehled a Obnovit vše", () => {
  assert.deepEqual(patchCardLook(undefined, "notes", { bg: "solid", density: "style" }), { bg: "solid" });
  assert.equal(patchCardLook({ bg: "solid" }, "notes", { bg: "style" }), undefined);
  const d = fullLayout();
  const l = setCardLook(setCardLook(d, "prace", "claude", { accent: "red" }), "osobni", "clock", { bg: "clear" });
  assert.deepEqual(pageOf(l, "prace").cards.find((c) => c.id === "claude").look, { accent: "red" });
  assert.equal(pageOf(d, "prace").cards.find((c) => c.id === "claude").look, undefined, "vstup netknutý");
  assert.deepEqual(customizedCards(l), [{ pageId: "prace", cardId: "claude" }, { pageId: "osobni", cardId: "clock" }]);
  const back = setCardLook(l, "prace", "claude", undefined);
  assert.ok(!("look" in pageOf(back, "prace").cards.find((c) => c.id === "claude")));
  const reset = resetCardLooks(l);
  assert.deepEqual(customizedCards(reset), []);
  assert.ok(sameLayout(reset, d), "po Obnovit vše jako výchozí");
});

test("normalizeLayout: vzhled se drží, klon je hluboký, starý tvar beze změny", () => {
  const raw = { pages: [{ id: "prace", name: "", cards: [{ id: "repos", size: "m", look: { bg: "tint", view: "tiles", nesmysl: 1 } }, { id: "claude", size: "m" }] }], hiddenParts: [] };
  const n = normalizeLayout(raw);
  assert.deepEqual(n.pages[0].cards[0], { id: "repos", size: "m", look: { bg: "tint", view: "tiles" } });
  assert.deepEqual(n.pages[0].cards[1], { id: "claude", size: "m" }, "karta bez vzhledu nedostane pole");
  const c = clone(n);
  c.pages[0].cards[0].look.bg = "solid";
  assert.equal(n.pages[0].cards[0].look.bg, "tint");
  // uložit → načíst (JSON) → stejné
  assert.deepEqual(normalizeLayout(JSON.parse(JSON.stringify(n))), n);
});

test("zrcadlo Rustu: akcenty karty jako tilelook.rs", () => {
  const src = readFileSync(new URL("../../src-tauri/src/tilelook.rs", import.meta.url), "utf8");
  const block = src.slice(src.indexOf("pub const CARD_ACCENTS"), src.indexOf("];", src.indexOf("pub const CARD_ACCENTS")));
  assert.deepEqual(block.match(/"([a-z]+)"/g).map((s) => s.slice(1, -1)), [...CARD_ACCENTS]);
});

test("Bento jen u karet, které ho kreslí", () => {
  assert.deepEqual(viewsOf("notes"), ["list", "tiles", "bento", "compact"]);
  assert.deepEqual(viewsOf("actions"), ["list", "tiles", "compact"]);
  assert.deepEqual(viewsOf("clock"), []);
  assert.deepEqual(normalizeCardLook({ view: "bento" }, "repos"), { view: "bento" });
  assert.equal(normalizeCardLook({ view: "bento" }, "actions"), undefined, "akce bento nekreslí");
});
