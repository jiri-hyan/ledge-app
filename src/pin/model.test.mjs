// Lístky na plochu — čistá logika (`model.ts`): přidání bodu / textu, barvy, připnutí ze snapshotu.
import { test } from "node:test";
import assert from "node:assert/strict";
import { anyThrough, appendLine, isPinned, isTaskLine, pinColors, samePath, taskLine, TEXT_MIN } from "./model.ts";

test("taskLine: bod checklistu z textu", () => {
  const cases = [
    ["koupit mléko", "- [ ] koupit mléko"],
    ["  tlačítko\n nejde  ", "- [ ] tlačítko nejde"],
    ["", ""],
    ["   \n ", ""],
  ];
  for (const [inp, want] of cases) assert.equal(taskLine(inp), want, JSON.stringify(inp));
});

test("appendLine: bod pod bod, jinak nový odstavec, prázdné nic", () => {
  const cases = [
    ["", "- [ ] a", "- [ ] a"],
    ["Lístek 18. 10.", "- [ ] a", "Lístek 18. 10.\n\n- [ ] a"],
    ["Lístek\n\n- [ ] a\n", "- [ ] b", "Lístek\n\n- [ ] a\n- [ ] b"],
    ["- [x] hotovo", "- [ ] b", "- [x] hotovo\n- [ ] b"],
    ["- [ ] a", "https://example.com/stranka", "- [ ] a\n\nhttps://example.com/stranka"],
    ["text", "řádek 1\r\nřádek 2\r\n", "text\n\nřádek 1\nřádek 2"],
    ["text\n\n![](./a-1.png)\n", "- [ ] b", "text\n\n![](./a-1.png)\n\n- [ ] b"],
    ["beze změny  ", "   ", "beze změny  "],
  ];
  for (const [body, line, want] of cases) assert.equal(appendLine(body, line), want, JSON.stringify([body, line]));
  assert.ok(isTaskLine("  * [X] x") && !isTaskLine("- [] x") && !isTaskLine("[ ] x"));
});

test("pinColors: rozsahy, text nikdy pod hranici, stín u průhledného pozadí", () => {
  assert.deepEqual(pinColors(100, 100), { background: "rgba(24, 26, 32, 1)", bar: "rgba(24, 26, 32, 1)", color: "rgba(244, 245, 248, 1)", shadow: "none" });
  const glass = pinColors(20, 0);
  assert.equal(glass.background, "rgba(24, 26, 32, 0.2)");
  assert.equal(glass.bar, "rgba(24, 26, 32, 0.3)");
  assert.equal(glass.color, `rgba(244, 245, 248, ${TEXT_MIN / 100})`);
  assert.notEqual(glass.shadow, "none");
  assert.equal(pinColors(-5, 300).background, "rgba(24, 26, 32, 0)");
  assert.equal(pinColors(Number.NaN, Number.NaN).color, "rgba(244, 245, 248, 1)");
  assert.equal(pinColors(45, 80).shadow, "none");
});

test("připnutí ze snapshotu: cesty bez ohledu na velikost a lomítka", () => {
  const pins = { open: [{ path: "C:\\Users\\x\\Ledge\\notes\\_obecne\\a.md", through: false }, { path: "C:\\n\\b.md", through: true }], key: "Ctrl+Alt+P" };
  assert.ok(samePath("c:/users/X/ledge/notes/_obecne/A.md", pins.open[0].path));
  assert.ok(isPinned(pins, "c:/n/B.md"));
  assert.ok(!isPinned(pins, "C:\\n\\c.md"));
  assert.ok(!isPinned(undefined, "C:\\n\\b.md"));
  assert.ok(anyThrough(pins));
  assert.ok(!anyThrough({ open: [pins.open[0]], key: "" }));
  assert.ok(!anyThrough(null));
});
