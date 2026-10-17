// Tabulkové testy kompaktní palety (`compact.ts`): co ukázat při daném dotazu a režimu, šipky.
//   node --experimental-strip-types --test src/palette/compact.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { arrow, shown, visible } from "./compact.ts";
import { rank } from "./search.ts";

test("shown: tabulka režimu, dotazu a rozbalení", () => {
  /** [compact, dotaz, rozbaleno šipkou, počet nedávných, čekáme] */
  const cases = [
    [false, "", false, 0, "list"],
    [false, "", false, 3, "list"],
    [false, "nas", false, 3, "list"],
    [true, "", false, 3, "none"],
    [true, "   ", false, 3, "none"],
    [true, "", true, 3, "recent"],
    [true, "", true, 0, "list"],
    [true, "nas", false, 0, "list"],
    [true, "nas", true, 3, "list"],
  ];
  for (const [compact, query, browsing, n, want] of cases) {
    assert.equal(shown({ compact, query, browsing }, n), want, JSON.stringify({ compact, query, browsing, n }));
  }
});

test("visible: none = nic, recent = jen nedávné v pořadí rank, list = vše", () => {
  const items = [
    { id: "a", title: "Nastavení" },
    { id: "b", title: "Nová poznámka" },
    { id: "c", title: "Projekt Harbor" },
  ];
  const recent = ["c", "a"];
  const ranked = rank(items, "", recent);
  const set = new Set(recent);
  assert.deepEqual(visible(ranked, "none", set), []);
  assert.deepEqual(visible(ranked, "recent", set).map((r) => r.item.id), ["c", "a"]);
  assert.deepEqual(visible(ranked, "list", set).map((r) => r.item.id), ["c", "a", "b"]);
});

test("arrow: sbalená kompaktní paleta, rozbalení a sbalení, běžné listování", () => {
  const F = (compact, query, browsing) => ({ compact, query, browsing });
  /** [klávesa, stav, výběr, viditelných řádků, čekáme {sel, browsing}] */
  const cases = [
    // sbalená: ↓ rozbalí na první řádek, ↑ nic
    ["ArrowDown", F(true, "", false), 0, 0, { sel: 0, browsing: true }],
    ["ArrowUp", F(true, "", false), 0, 0, { sel: 0, browsing: false }],
    // rozbalená bez dotazu: ↑ z prvního sbalí, jinak listuje
    ["ArrowUp", F(true, "", true), 0, 3, { sel: 0, browsing: false }],
    ["ArrowUp", F(true, "", true), 2, 3, { sel: 1, browsing: true }],
    ["ArrowDown", F(true, "", true), 2, 3, { sel: 0, browsing: true }],
    // s dotazem jako běžná paleta (dokola), browsing se nemění
    ["ArrowUp", F(true, "x", false), 0, 3, { sel: 2, browsing: false }],
    ["ArrowDown", F(true, "x", false), 1, 3, { sel: 2, browsing: false }],
    // běžná paleta: dokola, prázdný seznam = výběr 0
    ["ArrowUp", F(false, "", false), 0, 4, { sel: 3, browsing: false }],
    ["ArrowDown", F(false, "", false), 3, 4, { sel: 0, browsing: false }],
    ["ArrowDown", F(false, "zz", false), 0, 0, { sel: 0, browsing: false }],
  ];
  for (const [key, f, sel, n, want] of cases) {
    assert.deepEqual(arrow(key, f, sel, n), want, `${key} ${JSON.stringify(f)} sel=${sel} n=${n}`);
  }
});
