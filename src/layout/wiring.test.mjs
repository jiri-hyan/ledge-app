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
  const drag = src("./cardDrag.ts");
  assert.ok(!/draggable|onDragStart|ondragstart/.test(deck + drag), "žádné HTML5 drag&drop (ve WebView2 nefunguje)");
  assert.ok(drag.includes('window.addEventListener("pointermove", move)'), "posluchače na window — přežijí posun karty");
  assert.ok(deck.includes("order: String(view().indexOf(id))"), "náhled pořadí přes CSS order");
  assert.ok(drag.includes('finish("escape")') && drag.includes("ev.stopPropagation()"), "Esc během tahu nezavře úpravy");
  assert.ok(drag.includes("autoScrollStep"), "posun u okraje");
  assert.ok(deck.includes("sizeNow(id)") && deck.includes("hNow(id)"), "tah za okraj mění kartu živě");
  assert.ok(!/@keyframes\s+(wiggle|jiggle|kyv)/i.test(src("./layout.css")), "bez kývání karet");
});

test("kolo 10: přirozený drag & drop — podržení hlavičky, duch, zóna Skrýt, přepnutí záložky, rAF jen během tahu", () => {
  const deck = src("./Deck.tsx");
  const drag = src("./cardDrag.ts");
  const css = src("./layout.css");
  const dnd = src("./dnd.ts");
  // celá karta je úchyt a podržení hlavičky zapíná úpravy
  assert.ok(deck.includes("onPointerDown={(e) => cd.down(e, id)}"), "pointerdown na celé položce karty");
  assert.ok(deck.includes("begin={") === false && deck.includes("begin: () => p.begin?.()"), "podržení mimo úpravy zapne úpravy přes prop begin");
  assert.ok(src("../MainPage.tsx").includes("begin={() => edit()}"), "MainPage dává mřížce zapnutí úprav");
  assert.ok(drag.includes("holdStep(") && drag.includes("canHold(") && drag.includes("HOLD_MS"), "podržení přes čisté funkce");
  assert.ok(drag.includes("EDIT_CONTROL_SELECTOR") && drag.includes('t.closest(".lay-grip")'), "ovládací prvky karty tah nezačnou, ⠿ ano");
  // duch bez ovládání, vlastní třída, mimo měření karet
  assert.ok(drag.includes('"lay-fly"') && deck.includes(":not(.lay-fly)") && drag.includes(":not(.lay-fly)"), "duch se nepočítá mezi karty");
  assert.ok(/\.lay-fly[^}]*pointer-events: none/.test(css), "duch nezachytává ukazatel");
  // rAF jen v tahu a uklizená
  assert.equal((drag.match(/requestAnimationFrame\(/g) ?? []).length, 2, "rAF: první snímek a smyčka (obojí v session)");
  assert.ok(drag.includes("cancelAnimationFrame(raf)"), "smyčka se při konci tahu zruší");
  assert.ok(!/setInterval/.test(drag + deck), "žádné časovače v klidu");
  // zóna Skrýt a prodleva nad ouškem
  assert.ok(deck.includes('class="lay-zone"') && drag.includes("inBox(z,"), "zóna Skrýt kartu");
  assert.ok(drag.includes("dwellStep(") && drag.includes("setEditPage(tabId)"), "prodleva nad ouškem přepne záložku");
  assert.ok(css.includes(`${dnd.match(/DWELL_MS = (\d+)/)[1]}ms linear`), "proužek prodlevy v CSS trvá stejně jako DWELL_MS");
  // FLIP respektuje nastavení pohybu Ledge a omezení pohybu
  assert.ok(deck.includes("flipPlan(m.style, m.speed, reducedMotion())"), "FLIP podle motionNow + reduced-motion");
  // po dopadu lišta s Vrátit; Zpět drží history
  assert.ok(deck.includes("cards.layout.dragMoved") && deck.includes("offerUndo("), "po dopadu lišta „… · Vrátit“");
});

test("případ 10: lišta úprav má Zpět a Obnovit výchozí s potvrzením", () => {
  const bar = src("./EditBar.tsx");
  assert.ok(bar.includes("undoLast") && bar.includes("canUndo"), "Zpět");
  assert.ok(bar.includes("resetToDefault") && bar.includes("cards.layout.resetAsk"), "Obnovit výchozí se ptá");
});
