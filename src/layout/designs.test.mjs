// Registr vzhledů karet (kolo 25): výchozí je první, neznámé id nespadne, výchozí se nezapisuje.
import test from "node:test";
import assert from "node:assert/strict";
import { CARD_DESIGNS, defaultDesign, designOf, designsOf, storedDesign } from "./designs.ts";
import { normalizeCardLook, patchCardLook } from "./look.ts";

test("každá karta má unikátní id a páry [klíč, česky]", () => {
  for (const [card, list] of Object.entries(CARD_DESIGNS)) {
    assert.ok(list.length >= 2, card);
    assert.equal(new Set(list.map((d) => d.id)).size, list.length, card);
    for (const d of list) for (const x of [d.name, d.line]) assert.ok(x[0].startsWith("cards.design.") && x[1], `${card}/${d.id}`);
  }
});

test("claude a system mají sedm až devět vzhledů, výchozí první", () => {
  assert.equal(defaultDesign("claude"), "classic");
  assert.equal(defaultDesign("system"), "lines");
  assert.deepEqual(designsOf("claude").map((d) => d.id), ["classic", "rings", "pace", "hero", "heroWeek", "heroSpend", "spend", "sessions", "gauge"]);
  assert.equal(designsOf("system").length, 7);
  assert.equal(designsOf("notes").length, 6);
  assert.equal(defaultDesign("notes"), "list");
  assert.deepEqual(designsOf("actions"), []);
});

test("designOf: zvolený, neznámý i chybějící", () => {
  const cases = [
    ["claude", { design: "rings" }, "rings"],
    ["claude", { design: "nesmysl" }, "classic"],
    ["claude", undefined, "classic"],
    ["claude", null, "classic"],
    ["claude", {}, "classic"],
    ["system", { design: "net" }, "net"],
    ["system", { design: "rings" }, "rings"],
    ["notes", { design: "rings" }, "list"],
    ["notes", { design: "kanban" }, "kanban"],
    ["actions", { design: "rings" }, undefined],
  ];
  for (const [card, look, want] of cases) assert.equal(designOf(card, look)?.id, want, `${card} ${JSON.stringify(look)}`);
});

test("storedDesign: výchozí a neznámé se nezapisuje", () => {
  assert.equal(storedDesign("claude", "classic"), undefined);
  assert.equal(storedDesign("claude", "gauge"), "gauge");
  assert.equal(storedDesign("claude", "tiles"), undefined);
  assert.equal(storedDesign("claude", 5), undefined);
  assert.equal(storedDesign("actions", "gauge"), undefined);
  assert.equal(storedDesign("notes", "gauge"), undefined);
});

test("normalizeCardLook a patchCardLook se vzhledem karty", () => {
  assert.deepEqual(normalizeCardLook({ design: "gauge" }, "claude"), { design: "gauge" });
  assert.equal(normalizeCardLook({ design: "gauge" }, "system"), undefined, "cizí id");
  assert.equal(normalizeCardLook({ design: "lines" }, "system"), undefined, "výchozí");
  assert.equal(normalizeCardLook({ design: 3 }, "system"), undefined);
  assert.deepEqual(normalizeCardLook({ accent: "teal", design: "bars" }, "system"), { accent: "teal", design: "bars" });
  assert.deepEqual(patchCardLook(undefined, "claude", { design: "pace" }), { design: "pace" });
  assert.equal(patchCardLook({ design: "pace" }, "claude", { design: undefined }), undefined);
  assert.equal(patchCardLook({ design: "pace" }, "claude", { design: "classic" }), undefined, "klik na výchozí = pryč");
  // starý config bez klíče se čte beze změny
  assert.deepEqual(normalizeCardLook({ accent: "blue", hero: true }, "claude"), { accent: "blue", hero: true });
});
