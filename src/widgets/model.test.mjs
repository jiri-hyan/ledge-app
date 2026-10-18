// Widgety ze skriptů — čistá logika UI (bez DOM): id `widget:<soubor>` (stejná pravidla jako Rust
// `widgets::plan::file_of`), interval, klik na položku, průběh, katalog karet, položky proužku.
//   node --experimental-strip-types --test src/widgets/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { catalogWidgets, intervalParts, isStale, isWidgetId, itemKind, progressPct, stripWidgetIds, TONE_VAR, widgetFile, withStripItem } from "./model.ts";

test("id widgetu: jen jméno souboru (tabulka jako Rust file_of)", () => {
  const cases = [
    ["widget:kurz-cnb.ps1", "kurz-cnb.ps1"],
    ["widget:a b.py", "a b.py"],
    ["widget:", null],
    ["widget:..\\x.ps1", null],
    ["widget:sub/x.ps1", null],
    ["widget:C:x", null],
    ["widget:.skryty", null],
    ["widget:a\nb", null],
    [`widget:${"x".repeat(201)}`, null],
    ["clock", null],
    [42, null],
  ];
  for (const [id, want] of cases) {
    assert.equal(widgetFile(id), want, String(id));
    assert.equal(isWidgetId(id), want !== null, String(id));
  }
});

test("interval pro člověka", () => {
  const cases = [
    [30, { n: 30, unit: "s" }],
    [90, { n: 90, unit: "s" }],
    [300, { n: 5, unit: "min" }],
    [3600, { n: 60, unit: "min" }],
    [5400, { n: 1.5, unit: "h" }],
    [86400, { n: 24, unit: "h" }],
  ];
  for (const [s, want] of cases) assert.deepEqual(intervalParts(s), want, String(s));
});

test("klik na položku, průběh, stará data, barvy", () => {
  assert.equal(itemKind({ label: "a" }), null);
  assert.equal(itemKind({ label: "a", url: "https://x.cz", action: { type: "copy", text: "x" } }), "url", "odkaz má přednost");
  assert.equal(itemKind({ label: "a", action: { type: "run", script: "x.ps1", arg: null } }), "run");
  assert.equal(itemKind({ label: "", sep: true, url: "https://x.cz" }), null);
  for (const [p, want] of [[null, null], [0, 0], [0.424, 42], [1.7, 100], [-1, 0], [NaN, null]]) assert.equal(progressPct(p), want, String(p));
  assert.equal(isStale(null, 60, 1e6), false);
  assert.equal(isStale(0, 60, 119_000), false);
  assert.equal(isStale(0, 60, 121_000), true);
  assert.deepEqual(Object.keys(TONE_VAR), ["accent", "text", "muted", "violet", "blue", "teal", "green", "amber", "red"], "stejné předvolby jako proužek");
});

const w = (id, show = "both", title = id) => ({ id, show, title, description: null, icon: null });

test("katalog karet: widgety do karty, které na záložce nejsou, a kde jsou jinde", () => {
  const pages = [
    { id: "prace", cards: [{ id: "projects" }, { id: "widget:a.ps1" }] },
    { id: "osobni", cards: [{ id: "widget:b.py" }] },
  ];
  const ws = [w("widget:a.ps1"), w("widget:b.py", "card"), w("widget:c.cmd", "strip"), w("widget:d.js")];
  const got = catalogWidgets(ws, pages, "prace").map((x) => [x.id, x.on]);
  assert.deepEqual(got, [["widget:b.py", ["osobni"]], ["widget:d.js", []]], "jen-proužek a už umístěné ne");
});

test("položky proužku z widgetů", () => {
  const ws = [w("widget:a.ps1", "card"), w("widget:b.py", "strip"), w("widget:c.cmd")];
  assert.deepEqual(stripWidgetIds(ws, ["clock", "widget:c.cmd", "widget:zmizel.ps1"]), ["widget:c.cmd", "widget:zmizel.ps1", "widget:b.py"], "zapnuté v pořadí, smazaný soubor zůstane vidět");
  const items = ["clock"];
  assert.deepEqual(withStripItem(items, "widget:b.py", true), ["clock", "widget:b.py"]);
  assert.deepEqual(withStripItem(["clock", "widget:b.py"], "widget:b.py", false), ["clock"]);
  assert.deepEqual(items, ["clock"], "vstup netknutý");
});
