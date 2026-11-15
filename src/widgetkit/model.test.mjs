// @vitest-environment node
// Widgetkit — resolver `*.kit.json` a tabulka zdrojů (čisté funkce, bez DOM).
//   node --experimental-strip-types --test src/widgetkit/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { FORMATS, fmtValue, parseTemplate, resolve, toneOf } from "./model.ts";
import { SOURCES, sourceValue } from "./sources.ts";
import { deskSnapshot } from "../desk/model.ts";

const NOW = Date.UTC(2026, 10, 14, 9, 5, 0); // lokální čas může být posunutý; hodiny testujeme jen tvarem
const MIN = 60_000;

/** Snímek ve tvaru drawer `Snapshot` (jen pole, která čtou zdroje). */
const snap = () => ({
  limits: { at: NOW - MIN, staleMs: 600_000, fiveHour: { pct: 82.6, resetsAt: NOW + 90 * MIN }, sevenDay: { pct: 20, resetsAt: NOW + 3 * 1440 * MIN } },
  pending: [{ id: 1 }],
  questions: [{ id: 2 }, { id: 3 }],
  procs: [{ phase: "ready" }, { phase: "running" }, { phase: "failed" }],
  costs: { state: "ok", summary: { ranges: [{ days: 1, total: { cost: 3.456 } }, { days: 7, total: { cost: 120 } }] } },
  personal: { calendar: { days: [{ items: [
    { start: NOW - 2 * 3600e3, end: NOW - 3600e3, allDay: false, title: "Už skončilo" },
    { start: NOW, end: NOW + 3600e3, allDay: true, title: "Celý den" },
    { start: NOW + 30 * MIN, end: NOW + 60 * MIN, allDay: false, title: "Stand-up" },
    { start: NOW + 5 * 3600e3, end: NOW + 6 * 3600e3, allDay: false, title: "Později" },
  ] }] } },
});

/** Stejná data ve tvaru `DeskData` (plocha). */
const deskData = () => {
  const s = snap();
  return {
    now: NOW, cfgError: null, claudeEnabled: true, working: false,
    limits: s.limits, procs: s.procs, costs: s.costs, calendar: s.personal.calendar,
    waitingTotal: 3,
    waiting: [{ id: 1, kind: "approve" }, { id: 2, kind: "question" }, { id: 3, kind: "question" }],
  };
};

test("šablona: kusy textu, formáty, chyby syntaxe", () => {
  assert.deepEqual(parseTemplate("Čeká: {waiting.count}"), [{ lit: "Čeká: " }, { src: "waiting.count", format: undefined }]);
  assert.deepEqual(parseTemplate("{a|usd} a {b}"), [{ src: "a", format: "usd" }, { lit: " a " }, { src: "b", format: undefined }]);
  assert.deepEqual(parseTemplate("bez zdroje"), [{ lit: "bez zdroje" }]);
  for (const bad of ["{a", "a}", "{}", "{a|b|c}", "{ }"]) assert.equal(parseTemplate(bad), null, bad);
});

test("formátovače: usd dur pct hm date, null = pomlčka", () => {
  assert.equal(fmtValue(null, "usd"), "—");
  assert.equal(fmtValue(null), "—");
  assert.equal(FORMATS.pct(82.6), "82 %");
  assert.equal(FORMATS.hm(125), "2:05");
  assert.equal(FORMATS.hm("09:05"), "09:05");
  assert.equal(FORMATS.dur(90), "1 h 30 min");
  assert.equal(FORMATS.dur(-5), "0 s");
  assert.match(FORMATS.usd(3.456), /3[,.]46/);
  assert.match(FORMATS.usd(120.4), /^120\b/);
  assert.equal(fmtValue(7), "7");
  assert.equal(fmtValue("text"), "text");
});

test("tón z prahů: nejvyšší dosažený, null / neplatné = žádný", () => {
  const over = [[80, "amber"], [95, "red"]];
  const cases = [[10, undefined], [80, "amber"], [94.9, "amber"], [95, "red"], [100, "red"], [null, undefined], ["x", undefined]];
  for (const [v, want] of cases) assert.equal(toneOf(over, v), want, String(v));
  assert.equal(toneOf([[1, "red"], [0, "ok"]], 5), "red", "pořadí v poli nehraje roli");
  assert.equal(toneOf([[1, "neon"], ["x", "red"], null], 5), undefined);
  assert.equal(toneOf(undefined, 5), undefined);
  assert.equal(toneOf([[0, "dim"]], 0), "muted");
});

test("resolve: kokpit — pruh s tónem, počet čekajících, akce", () => {
  const spec = { kit: 1, title: "Kokpit", rows: [
    { bar: "limits.five.pct", label: "5 h", tone: { over: [[80, "amber"], [95, "red"]] } },
    { bar: "limits.week.pct", label: "Týden" },
    { text: "Čeká: {waiting.count}", tone: { over: [[1, "red"]] }, open: "drawer:claude" },
    { text: "Dnes {costs.today|usd}", copy: "{costs.today}" },
    { text: "Další: {calendar.next.title} za {calendar.next.inMin|dur}" },
  ], buttons: [{ label: "Build", action: "build-ledge" }] };
  const d = resolve(spec, snap(), NOW);
  assert.equal(d.title, "Kokpit");
  assert.equal(d.notes, undefined);
  const [a, b, c, e, f, sep, btn] = d.items;
  assert.deepEqual([a.label, a.value, a.color, a.bar], ["5 h", "82 %", "amber", 0.826]);
  assert.deepEqual([b.value, b.color, b.bar], ["20 %", undefined, 0.2]);
  assert.deepEqual([c.label, c.color, c.kit], ["Čeká: 3", "red", { type: "drawer", card: "claude" }]);
  assert.match(e.label, /^Dnes 3[,.]46/);
  assert.equal(e.kit.type, "copy");
  assert.match(e.kit.text, /^3[,.]5$/);
  assert.equal(f.label, "Další: Stand-up za 30 min");
  assert.equal(sep.sep, true);
  assert.deepEqual([btn.label, btn.icon, btn.kit], ["Build", "▸", { type: "action", id: "build-ledge" }]);
});

test("resolve: neznámý zdroj / formát / syntaxe = řádek vynechán + poznámka, ostatní zůstanou", () => {
  const spec = { kit: 1, title: "x", rows: [
    { text: "{secret.notes}" },
    { bar: "nope.pct" },
    { text: "{clock.hm|zlato}" },
    { text: "{rozbité" },
    { label: "bez obsahu" },
    null,
    { text: "ok {procs.count}" },
  ] };
  const d = resolve(spec, snap(), NOW);
  assert.deepEqual(d.items.map((i) => i.label), ["ok 2"]);
  assert.equal(d.notes.length, 6);
  assert.match(d.notes[0], /secret\.notes/);
  assert.match(d.notes[2], /zlato/);
});

test("resolve: null = pomlčka, rozbitý snapshot ani spec nikdy nevyhodí", () => {
  const spec = { kit: 1, title: "t", rows: [{ bar: "limits.five.pct", label: "5 h", tone: { over: [[0, "red"]] } }, { text: "{costs.today|usd} {calendar.next.title}" }, { text: "{waiting.count} {procs.count}" }] };
  for (const bad of [null, undefined, {}, { limits: null, costs: { summary: null }, personal: null, procs: "x" }, { personal: { calendar: { days: [null] } } }]) {
    const d = resolve(spec, bad, NOW);
    assert.equal(d.items.length, 3, JSON.stringify(bad));
    assert.deepEqual([d.items[0].value, d.items[0].color, d.items[0].bar], ["—", undefined, undefined]);
  }
  assert.equal(resolve(spec, {}, NOW).items[1].label, "— —");
  assert.equal(resolve(spec, {}, NOW).items[2].label, "— —", "bez dat nevíme: ne 0");
  for (const badSpec of [null, {}, { rows: "x", buttons: 5 }, { rows: [1, "a", []], buttons: [null, {}] }]) {
    const d = resolve(badSpec, snap(), NOW);
    assert.ok(Array.isArray(d.items));
  }
});

test("resolve: stropy, text ≤ 120, url jen http(s), tlačítko bez akce se vynechá", () => {
  const rows = Array.from({ length: 20 }, (_, i) => ({ text: `r${i}` }));
  assert.equal(resolve({ kit: 1, title: "t", rows }, snap(), NOW).items.length, 12);
  const long = resolve({ kit: 1, title: "t", rows: [{ text: "x".repeat(300) }] }, snap(), NOW);
  assert.equal(long.items[0].label.length, 120);
  const d = resolve({ kit: 1, title: "t", rows: [{ text: "a", url: "javascript:alert(1)" }, { text: "b", url: "https://x.cz" }, { text: "c", open: "C:\\x" }], buttons: [{ label: "bez" }, { label: "ano", url: "http://x.cz" }, ...Array(8).fill({ label: "k", copy: "x" })] }, snap(), NOW);
  assert.equal(d.items[0].kit, undefined, "javascript: se nepřenese");
  assert.deepEqual(d.items[1].kit, { type: "url", url: "https://x.cz" });
  assert.deepEqual(d.items[2].kit, { type: "open", path: "C:\\x" });
  assert.equal(d.items.filter((i) => i.icon === "▸").length, 5, "strop 6 se bere před kontrolou, 1 neplatné");
  assert.match(d.notes[0], /Tlačítko 1/);
});

test("zdroje: hodnoty z drawer snapshotu", () => {
  const s = snap();
  const want = {
    "limits.five.pct": 82.6, "limits.five.resetMin": 90, "limits.week.pct": 20, "limits.week.resetMin": 4320,
    "waiting.count": 3, "procs.count": 2, "costs.today": 3.456, "costs.week": 120,
    "calendar.next.title": "Stand-up", "calendar.next.inMin": 30,
  };
  for (const [k, v] of Object.entries(want)) assert.equal(sourceValue(k, s, NOW), v, k);
  assert.match(sourceValue("clock.hm", s, NOW), /^\d\d:\d\d$/);
  assert.equal(typeof sourceValue("clock.date", s, NOW), "string");
  assert.equal(sourceValue("limits", s, NOW), undefined, "předek cesty není zdroj");
  assert.equal(sourceValue("constructor", s, NOW), undefined, "prototyp není zdroj");
  assert.equal(sourceValue("notes.0.title", s, NOW), undefined, "žádné get(path) nad snapshotem");
});

test("zdroje: reset v minulosti a chybějící rozsah útraty = null, ne 0", () => {
  assert.equal(sourceValue("limits.five.resetMin", { limits: { fiveHour: { pct: 1, resetsAt: NOW - 1 } } }, NOW), null);
  assert.equal(sourceValue("costs.today", { costs: { state: "ok", summary: { ranges: [{ days: 30, total: { cost: 9 } }] } } }, NOW), null, "30denní rozsah není dnešek");
  assert.equal(sourceValue("costs.today", { costs: { state: "ok", summary: { ranges: [{ days: 1, total: { cost: null } }] } } }, NOW), null);
  assert.equal(sourceValue("calendar.next.title", { personal: { calendar: { days: [] } } }, NOW), null);
});

test("plocha i šuplík ukážou totéž: každý zdroj ze SOURCES je i v tvaru deskSnapshot", () => {
  const drawer = snap();
  const desk = deskSnapshot(deskData());
  for (const key of Object.keys(SOURCES)) {
    const a = sourceValue(key, drawer, NOW);
    const b = sourceValue(key, desk, NOW);
    assert.notEqual(b, null, `${key}: na ploše chybí (deskSnapshot / DeskData)`);
    assert.deepEqual(b, a, `${key}: plocha a šuplík se liší`);
  }
  // prázdná plocha: nevíme = null (ne 0), kromě hodin
  const empty = deskSnapshot(null);
  for (const key of Object.keys(SOURCES)) {
    if (key.startsWith("clock.")) continue;
    assert.equal(sourceValue(key, empty, NOW), null, key);
  }
});

test("limity v kitu: jen čerstvý vzorek; starý, po resetu i bez dat = null (ne starý údaj)", () => {
  const w = (extra) => ({ limits: { at: NOW - MIN, staleMs: 600_000, fiveHour: { pct: 50, resetsAt: NOW + MIN * 60 }, sevenDay: { pct: 30, resetsAt: NOW + MIN * 600 }, ...extra } });
  assert.equal(sourceValue("limits.five.pct", w(), NOW), 50, "čerstvé");
  assert.equal(sourceValue("limits.five.pct", w({ at: NOW - 11 * MIN }), NOW), null, "starší než staleMs");
  assert.equal(sourceValue("limits.week.pct", w({ at: NOW - 11 * MIN }), NOW), null);
  assert.equal(sourceValue("limits.five.pct", w({ fiveHour: { pct: 50, resetsAt: NOW - 1 } }), NOW), null, "okno po resetu");
  assert.equal(sourceValue("limits.week.pct", w({ fiveHour: { pct: 50, resetsAt: NOW - 1 } }), NOW), 30, "týden dál čerstvý");
  assert.equal(sourceValue("limits.five.pct", w({ at: null }), NOW), null, "bez vzorku");
  assert.equal(sourceValue("limits.five.pct", { limits: null }, NOW), null, "bez dat");
});

test("útrata v kitu: jen při state ok", () => {
  const c = (state) => ({ costs: { state, summary: { ranges: [{ days: 1, total: { cost: 2 } }] } } });
  assert.equal(sourceValue("costs.today", c("ok"), NOW), 2);
  for (const st of ["error", "computing", "off", "idle"]) assert.equal(sourceValue("costs.today", c(st), NOW), null, st);
});

test("plocha: procs bez dat = „—“ (ne 0), s daty počet; waiting.count je skutečný počet, ne ořezaný seznam", () => {
  assert.equal(sourceValue("procs.count", deskSnapshot(null), NOW), null);
  assert.equal(sourceValue("procs.count", deskSnapshot({ now: NOW, procs: [] }), NOW), 0);
  assert.equal(sourceValue("procs.count", deskSnapshot({ now: NOW, procs: [{ phase: "ready" }] }), NOW), 1);
  const rows = Array.from({ length: 6 }, (_, i) => ({ id: i, kind: "approve" }));
  assert.equal(sourceValue("waiting.count", deskSnapshot({ now: NOW, waiting: rows, waitingTotal: 9 }), NOW), 9);
  assert.equal(sourceValue("waiting.count", deskSnapshot({ now: NOW, waiting: rows }), NOW), 6, "starší data bez waitingTotal");
  assert.equal(sourceValue("waiting.count", deskSnapshot({ now: NOW }), NOW), null);
});
