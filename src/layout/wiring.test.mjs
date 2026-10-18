// Hlídač zapojení rozložení (čte zdroják — chování v DOM unit test neověří, tohle hlídá, že oprava nezmizí).
//   node --experimental-strip-types --test src/layout/wiring.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const src = (p) => readFileSync(new URL(p, import.meta.url), "utf8");

test("případ 8: ⋯ na kartě v režimu úprav mění draft, ne config (jinak by Hotovo změnu přepsalo)", () => {
  const menu = src("../cards/CardMenu.tsx");
  assert.ok(menu.includes("if (editing())") && menu.includes("change((l) => f(l).layout)"), "tweak: v úpravách do draftu");
  for (const f of ["setSize", "setHeight", "setLimit"]) assert.ok(!new RegExp(`saveLayout[(][^)]*${f}`).test(menu), `${f} jde přes tweak, ne saveLayout`);
  assert.ok(menu.includes("placePopup"), "pozice seznamu ze sdíleného ui/place.ts");
});

test("případ 6 + 9: tah karty je pointer (ne HTML5), náhled bez přeskládání DOM, Esc zruší jen tah, velikost živě", () => {
  const deck = src("./Deck.tsx");
  assert.ok(!/draggable|onDragStart|ondragstart/.test(deck), "žádné HTML5 drag&drop (ve WebView2 nefunguje)");
  assert.ok(deck.includes('window.addEventListener("pointermove", move)'), "posluchače na window — přežijí posun karty");
  assert.ok(deck.includes("order: String(view().indexOf(id))"), "náhled pořadí přes CSS order");
  assert.ok(deck.includes('finish("escape")') && deck.includes("ev.stopPropagation()"), "Esc během tahu nezavře úpravy");
  assert.ok(deck.includes("autoScrollStep"), "posun u okraje");
  assert.ok(deck.includes("sizeNow(id)") && deck.includes("hNow(id)"), "tah za okraj mění kartu živě");
  assert.ok(!/@keyframes\s+(wiggle|jiggle|kyv)/i.test(src("./layout.css")), "bez kývání karet");
});

test("případ 10: lišta úprav má Zpět a Obnovit výchozí s potvrzením", () => {
  const bar = src("./EditBar.tsx");
  assert.ok(bar.includes("undoLast") && bar.includes("canUndo"), "Zpět");
  assert.ok(bar.includes("resetToDefault") && bar.includes("cards.layout.resetAsk"), "Obnovit výchozí se ptá");
});
