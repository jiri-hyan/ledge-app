// Domov Studia (kolo 24): kresba z reálného layoutu, karta → část, zóny = 8 stanic, „Na co nezapomenout“
// jen ze skutečného stavu (nic = žádná karta).
//   node --experimental-strip-types --test src/studio/anatomy.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { HOME_MAX_CARDS, ZONES, cardPart, homeModel, reminders } from "./anatomy.ts";
import { PART_IDS } from "./parts.ts";
import { CARD_IDS } from "../layout/model.ts";

const layout = (cards) => ({ pages: [{ id: "prace", name: "Práce", cards: cards.map((id) => ({ id, size: "m" })) }, { id: "osobni", name: "Osobní", cards: [] }] });

test("homeModel: záložky, aktivní, karty s šířkou a částí, přebytek", () => {
  const h = homeModel({ pages: [{ id: "a", name: "A", cards: [{ id: "projects", size: "xl" }, { id: "weather", size: "s" }, { id: "claude" }] }, { id: "b", name: "B" }] }, "a");
  assert.deepEqual(h.tabs, [{ id: "a", name: "A", on: true }, { id: "b", name: "B", on: false }]);
  assert.deepEqual(h.cards, [{ id: "projects", span: 4, part: "projekty" }, { id: "weather", span: 1, part: "obsah" }, { id: "claude", span: 2, part: "claude" }]);
  assert.equal(homeModel({ pages: [{ id: "a", name: "A" }, { id: "b", name: "B" }] }, "b").tabs[1].on, true);
  assert.deepEqual(homeModel(null), { tabs: [], cards: [], more: 0 });
  const many = homeModel(layout(Array.from({ length: 11 }, () => "clock")));
  assert.equal(many.cards.length, HOME_MAX_CARDS);
  assert.equal(many.more, 11 - HOME_MAX_CARDS);
});

test("každá vestavěná karta má část; zóny = 8 stanic, každá jednou", () => {
  for (const id of CARD_IDS) assert.ok(PART_IDS.includes(cardPart(id)), id);
  assert.equal(cardPart("widget:x.ps1"), "obsah");
  assert.deepEqual(ZONES.map((z) => z.part), PART_IDS);
  assert.equal(new Set(ZONES.map((z) => z.zone)).size, 8);
});

test("Na co nezapomenout: jen ze skutečného stavu", () => {
  const ids = (x) => reminders(x).map((r) => r.id);
  const full = { layout: layout(["weather", "calendar", "projects"]), autostart: false, personal: { weather: { lat: null, lon: null } } };
  const cases = [
    [{ cfg: {} }, []],
    [{ cfg: { autostart: true }, hooks: { state: "full" }, update: { state: "aktualni" } }, []],
    [{ cfg: full, hooks: { state: "none" }, update: { state: "dostupna" } }, ["hooks", "update", "city", "calendar", "project", "autostart"]],
    [{ cfg: { ...full, claude: { enabled: false } }, hooks: { state: "none" } }, ["city", "calendar", "project", "autostart"]],
    [{ cfg: full, packaged: true }, ["city", "calendar", "project"]],
    [{ cfg: { layout: layout(["weather"]), personal: { weather: { lat: 50, lon: 14, label: "Město" } } } }, []],
    [{ cfg: { layout: layout(["calendar"]), accounts: { google: { email: "uzivatel@example.com" } } } }, []],
    [{ cfg: { layout: layout(["projects"]), roots: ["C:/repos"] } }, []],
    [{ cfg: { layout: layout([]), autostart: true }, hooks: { state: "partial" } }, ["hooks"]],
    [{ cfg: { autostart: true }, hooks: null, update: null }, []],
  ];
  for (const [x, want] of cases) assert.deepEqual(ids(x), want, JSON.stringify(x));
  for (const r of reminders({ cfg: full, hooks: { state: "none" }, update: { state: "dostupna" } })) assert.ok(PART_IDS.includes(r.part) && r.go, r.id);
});
