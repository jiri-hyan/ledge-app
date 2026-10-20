// Tabulkové testy tažení karet v režimu úprav (`drag.ts`): co se stane po puštění / zrušení, posun
// mřížky u okraje, klávesy na úchytu, náhled pořadí během tahu.
//   node --experimental-strip-types --test src/layout/drag.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { autoScrollStep, dropAction, keyAction, previewOrder } from "./drag.ts";

test("případ 5: pustit = přesun, zrušení (pointercancel, Esc) = nic", () => {
  // dřív: pointercancel (WebView2 ztratí capture) kartu přesunul, Esc během tahu zrušil celé úpravy
  const cases = [
    ["pustit na jiné místo", { from: 0, to: 2 }, null, "up", { kind: "move", from: 0, to: 2 }],
    ["pustit na své místo", { from: 1, to: 1 }, null, "up", { kind: "none" }],
    ["pustit na záložku", { from: 1, to: 1 }, "hry", "up", { kind: "page", tab: "hry" }],
    ["ztráta ukazatele", { from: 0, to: 2 }, null, "cancel", { kind: "none" }],
    ["Esc", { from: 0, to: 2 }, "hry", "escape", { kind: "none" }],
    ["bez tahu", null, null, "up", { kind: "none" }],
  ];
  for (const [name, d, tab, how, want] of cases) assert.deepEqual(dropAction(d, tab, how), want, name);
});

test("kolo 10: zóna Skrýt, mřížka jiné záložky, Esc všude nic", () => {
  const d = { from: 1, to: 3 };
  const there = { foreign: { page: "hry", blocked: false } };
  const dup = { foreign: { page: "hry", blocked: true } };
  const cases = [
    ["pustit na zónu Skrýt", d, null, "up", { zone: true }, { kind: "hide" }],
    ["zóna má přednost před záložkou", d, "hry", "up", { zone: true }, { kind: "hide" }],
    ["tah přepnutý na jinou záložku: vložit na určené místo", d, null, "up", there, { kind: "insert", page: "hry", index: 3 }],
    ["tam, kde karta už je: nic se nestěhuje, jen vysvětlení", d, null, "up", dup, { kind: "blocked", page: "hry" }],
    ["přímo na ouško má přednost před mřížkou", d, "osobni", "up", there, { kind: "page", tab: "osobni" }],
    ["Esc na zóně", d, null, "escape", { zone: true }, { kind: "none" }],
    ["ztráta ukazatele na jiné záložce", d, null, "cancel", there, { kind: "none" }],
    ["Esc na jiné záložce", d, "hry", "escape", there, { kind: "none" }],
    ["bez tahu na zóně", null, null, "up", { zone: true }, { kind: "none" }],
  ];
  for (const [name, st, tab, how, where, want] of cases) assert.deepEqual(dropAction(st, tab, how, where), want, name);
});

test("náhled pořadí: tažená karta je hned na místě dopadu, ostatní se posunou", () => {
  assert.deepEqual(previewOrder(["a", "b", "c", "d"], 0, 2), ["b", "c", "a", "d"]);
  assert.deepEqual(previewOrder(["a", "b", "c", "d"], 3, 0), ["d", "a", "b", "c"]);
  assert.deepEqual(previewOrder(["a", "b"], 1, 1), ["a", "b"]);
  assert.deepEqual(previewOrder(["a", "b"], 0, 9), ["a", "b"], "mimo rozsah = beze změny");
});

test("posun u okraje: blíž okraji = rychleji, uprostřed nic", () => {
  // mřížka od 100 do 500 px, pás 40 px, nejvýš 16 px za snímek
  const cases = [
    [300, 0],
    [141, 0],
    [140, 0],
    [120, -8],
    [100, -16],
    [40, -16],
    [480, 8],
    [500, 16],
    [900, 16],
  ];
  for (const [y, want] of cases) assert.equal(autoScrollStep(y, 100, 500) + 0, want, `${y}`);
  assert.equal(autoScrollStep(150, 100, 130), 0, "malá mřížka (méně než dva pásy) = bez posunu");
});

test("klávesy na úchytu: šipky přesun, + / − šířka, PageUp / PageDown výška", () => {
  const cases = [
    [{ key: "ArrowLeft" }, { move: -1 }],
    [{ key: "ArrowUp", altKey: true }, { move: -1 }],
    [{ key: "ArrowRight" }, { move: 1 }],
    [{ key: "ArrowDown" }, { move: 1 }],
    [{ key: "+" }, { size: 1 }],
    [{ key: "=" }, { size: 1 }],
    [{ key: "-" }, { size: -1 }],
    [{ key: "PageDown" }, { height: 1 }],
    [{ key: "PageUp" }, { height: -1 }],
    [{ key: "Enter" }, null],
    [{ key: "a" }, null],
    [{ key: "+", ctrlKey: true }, null],
  ];
  for (const [e, want] of cases) assert.deepEqual(keyAction(e), want, JSON.stringify(e));
});
