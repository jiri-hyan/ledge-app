// Tabulkové testy čisté logiky karty Přihrádka (`model.ts`): práh tahu ven, viditelnost karty,
// poznání tahu souborů.
//   node --experimental-strip-types --test src/shelf/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { DRAG_PX, cardVisible, carriesFiles, isDrag } from "./model.ts";

const item = (path, missing = false) => ({ path, name: path, dir: "", missing, addedMs: 0 });

test("práh tahu ven: menší pohyb je klik", () => {
  const cases = [
    [0, 0, false],
    [DRAG_PX - 1, 0, false],
    [3, 4, false],
    [DRAG_PX, 0, true],
    [0, -DRAG_PX, true],
    [5, 5, true],
  ];
  for (const [dx, dy, want] of cases) assert.equal(isDrag(dx, dy), want, `${dx},${dy}`);
});

test("karta jen se zapnutou a neprázdnou přihrádkou", () => {
  const cases = [
    [undefined, false],
    [{ enabled: true, items: [] }, false],
    [{ enabled: false, items: [item("C:\a")] }, false],
    [{ enabled: true, items: [item("C:\a")] }, true],
  ];
  for (const [l, want] of cases) assert.equal(cardVisible(l), want, JSON.stringify(l));
});

test("tah souborů vs. přeřazování uvnitř šuplíku", () => {
  const cases = [
    [["Files"], true],
    [["text/plain", "Files"], true],
    [["text/plain"], false],
    [[], false],
    [undefined, false],
    [null, false],
  ];
  for (const [types, want] of cases) assert.equal(carriesFiles(types), want, String(types));
});
