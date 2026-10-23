// Tabulkové testy částí (sekcí) uvnitř karet bez DOM (`parts.ts`) + hlídač zapojení (čte zdroják — chování
// v DOM unit test neověří, tohle hlídá, že zapojení nezmizí).
//   node --experimental-strip-types --test src/layout/parts.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { foldKey, partKeyAction, toggleFold } from "./parts.ts";

const src = (p) => readFileSync(new URL(p, import.meta.url), "utf8");

test("klávesy na úchytu části: šipky přesun, Delete skrýt, s Ctrl nic", () => {
  const cases = [
    ["šipka vlevo", { key: "ArrowLeft" }, { move: -1 }],
    ["šipka nahoru", { key: "ArrowUp" }, { move: -1 }],
    ["šipka vpravo", { key: "ArrowRight" }, { move: 1 }],
    ["šipka dolů", { key: "ArrowDown" }, { move: 1 }],
    ["Alt + šipka jako šipka", { key: "ArrowDown", altKey: true }, { move: 1 }],
    ["Delete", { key: "Delete" }, { hide: true }],
    ["Backspace", { key: "Backspace" }, { hide: true }],
    ["Ctrl + šipka (patří prohlížeči / textu)", { key: "ArrowDown", ctrlKey: true }, null],
    ["Meta + Delete", { key: "Delete", metaKey: true }, null],
    ["písmeno", { key: "a" }, null],
    ["Enter", { key: "Enter" }, null],
  ];
  for (const [name, e, want] of cases) assert.deepEqual(partKeyAction(e), want, name);
});

test("sbalení části: přepnutí je čisté, klíč drží kartu i část", () => {
  const k = foldKey("claude", "claudeLimits");
  assert.equal(k, "claude:claudeLimits");
  assert.notEqual(foldKey("media", "sound"), foldKey("clock", "sound"));
  const a = toggleFold({}, k);
  assert.deepEqual(a, { [k]: true });
  assert.deepEqual(toggleFold(a, k), {}, "podruhé = rozbaleno (záznam pryč)");
  const orig = { x: true };
  const b = toggleFold(orig, k);
  assert.deepEqual(orig, { x: true }, "vstup netknutý");
  assert.deepEqual(b, { x: true, [k]: true });
});

test("kolo 13: části karet — sdílený motor tahu, pořadí v kartách, skrytí na zónu, sbalení nadpisem", () => {
  const drag = src("./partDrag.ts");
  const slot = src("./PartSlot.tsx");
  const deck = src("./Deck.tsx");
  const css = src("./dnd.css");
  // stejný motor jako karty a záložky: ne vlastní kopie FLIPu ani ducha
  assert.ok(drag.includes("flipEls(") && drag.includes("landGhost(") && drag.includes("gridDropIndex(") && drag.includes("dropAction("), "FLIP, přistání, cíl i dopad ze sdílených modulů");
  assert.ok(!/draggable|onDragStart|ondragstart|setPointerCapture/.test(drag + slot), "žádné HTML5 drag&drop");
  assert.ok(drag.includes('window.addEventListener("pointermove", move)') && drag.includes('finish("escape")') && drag.includes("cancelAnimationFrame(raf)"), "pointer na window, Esc zruší, rAF se uklízí");
  assert.equal((drag.match(/requestAnimationFrame\(/g) ?? []).length, 2, "rAF jen první snímek a smyčka v tahu");
  assert.ok(!/setInterval/.test(drag + slot + deck), "žádné časovače v klidu");
  // pořadí: CSS order (DOM se nepřeskládá), uložené pořadí z modelu
  assert.ok(slot.includes("order: String(ctl.order(p.card).indexOf(p.k))"), "pořadí části přes CSS order");
  assert.ok(deck.includes("partsOf(cardOf(card), card)") && deck.includes("movePart(l, page"), "pořadí z layout…parts, změna přes movePart");
  // skrytí: zóna Skrýt a hiddenParts; hlavní obsah (body) se skrýt nedá
  assert.ok(deck.includes("pd.hideable()") && deck.includes("setPartHidden(l, k, true)") && drag.includes("key === BODY ? undefined"), "část na zónu Skrýt → hiddenParts, body ne");
  assert.ok(slot.includes("<Show when={!p.hidden}>"), "skrytá část nic nekreslí ani nespouští");
  // karty části opravdu používají
  const cards = { claude: src("../ClaudeSessions.tsx"), clock: src("../cards/ClockCard.tsx"), media: src("../media/MediaCard.tsx") };
  for (const [card, code] of Object.entries(cards)) assert.ok(code.includes(`<PartsBox card="${card}">`) && code.includes("<PartSlot"), `${card}: části v obalu`);
  for (const k of ["claudeLimits", "claudeCosts", "nowPlaying", "sound", "battery"]) assert.ok(Object.values(cards).some((c) => c.includes(`k="${k}"`)), `část ${k} je slot`);
  // výchozí vzhled beze změny: obaly bez boxu, animace jen na klik a ne při omezení pohybu
  assert.ok(/\.lp-box, \.lp-slot \{ display: contents; \}/.test(css), "obaly ve výchozím stavu nemají box");
  assert.ok(css.includes("grid-template-rows") && css.includes("prefers-reduced-motion") && !/backdrop-filter|filter:\s*blur/.test(css), "sbalení přes grid-template-rows, bez blur");
  assert.ok(slot.includes("aria-expanded"), "sbalení nadpisem je tlačítko s aria-expanded");
});
