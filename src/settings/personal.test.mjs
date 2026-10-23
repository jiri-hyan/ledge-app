// Čistá logika panelů Osobní a Začínáme: pořadí sekcí podle rozložení (`personalModel.ts`), pole Město
// (`cityModel.ts`, zrcadlo Rust `weather::clean_query`) a „Co je nového“ jen po přechodu z nižší verze
// (`startModel.ts`). Bez sítě a bez DOM.
//   node --experimental-strip-types --test src/settings/personal.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { PERSONAL_SECTIONS, layoutCardIds, splitPersonal } from "./personalModel.ts";
import { GEO_DEBOUNCE_MS, GEO_MIN_CHARS, LABEL_MAX, geoQuery, placeLabel, placeToWeather } from "./cityModel.ts";
import { cmpVersion, parseVersion, upgradedFrom } from "./startModel.ts";

const page = (...ids) => ({ id: "p", name: "P", cards: ids.map((id) => ({ id, size: "M" })) });

test("layoutCardIds: karty ze všech záložek, rozbitá struktura = nic", () => {
  assert.deepEqual(layoutCardIds({ pages: [page("weather", "clock"), page("calendar")] }), ["weather", "clock", "calendar"]);
  for (const bad of [null, undefined, 5, {}, { pages: "x" }, { pages: [null, { cards: 3 }, { cards: [null, { id: 4 }] }] }]) assert.deepEqual(layoutCardIds(bad), [], String(bad));
});

test("splitPersonal: nejdřív sekce karet z rozložení, zbytek za „Další sekce“", () => {
  const cases = [
    [[], [], ["weather", "calendars", "media", "timer", "github", "system", "suggest", "clipboard"]],
    [["weather", "calendar"], ["weather", "calendars"], ["media", "timer", "github", "system", "suggest", "clipboard"]],
    [["system", "weather"], ["weather", "system"], ["calendars", "media", "timer", "github", "suggest", "clipboard"]],
    [["clock", "projects", "weather"], ["weather"], ["calendars", "media", "timer", "github", "system", "suggest", "clipboard"]],
    [["weather", "calendar", "media", "timer", "github", "system"], ["weather", "calendars", "media", "timer", "github", "system"], ["suggest", "clipboard"]],
  ];
  for (const [cards, shown, rest] of cases) assert.deepEqual(splitPersonal(cards), { shown, rest }, cards.join());
  // každá sekce je právě v jedné části a pomocné sekce bez karty jsou vždy vzadu
  for (const s of PERSONAL_SECTIONS.filter((x) => !x.card)) assert.ok(splitPersonal(PERSONAL_SECTIONS.map((x) => x.card).filter(Boolean)).rest.includes(s.id));
});

test("Personal.tsx skládá všechny sekce z modelu", () => {
  const src = readFileSync(new URL("panels/Personal.tsx", import.meta.url), "utf8");
  for (const s of PERSONAL_SECTIONS) assert.ok(new RegExp(`^  ${s.id}: `, "m").test(src), s.id);
});

test("geoQuery: aspoň 2 znaky po ořezu, jinak se nic neptá", () => {
  const cases = [["", null], [" ", null], ["a", null], [" a ", null], ["Pr", "Pr"], ["  Plzeň ", "Plzeň"], ["Ž", null], ["Žd", "Žd"], ["😀", null]];
  for (const [raw, want] of cases) assert.equal(geoQuery(raw), want, JSON.stringify(raw));
  assert.equal(GEO_MIN_CHARS, 2);
  assert.ok(GEO_DEBOUNCE_MS >= 400, "debounce aspoň 400 ms");
});

test("placeLabel a placeToWeather", () => {
  const praha = { name: "Praha", region: "Hlavní město Praha", country: "Česko", lat: 50.088, lon: 14.4208 };
  assert.equal(placeLabel(praha), "Praha — Hlavní město Praha, Česko");
  assert.equal(placeLabel({ ...praha, region: "Praha" }), "Praha — Česko");
  assert.equal(placeLabel({ ...praha, region: null }), "Praha — Česko");
  assert.equal(placeLabel({ ...praha, region: null, country: null }), "Praha");
  assert.deepEqual(placeToWeather(praha), { lat: 50.088, lon: 14.4208, label: "Praha" });
  assert.equal(placeToWeather({ ...praha, name: "Š".repeat(60) }).label.length, LABEL_MAX);
});

test("pole Město nemá dotaz v klidu: geo_search se volá jen z časovače po psaní", () => {
  const src = readFileSync(new URL("panels/CityPicker.tsx", import.meta.url), "utf8");
  assert.equal((src.match(/geoSearch\(/g) ?? []).length, 1);
  assert.ok(src.indexOf("geoSearch(") > src.indexOf("setTimeout"), "dotaz až v časovači");
  assert.ok(!/onMount|createEffect|createResource/.test(src), "nic se nestahuje při otevření");
});

test("verze: parseVersion a cmpVersion", () => {
  assert.deepEqual(parseVersion("0.4.1"), [0, 4, 1]);
  assert.deepEqual(parseVersion("v1.2"), [1, 2]);
  for (const bad of ["", null, undefined, "abc", "1.x", "1..2", "1.2.3.4.5"]) assert.equal(parseVersion(bad), null, String(bad));
  const cases = [["0.4.0", "0.4.1", -1], ["0.4.1", "0.4.1", 0], ["0.10.0", "0.9.9", 1], ["1.0", "1.0.0", 0], ["x", "1.0.0", null], ["1.0.0", null, null]];
  for (const [a, b, want] of cases) assert.equal(cmpVersion(a, b), want, `${a} ? ${b}`);
});

test("upgradedFrom: „Co je nového“ jen po přechodu z nižší verze, nový uživatel ne", () => {
  const cases = [
    [null, "0.4.1", false], // poprvé = nový uživatel
    ["", "0.4.1", false],
    ["0.4.0", "0.4.1", true],
    ["0.4.1", "0.4.1", false],
    ["0.5.0", "0.4.1", false], // downgrade
    ["0.4.0", undefined, false],
    ["smetí", "0.4.1", false],
  ];
  for (const [seen, cur, want] of cases) assert.equal(upgradedFrom(seen, cur), want, `${seen} → ${cur}`);
});

test("Začínáme: Co je nového a staré názvy jen pro upgrade, tlačítko spouští průvodce znovu", () => {
  const src = readFileSync(new URL("panels/Start.tsx", import.meta.url), "utf8");
  assert.match(src, /<Show when=\{up\}>[\s\S]*Co je nového/);
  assert.match(src, /old \? t\("settings\.start\.tip\.search\.text"/);
  assert.match(src, /Spustit průvodce znovu/);
  assert.match(src, /startWizard\(\)/);
});
