// Widgety na ploše — čistá logika UI (bez DOM): `corepack pnpm test:ui`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { BUILTIN_KINDS, canPin, clampOpacity, cleanProfileName, deskSnapshot, isKind, kindName, kindsOfCard, LAYERS, liveCount, sameName, waitMinutes, widgetOf } from "./model.ts";

const w = (id, kind, x = {}) => ({ id, kind, x: null, y: null, w: 300, h: 120, cw: 200, ch: 70, compact: false, layer: "normal", locked: false, through: false, opacity: 100, hideInGame: true, ...x });

test("co jde připnout z které karty", () => {
  const cases = [
    ["claude", ["limits", "waiting"]],
    ["running", ["dev"]],
    ["calendar", ["agenda"]],
    ["week", ["week"]],
    ["bigclock", ["clock"]],
    ["usage", ["usage"]],
    ["clock", []],
    ["widget:docker.ps1", ["widget:docker.ps1"]],
    ["widget:", []],
    ["projects", []],
    ["notes", []],
    ["", []],
  ];
  for (const [card, want] of cases) assert.deepEqual(kindsOfCard(card), want, card);
  for (const k of [...BUILTIN_KINDS, "widget:a.cmd"]) assert.ok(isKind(k), k);
  for (const k of ["widget:", "claude", "", 3, null]) assert.ok(!isKind(k), String(k));
  assert.deepEqual(kindName("limits"), ["cards.desk.kind.limits", "Limity Clauda"]);
  assert.equal(kindName("widget:kurz.py"), "kurz.py");
});

test("zrcadlo Rustu: druhy, vrstvy, strop", () => {
  const rs = readFileSync(new URL("../../src-tauri/src/desk/mod.rs", import.meta.url), "utf8");
  for (const k of BUILTIN_KINDS) assert.ok(rs.includes(`"${k}" => Some(Kind::`), `Kind::parse zná ${k}`);
  assert.deepEqual(LAYERS.map((l) => l.v), ["top", "normal", "desktop"]);
  for (const l of ["Top", "Normal", "Desktop"]) assert.ok(rs.includes(`    ${l},`), `Layer::${l}`);
  assert.ok(/pub const OPACITY_MIN: u8 = 30;/.test(rs));
});

test("stav připnutí ze snapshotu a strop", () => {
  const v = { widgets: [w("lim1", "limits"), w("dev1", "dev")], profiles: [], active: null, hiddenAll: false, max: 2 };
  assert.equal(widgetOf(v, "limits")?.id, "lim1");
  assert.equal(widgetOf(v, "agenda"), null);
  assert.equal(widgetOf(undefined, "limits"), null);
  assert.ok(canPin(v, "limits"), "připnutý jde vždy (jen se ukáže)");
  assert.ok(!canPin(v, "agenda"), "plno");
  assert.ok(canPin({ ...v, max: 6 }, "agenda"));
  assert.ok(canPin(undefined, "agenda"), "staré jádro bez `desk` = zkusit");
});

test("průhlednost, jméno profilu, minuty čekání", () => {
  for (const [v, want] of [[0, 30], [29.6, 30], [55.4, 55], [100, 100], [250, 100], [NaN, 100]]) assert.equal(clampOpacity(v), want, String(v));
  assert.equal(cleanProfileName("  Práce   doma  "), "Práce doma");
  assert.equal(cleanProfileName("   "), "");
  assert.equal(cleanProfileName("x".repeat(60)).length, 40);
  assert.ok(sameName("PRÁCE", " práce "));
  assert.ok(!sameName("Práce", "Hraní"));
  assert.equal(waitMinutes(0, 125_000), 2);
  assert.equal(waitMinutes(10_000, 0), 0, "hodiny napřed → nikdy záporné");
});

test("data widgetu ve tvaru snapshotu: co chybí, je prázdné", () => {
  const empty = deskSnapshot(null);
  assert.deepEqual([empty.procs, empty.projects, empty.widgets, empty.sessions], [[], [], [], []]);
  assert.equal(empty.cfgError, null);
  assert.equal(empty.personal.calendar, undefined);
  const d = { now: 60_000, cfgError: "x", claudeEnabled: true, working: true, procs: [{ id: "app", phase: "ready" }, { id: "app-v2", phase: "failed" }], projects: [{ id: "app", name: "App" }] };
  const s = deskSnapshot(d);
  assert.equal(s.sessions[0].status, "working");
  assert.equal(s.projects[0].name, "App");
  assert.equal(s.cfgError, "x");
  assert.equal(liveCount(d), 1);
  assert.equal(liveCount(null), 0);
});

test("okno widgetu nikdy neschvaluje: žádné decide v kódu desk/", () => {
  for (const f of ["DeskApp.tsx", "views.tsx", "api.ts"]) {
    const text = readFileSync(new URL(`./${f}`, import.meta.url), "utf8");
    assert.ok(!/\bdecide\b|answer_question|widget_consent|widgetsApi\.consent/.test(text), `${f} nic neschvaluje`);
  }
});
