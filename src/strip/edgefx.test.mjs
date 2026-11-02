// Živá hrana (`edgefx.ts`, kolo 28): ořez jako Rust `Deco::norm`, starý config beze změny, stav → efekt. Bez DOM.
//   node --experimental-strip-types --test src/strip/edgefx.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_DECO, DEFAULT_LIVE, EDGE_SETS, SET_COLORS, edgeColor, liveEdge, normDeco, spanInset, stateTone } from "./edgefx.ts";

test("starý config (kolo 25) = stejný vzhled, nové klíče výchozí", () => {
  const d = normDeco({ edge: "glow", len: "center", color: "claude", width: 1 });
  assert.deepEqual(d, { ...DEFAULT_DECO, edge: "glow", color: "claude", width: 1, live: { ...DEFAULT_LIVE } });
  assert.equal(liveEdge(d, "work"), "glow");
  assert.equal(spanInset(d), 30);
  assert.deepEqual(normDeco(undefined), DEFAULT_DECO);
});

test("ořez rozsahů a barev jako Rust", () => {
  const n = normDeco({ pct: 0, speed: 9, power: 0, color: "Aurora", color3: "#ABCDEF", live: { on: true, work: "duha" } });
  assert.deepEqual([n.pct, n.speed, n.power, n.color, n.color3, n.live.on, n.live.work], [10, 5, 1, "aurora", "#abcdef", true, "flow"]);
  // (vstup, čekaná barva)
  for (const [raw, want] of [["accent", "accent"], ["Claude", "claude"], ["#FF8800", "#ff8800"], ["duha", "accent"], ["embers", "embers"]]) assert.equal(edgeColor(raw), want, raw);
  for (const id of EDGE_SETS) assert.equal(SET_COLORS[id].length, 3, id);
});

test("stav → efekt a barva (jako Rust edgefx::apply)", () => {
  const d = normDeco({ edge: "underline", live: { on: true } });
  // (stav, efekt, tón)
  for (const [s, e, tone] of [["idle", "underline", null], ["work", "flow", null], ["wait", "breath", "wait"], ["done", "sweep", "ok"], ["limit", "dashes", "wait"]]) {
    assert.equal(liveEdge(d, s), e, s);
    assert.equal(stateTone(d, s), tone, s);
  }
  // vypnutý stav = stálá hrana; vypnutá živá hrana jen s „Vyzkoušet“
  const off = normDeco({ edge: "underline", live: { on: true, wait: "none" } });
  assert.deepEqual([liveEdge(off, "wait"), stateTone(off, "wait")], ["underline", null]);
  const dead = normDeco({ edge: "glow" });
  assert.deepEqual([liveEdge(dead, "done"), liveEdge(dead, "done", true)], ["glow", "sweep"]);
  // délky: střed 30 %, široká 10 %, celá 8 %, vlastní (100 − %) / 2
  for (const [len, pct, want] of [["center", 40, 30], ["wide", 40, 10], ["full", 40, 8], ["custom", 60, 20], ["custom", 100, 8]]) assert.equal(spanInset(normDeco({ len, pct })), want, `${len} ${pct}`);
});
