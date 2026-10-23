// Lístky na plochu — čistá logika (`model.ts`): přidání bodu / textu, barvy, připnutí ze snapshotu.
import { test } from "node:test";
import assert from "node:assert/strict";
import { anyThrough, appendLine, caretFromText, isPinned, isTaskLine, pinColors, pinSuggestVisible, pointLine, remindAskText, samePath, taskLine, TEXT_MIN } from "./model.ts";

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

test("číslovaný seznam: řádky se lepí a „+ bod“ pokračuje číslováním", () => {
  assert.ok(isTaskLine("1. a") && isTaskLine("  12) b") && !isTaskLine("1.a") && !isTaskLine("rok 2024. dobrý"));
  const pointCases = [
    ["1. a\n2. b\n", "nová položka", "3. nová položka"],
    ["Seznam\n\n1) a\n2) b", "c", "3) c"],
    ["  1. vnořená", "x\ny", "  2. x y"],
    ["9. a", "b", "10. b"],
    ["- [ ] a", "b", "- [ ] b"],
    ["text", "b", "- [ ] b"],
    ["", "b", "- [ ] b"],
    ["1. a", "  ", ""],
  ];
  for (const [body, text, want] of pointCases) assert.equal(pointLine(body, text), want, JSON.stringify([body, text]));
  const cases = [
    ["1. a\n2. b\n", "3. c", "1. a\n2. b\n3. c"],
    ["Lístek\n\n1. a", "- [ ] b", "Lístek\n\n1. a\n- [ ] b"],
    ["1. a", "text mimo seznam", "1. a\n\ntext mimo seznam"],
  ];
  for (const [body, line, want] of cases) assert.equal(appendLine(body, line), want, JSON.stringify([body, line]));
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

test("caretFromText: dvojklik do vykresleného textu → pozice v těle", () => {
  const body = "Nadpis\n\n- [ ] koupit **mléko** a chleba\n2. druhý bod\nposlední řádek";
  const at = (s, i) => body.indexOf(s) + i;
  const cases = [
    ["Nadpis", 3, 3],
    ["koupit ", 3, at("koupit", 3)],
    [" a chleba", 3, at(" a chleba", 3)],
    ["druhý bod", 0, at("druhý bod", 0)],
    ["druhý bod", 9, at("druhý bod", 9)],
    ["poslední řádek", 14, body.length],
    ["neexistuje", 2, null],
    ["   ", 1, null],
  ];
  for (const [node, off, want] of cases) assert.equal(caretFromText(body, node, off), want, JSON.stringify([node, off]));
});

test("připomínka z přidaného bodu: ptát se jen na časový text a jen u poznámky bez připomínky", () => {
  const cases = [
    ["zítra v 10 zavolat X", null, "zítra v 10 zavolat X"],
    ["  zítra   v 10\n zavolat ", undefined, "zítra v 10 zavolat"],
    ["koupit mléko", null, null],
    ["do 25. 10. odevzdat", null, "do 25. 10. odevzdat"],
    ["zítra v 10 zavolat X", 1_800_000_000_000, null],
    ["ab", null, null],
    ["", null, null],
  ];
  for (const [text, remindMs, want] of cases) assert.equal(remindAskText(text, remindMs), want, JSON.stringify(text));
  assert.equal(remindAskText("zítra ".repeat(10_000), null).length, 20_000);
  const sg = { label: "zítra 10:00", iso: "2026-10-24T10:00:00+02:00" };
  assert.equal(pinSuggestVisible(sg, null), sg);
  assert.equal(pinSuggestVisible(sg, 5), null, "poznámka už připomínku má");
  assert.equal(pinSuggestVisible({ label: "x", iso: "nesmysl" }, null), null, "nepoužitelné ISO");
  assert.equal(pinSuggestVisible(null, null), null);
});
