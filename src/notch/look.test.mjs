// Kresba sekcí lišty notche (`look.ts`) bez DOM.
//   node --experimental-strip-types --test src/notch/look.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { HEAT_FULL, HEAT_WARN, heatHex, chartPaths, eqLit, handAngles, htopBar, knobAngle, lastBars, loadColor, nextMeeting, nudge, paintOf, smoothPath, weatherNow, weightPx } from "./look.ts";

test("barva: akcent, zátěž, přechod, hex", () => {
  const cases = [
    [undefined, 50, { kind: "solid", css: "var(--accent)" }],
    ["accent", 99, { kind: "solid", css: "var(--accent)" }],
    ["load", 20, { kind: "solid", css: "var(--green)" }],
    ["load", 80, { kind: "solid", css: "var(--amber)" }],
    ["load", 74, { kind: "solid", css: "var(--green)" }],
    ["load", 90, { kind: "solid", css: "var(--red)" }],
    ["load", 95, { kind: "solid", css: "var(--red)" }],
    ["gradient", 10, { kind: "gradient" }],
    ["#112233", 10, { kind: "solid", css: "#112233" }],
    ["#11223", 10, { kind: "solid", css: "var(--accent)" }],
  ];
  for (const [c, pct, want] of cases) assert.deepEqual(paintOf(c, pct), want, `${c} ${pct}`);
  assert.equal(loadColor(null), "var(--green)");
  assert.deepEqual([weightPx("thin"), weightPx(undefined), weightPx("bold")], [1, 1.6, 2.6]);
});

test("graf: bez dvou vzorků nic, vyhlazení zůstane v okně", () => {
  assert.equal(chartPaths([], 100, 20, false, 100), null);
  assert.equal(chartPaths([5], 100, 20, true, 100), null);
  const p = chartPaths([0, 100, 0, 100], 100, 20, false, 100);
  assert.match(p.line, /^M[\d.]+ [\d.]+ L/);
  assert.match(p.area, / Z$/);
  const s = smoothPath([{ x: 0, y: 20 }, { x: 10, y: 0 }, { x: 20, y: 20 }, { x: 30, y: 0 }], 20);
  assert.match(s, / C/);
  for (const n of s.match(/-?[\d.]+/g).map(Number)) assert.ok(n >= 0 && n <= 30, s);
  assert.deepEqual(lastBars([10, null, 50, 200], 2), [0.5, 1]);
});

test("ručičky, kolečko, sloupky, htop, krok", () => {
  assert.deepEqual(handAngles(new Date(2026, 0, 5, 15, 30, 0)), { h: 105, m: 180, s: 0 });
  assert.deepEqual(handAngles(new Date(2026, 0, 5, 0, 0, 15)), { h: 0.13, m: 1.5, s: 90 });
  assert.deepEqual([knobAngle(0), knobAngle(50), knobAngle(100), knobAngle(150)], [-135, 0, 135, 135]);
  assert.deepEqual([eqLit(0, 8), eqLit(50, 8), eqLit(100, 8), eqLit(100, 8, true)], [0, 4, 8, 0]);
  assert.equal(htopBar(50, 10), "[|||||     ]");
  assert.equal(htopBar(120, 4), "[||||]");
  assert.deepEqual([nudge(99, 1), nudge(1, -1), nudge(40, 1)], [100, 0, 42]);
});

test("příští schůzka: jen s časem, neskončená; bez kalendáře null", () => {
  const now = new Date(2026, 3, 10, 12, 0).getTime();
  const it = (title, sh, eh, allDay = false) => ({ title, start: new Date(2026, 3, 10, sh).getTime(), end: new Date(2026, 3, 10, eh).getTime(), allDay, ongoing: false, location: "", calendar: 0 });
  const cal = { status: "ok", calendars: [], days: [{ kind: "today", date: "2026-04-10", items: [it("Ráno", 8, 9), it("Celý den", 0, 23, true), it("Porada", 14, 15)] }] };
  assert.equal(nextMeeting(cal, now)?.title, "Porada");
  assert.equal(nextMeeting(null, now), null);
  assert.equal(nextMeeting({ ...cal, days: [] }, now), null);
});

test("počasí: jen platná data, poloha na pruhu dne", () => {
  const w = (status, data) => ({ status, label: "app", data });
  // věk: bez `now` se nehlídá; s `now` ≥ 3 h nebo bez času stažení = nic
  const d0 = { temp: 5, min: 0, max: 10, desc: "", icon: "" };
  assert.equal(weatherNow({ status: "ok", label: "", data: d0, fetchedMs: 0 }, 3 * 3_600_000), null);
  assert.equal(weatherNow({ status: "ok", label: "", data: d0, fetchedMs: 1 }, 3 * 3_600_000 - 1)?.temp, 5);
  assert.equal(weatherNow({ status: "ok", label: "", data: d0 }, 1000), null);
  assert.equal(weatherNow(null), null);
  assert.equal(weatherNow(w("unset", null)), null);
  assert.equal(weatherNow(w("unavailable", { temp: 5, min: 0, max: 10, desc: "", icon: "" })), null);
  assert.equal(weatherNow(w("ok", { temp: 5, min: 0, max: 10, desc: "", icon: "" })).pos, 0.5);
  assert.equal(weatherNow(w("stale", { temp: 5, min: null, max: 10, desc: "", icon: "" })).pos, null);
});

test("tečky pod dnem: počet událostí, strop, bez kalendáře 0", async () => {
  const { dotsOf } = await import("./look.ts");
  const cal = { status: "ok", calendars: [], days: [{ kind: "today", date: "2026-04-10", items: [1, 2, 3, 4, 5] }, { kind: "tomorrow", date: "2026-04-11", items: [1] }] };
  assert.deepEqual([dotsOf(cal, "2026-04-10"), dotsOf(cal, "2026-04-11"), dotsOf(cal, "2026-04-12"), dotsOf(null, "2026-04-10")], [3, 1, 0, 0]);
});

test("heat: prahy = Rust capsule.rs (jedna pravda) a pevné body spojité barvy", () => {
  const rs = readFileSync(new URL("../../src-tauri/src/looks/capsule.rs", import.meta.url), "utf8");
  assert.equal(Number(/pub const WARN_PCT: f64 = ([\d.]+);/.exec(rs)[1]), HEAT_WARN);
  assert.equal(Number(/pub const FULL_PCT: f64 = ([\d.]+);/.exec(rs)[1]), HEAT_FULL);
  // loadOf / level čtou tutéž tabulku: `charts/thresholds.test.mjs`
  // kotvy: 0 zelená (#4ade80), 75 jantar (#fabf24), od 90 červená (#f77070); střed zelená→jantar; nesmysl = zelená
  for (const [p, want] of [[0, "#4ade80"], [null, "#4ade80"], [NaN, "#4ade80"], [-3, "#4ade80"], [75, "#fabf24"], [90, "#f77070"], [100, "#f77070"], [250, "#f77070"], [37.5, "#a2cf52"]])
    assert.equal(heatHex(p), want, String(p));
  assert.ok(parseInt(heatHex(60).slice(1, 3), 16) < parseInt(heatHex(70).slice(1, 3), 16), "červená složka roste");
});
