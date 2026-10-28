// Test ukázkových sad (kolo 24): smyšlená jména, jen example.com, platné rozložení, šířky šuplíku, demo jen mimo Tauri.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { SET_IDS, askFor, buildSet, drawerWidth } from "./sets.ts";
import { normalizeLayout } from "../layout/model.ts";

const NOW = Date.UTC(2026, 9, 28, 12, 0);
const REAL_COMPANIES = /google|microsoft|amazon|github\.com|facebook|apple|netflix|spotify/i;

test("sady: tři, každá má projekty a platné rozložení", () => {
  assert.deepEqual([...SET_IDS], ["studio", "creator", "minimal"]);
  for (const id of SET_IDS) {
    const d = buildSet(id, NOW);
    const l = normalizeLayout(d.layout);
    assert.ok(l.pages.length >= 1, id);
    assert.equal(JSON.stringify(l.pages.map((p) => p.cards.length)), JSON.stringify(d.layout.pages.map((p) => p.cards.length)), `${id}: normalizace nezahodila karty`);
    assert.ok((d.snap.projects ?? []).length >= 2, id);
  }
  assert.equal(buildSet("studio", NOW).snap.projects.length, 6);
  assert.equal(buildSet("creator", NOW).snap.projects.length, 3);
  assert.equal(buildSet("minimal", NOW).snap.projects.length, 2);
});

test("sady: studio má schválení a 2 session, čas je relativní k now", () => {
  const d = buildSet("studio", NOW);
  assert.equal(d.snap.sessions.length, 2);
  assert.equal(d.snap.pending.length, 1);
  assert.ok(d.snap.pending[0].expiresAt > NOW);
  assert.deepEqual(buildSet("studio", NOW), buildSet("studio", NOW));
  assert.equal(askFor(NOW).questions[0].options.length, 3);
});

test("sady: jen smyšlená jména a example.com (žádné skutečné firmy ani cizí adresy)", () => {
  for (const id of SET_IDS) {
    const text = JSON.stringify(buildSet(id, NOW)) + JSON.stringify(askFor(NOW));
    assert.ok(!REAL_COMPANIES.test(text), id);
    for (const m of text.matchAll(/https?:\/\/([^/"':]+)/g)) assert.ok(["example.com", "localhost"].includes(m[1]), `${id}: ${m[1]}`);
  }
});

test("drawerWidth: tabulka podle geom.rs", () => {
  const cases = [
    [{ width: "narrow" }, 820],
    [{ width: "medium" }, 1080],
    [{ width: "wide" }, 1440],
    [{ width: "custom" }, 1080],
    [{ layout: "notch", width: "wide" }, 720],
    [{ width: "wide", size: { width: 900 } }, 900],
  ];
  for (const [a, want] of cases) assert.equal(drawerWidth(a), want, JSON.stringify(a));
});

test("demo se v Tauri nikdy nezapne (index.tsx i api.ts)", () => {
  assert.match(readFileSync(new URL("../index.tsx", import.meta.url), "utf8"), /!inTauri && new URLSearchParams\(location\.search\)\.get\("demo"\) === "1"/);
  assert.match(readFileSync(new URL("../api.ts", import.meta.url), "utf8"), /export function setDemoHost[^}]*if \(!inTauri\)/s);
});
