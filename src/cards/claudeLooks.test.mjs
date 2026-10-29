// Vzhledy karty Claude (`claudeLooks.ts`): okno limitu, tempomat, útrata po dnech, dlaždice sezení.
import test from "node:test";
import assert from "node:assert/strict";
import { paceLine, sessionTiles, spendModel, winInfo } from "./claudeLooks.ts";
import { localDate } from "../board/model.ts";

const NOW = 1_800_000_000_000;
const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;
const lv = (pct, resetIn = 2 * HOUR, extra = {}) => ({ at: NOW - MIN, staleMs: 15 * MIN, fiveHour: pct == null ? null : { pct, resetsAt: NOW + resetIn }, sevenDay: null, ...extra });

test("winInfo: čerstvé, staré, po resetu, bez dat", () => {
  assert.deepEqual(winInfo({ pct: 42.7, resetsAt: NOW + HOUR }, lv(1), NOW), { kind: "fresh", pct: 42.7, tone: "ok", resetsAt: NOW + HOUR, ageMin: 0 });
  assert.equal(winInfo({ pct: 75, resetsAt: NOW + HOUR }, lv(1), NOW).tone, "warn");
  assert.equal(winInfo({ pct: 95, resetsAt: NOW + HOUR }, lv(1), NOW).tone, "bad");
  const old = winInfo({ pct: 10, resetsAt: NOW + HOUR }, { at: NOW - 40 * MIN, staleMs: 15 * MIN }, NOW);
  assert.deepEqual([old.kind, old.pct, old.ageMin], ["stale", 10, 40]);
  assert.equal(winInfo({ pct: 99, resetsAt: NOW - 1 }, lv(1), NOW).pct, null, "po resetu žádné procento");
  assert.equal(winInfo(null, lv(1), NOW).kind, "none");
  assert.equal(winInfo({ pct: 5, resetsAt: null }, null, NOW).kind, "none");
});

test("paceLine: průměr okna, tempo z historie, bez dat", () => {
  // okno končí za 2 h → uplynuly 3 h (plán 60 %); 60 % za 3 h = 20 %/h → 40 % zbývá za 2 h, tedy přesně do obnovy
  const avg = paceLine(lv(60), NOW);
  assert.equal(Math.round(avg.plan), 60);
  assert.ok(avg.kind === "runs-out" || avg.kind === "lasts");
  const hist = [[NOW - 30 * MIN, 50], [NOW, 70]];
  const fast = paceLine(lv(70, 3 * HOUR, { history5h: hist }), NOW);
  assert.equal(fast.kind, "runs-out");
  assert.equal(Math.round(fast.perH), 40);
  assert.equal(Math.round((fast.at - NOW) / MIN), 45, "30 % při 40 %/h");
  const slow = paceLine(lv(10, 1 * HOUR, { history5h: [[NOW - 30 * MIN, 9.9], [NOW, 10]] }), NOW);
  assert.equal(slow.kind, "lasts");
  const idle = paceLine(lv(10, 1 * HOUR, { history5h: [[NOW - 30 * MIN, 10], [NOW, 10]] }), NOW);
  assert.equal(idle.kind, "idle");
  assert.equal(paceLine(lv(null), NOW), null);
  assert.equal(paceLine(null, NOW), null);
  assert.equal(paceLine(lv(60, -MIN), NOW), null, "po resetu");
  assert.equal(paceLine({ ...lv(60), at: NOW - 40 * MIN }, NOW), null, "staré vzorky tempo nepočítají");
});

const days = (costs, endAgo = 0) => costs.map((cost, i) => ({ date: localDate(NOW - (endAgo + costs.length - 1 - i) * DAY), tokens: 100 * (i + 1), cost }));

test("spendModel: stavy a dnešek", () => {
  assert.equal(spendModel(undefined, NOW).state, "off");
  assert.equal(spendModel({ state: "off", summary: null }, NOW).state, "off");
  assert.equal(spendModel({ state: "computing", summary: null }, NOW).state, "computing");
  assert.equal(spendModel({ state: "ok", summary: null }, NOW).state, "none");
  assert.equal(spendModel({ state: "ok", summary: { days: [] } }, NOW).state, "none");
  const m = spendModel({ state: "ok", summary: { days: days([1, 3, 2]) } }, NOW);
  assert.deepEqual([m.state, m.priced, m.today, m.values, m.peak, m.avg], ["ok", true, 2, [1, 3, 2], 3, 2]);
  assert.equal(m.dates.length, 3);
  const old = spendModel({ state: "ok", summary: { days: days([1, 3, 2], 1) } }, NOW);
  assert.equal(old.today, null, "poslední den je včerejšek — o dnešku nic");
  const tok = spendModel({ state: "ok", summary: { days: days([null, null]) } }, NOW);
  assert.deepEqual([tok.priced, tok.values, tok.today], [false, [100, 200], 200], "bez cen tokeny");
  const many = spendModel({ state: "ok", summary: { days: days(Array.from({ length: 30 }, (_, i) => i)) } }, NOW);
  assert.equal(many.values.length, 14);
  assert.equal(many.dates.length, 14);
});

test("sessionTiles: pořadí, strop, počet zbytku", () => {
  const s = (id, status, updatedAt) => ({ id, status, updatedAt });
  const list = [s("a", "idle", 5), s("b", "working", 1), s("c", "ended", 9), s("d", "working", 3), s("e", "idle", 7)];
  const r = sessionTiles(list, 3);
  assert.deepEqual(r.shown.map((x) => x.id), ["d", "b", "e"]);
  assert.deepEqual([r.more, r.working], [2, 2]);
  assert.deepEqual(sessionTiles([], 3), { shown: [], more: 0, working: 0 });
  assert.equal(sessionTiles(list, 0).more, 5);
});
