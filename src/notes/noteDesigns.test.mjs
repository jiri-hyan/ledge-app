// Vzhledy karty Poznámky: výchozí první, neznámé nespadne, starý `view` se převede, Kanban dělí na otevřené a hotové.
import { test } from "node:test";
import assert from "node:assert/strict";
import { groupCount, kanbanCount, kanbanSplit, LEGACY_VIEW_DESIGN, NOTES_DESIGNS, notesDesign } from "./noteDesigns.ts";
import { designOf, designsOf, storedDesign } from "../layout/designs.ts";
import { normalizeCardLook } from "../layout/look.ts";

test("šest vzhledů, výchozí seznam, id unikátní", () => {
  assert.deepEqual(NOTES_DESIGNS.map((d) => d.id), ["list", "compact", "cards", "bento", "kanban", "timeline"]);
  assert.equal(designsOf("notes").length, 6);
  assert.equal(designOf("notes", undefined)?.id, "list");
  assert.equal(designOf("notes", { design: "nesmysl" })?.id, "list");
  assert.equal(storedDesign("notes", "list"), undefined);
  assert.equal(storedDesign("notes", "kanban"), "kanban");
});

test("notesDesign: neznámé = seznam", () => {
  for (const [x, want] of [["kanban", "kanban"], ["zzz", "list"], [undefined, "list"]]) assert.equal(notesDesign(x), want);
});

test("starý config: view se převede na vzhled, prázdný look zůstane prázdný", () => {
  assert.deepEqual(LEGACY_VIEW_DESIGN, { tiles: "cards", compact: "compact", bento: "bento" });
  assert.deepEqual(normalizeCardLook({ view: "tiles" }, "notes"), { design: "cards" });
  assert.deepEqual(normalizeCardLook({ view: "bento", bg: "tint" }, "notes"), { bg: "tint", design: "bento" });
  assert.deepEqual(normalizeCardLook({ view: "compact", design: "kanban" }, "notes"), { design: "kanban" });
  assert.equal(normalizeCardLook({ view: "list" }, "notes"), undefined);
  assert.equal(normalizeCardLook({}, "notes"), undefined);
});

test("seskupení v looku: platné se ukládá, neznámé a výchozí ne", () => {
  assert.deepEqual(normalizeCardLook({ groupBy: "date", groupStyle: "cards" }, "notes"), { groupBy: "date", groupStyle: "cards" });
  assert.equal(normalizeCardLook({ groupBy: "none", groupStyle: "line" }, "notes"), undefined);
  assert.equal(normalizeCardLook({ groupBy: "zzz", groupStyle: 3 }, "notes"), undefined);
  assert.equal(normalizeCardLook({ groupBy: "date" }, "claude"), undefined);
});

test("kanbanSplit: otevřené a hotové, pořadí zůstává", () => {
  const r = kanbanSplit([{ id: 1, done: false }, { id: 2, done: true }, { id: 3, done: false }]);
  assert.deepEqual(r.open.map((x) => x.id), [1, 3]);
  assert.deepEqual(r.done.map((x) => x.id), [2]);
  assert.deepEqual(kanbanSplit([]), { open: [], done: [] });
});

test("počty v hlavičkách: kanban ukáže celek, skupiny dolní odhad při ořezu (N3b)", () => {
  for (const [shown, total, want] of [[8, 40, "8 / 40"], [3, 3, "3"], [0, 0, "0"], [5, null, "5"], [5, undefined, "5"], [8, 8, "8"], [8, 7, "8"]]) assert.equal(kanbanCount(shown, total), want, `${shown}/${total}`);
  for (const [shown, cut, want] of [[3, true, "3+"], [3, false, "3"], [0, true, "0"]]) assert.equal(groupCount(shown, cut), want, `${shown}/${cut}`);
});
