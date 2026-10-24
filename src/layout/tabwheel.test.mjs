// Tabulkové testy kolečka nad záložkami (`tabwheel.ts`).
//   node --experimental-strip-types --test src/layout/tabwheel.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { COOLDOWN_MS, IDLE_MS, nextTab, pxOf, STEP_PX, WHEEL_INIT, wheelAllowed, wheelStep } from "./tabwheel.ts";

/** projde posloupnost [čas, deltaY] a vrátí směry */
function run(events) {
  let st = WHEEL_INIT;
  return events.map(([at, dy]) => {
    const r = wheelStep(st, dy, at);
    st = r.state;
    return r.dir;
  });
}

test("jeden zářez myši = jedna záložka", () => {
  assert.deepEqual(run([[1000, 100]]), [1]);
  assert.deepEqual(run([[1000, -100]]), [-1]);
});

test("jemný touchpad se sčítá do prahu", () => {
  assert.deepEqual(run([[1000, 10], [1010, 10], [1020, 10], [1030, 15]]), [0, 0, 0, 1]);
});

test("po přepnutí chvíli nic (proti trojímu přepnutí jedním otočením)", () => {
  assert.deepEqual(run([[1000, 100], [1050, 100], [1100, 100]]), [1, 0, 0]);
  assert.deepEqual(run([[1000, 100], [1000 + COOLDOWN_MS + 1, 100]]), [1, 1]);
});

test("pauza nasčítané zapomene", () => {
  assert.deepEqual(run([[1000, 30], [1000 + IDLE_MS + 10, 30]]), [0, 0]);
});

test("obrácený směr začíná znovu", () => {
  assert.deepEqual(run([[1000, 30], [1010, -30], [1020, -30]]), [0, 0, -1]);
});

test("nasčítané nad práh během odstupu nepřeteče a po odstupu přepne", () => {
  assert.deepEqual(run([[1000, 100], [1100, 500], [1000 + COOLDOWN_MS + 5, 1]]), [1, 0, 1]);
});

test("pxOf: řádky a stránky", () => {
  assert.equal(pxOf(3, 0), 3);
  assert.equal(pxOf(3, 1), 48);
  assert.equal(pxOf(1, 2), 400);
  assert.ok(pxOf(1, 1) < STEP_PX); // jeden řádek nestačí na přepnutí
});

test("nextTab: bez zalamování", () => {
  const cases = [[0, 3, 1, 1], [1, 3, 1, 2], [2, 3, 1, null], [0, 3, -1, null], [2, 3, -1, 1], [-1, 3, 1, null], [0, 1, 1, null]];
  for (const [cur, n, dir, want] of cases) assert.equal(nextTab(cur, n, dir), want, `${cur}/${n}/${dir}`);
});

test("wheelAllowed: volba, úpravy, počet záložek, Ctrl (zoom)", () => {
  const ok = { enabled: true, editing: false, tabs: 2, ctrl: false };
  const cases = [["ok", ok, true], ["vypnuto", { ...ok, enabled: false }, false], ["v úpravách", { ...ok, editing: true }, false], ["jedna záložka", { ...ok, tabs: 1 }, false], ["Ctrl", { ...ok, ctrl: true }, false]];
  for (const [name, o, want] of cases) assert.equal(wheelAllowed(o), want, name);
});

test("zapojení: Tabs.tsx čte volbu smart.tabWheel a posluchač je na seznamu záložek", () => {
  const src = fs.readFileSync(new URL("./Tabs.tsx", import.meta.url), "utf8");
  assert.ok(src.includes("smart?.tabWheel"));
  assert.ok(/role="tablist"[^>]*onWheel=\{onWheel\}/.test(src));
});
