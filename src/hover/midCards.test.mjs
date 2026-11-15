// Karty středního panelu docku (L147): ověření seznamu a přepínání v editoru — zrcadlo Rust `hovermode::dock_cards`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { DOCK_CARDS_DEFAULT, dockCardsOf, toggleDockCard } from "./midCards.ts";

test("dockCardsOf: neznámé pryč, bez opakování, max 3, prázdné = výchozí (jako Rust dock_cards)", () => {
  const def = [...DOCK_CARDS_DEFAULT];
  const cases = [
    [undefined, def], [null, def], ["notes", def], [[], def], [["nesmysl", ""], def], [[1, null], def], [["widget:x.js"], def],
    [["claude"], ["claude"]],
    [["timer", "x", "clock"], ["timer", "clock"]],
    [["claude", "claude", "notes"], ["claude", "notes"]],
    [["claude", "notes", "calendar", "mail"], ["claude", "notes", "calendar"]],
    [["x", "a", "projects", "running", "repos", "media"], ["projects", "running", "repos"]],
    [["week", "bigclock", "links", "toggles"], ["calendar", "clock", "actions"]], // sloučené karty (kolo 34) → nástupce, max 3
    [["week", "calendar"], ["calendar"]],
  ];
  for (const [raw, want] of cases) assert.deepEqual(dockCardsOf(raw), want, JSON.stringify(raw));
});

test("výchozí tři karty existují v katalogu a jsou tři", async () => {
  const { CARD_IDS } = await import("../layout/model.ts");
  assert.equal(DOCK_CARDS_DEFAULT.length, 3);
  for (const id of DOCK_CARDS_DEFAULT) assert.ok(CARD_IDS.includes(id), id);
});

test("toggleDockCard: přidat do tří, odebrat, poslední nejde vypnout", () => {
  assert.deepEqual(toggleDockCard(["claude"], "timer"), ["claude", "timer"]);
  assert.deepEqual(toggleDockCard(["claude", "timer"], "claude"), ["timer"]);
  assert.deepEqual(toggleDockCard(["claude"], "claude"), ["claude"], "poslední zůstane");
  assert.deepEqual(toggleDockCard(["a", "b", "c"].map(() => "claude"), "mail"), ["claude", "mail"]);
  assert.deepEqual(toggleDockCard(["claude", "notes", "calendar"], "mail"), ["claude", "notes", "calendar"], "víc než 3 nejde");
  assert.deepEqual(toggleDockCard(undefined, "mail"), [...DOCK_CARDS_DEFAULT], "plné výchozí (3) + čtvrtá nejde");
});
