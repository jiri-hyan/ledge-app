// Tabulkové testy vzhledů karty Týden (`weekModel.ts`, `weekDesigns.ts`): bez DOM, bez sítě, jen smyšlená data.
//   node --experimental-strip-types --test src/board/weekModel.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { allDayToday, countdown, dayMinutes, dialAngle, monthGrid, nextEvents, nowPos, polar, remainingToday, timedToday, timelineBlocks, wedgePath } from "./weekModel.ts";
import { WEEK_DESIGNS, weekDesign } from "./weekDesigns.ts";

// 2026-10-21 (středa) 14:30 místního času
const at = (h, m = 0, d = 21) => new Date(2026, 9, d, h, m).getTime();
const NOW = at(14, 30);
const ev = (title, s, e, extra = {}) => ({ start: s, end: e, allDay: false, title, location: "", cal: 0, ongoing: false, tzGuess: false, ...extra });
const cal = (today = [], later = []) => ({
  status: "ok",
  more: 0,
  calendars: [],
  loading: false,
  days: [
    { kind: "today", date: "2026-10-21", items: today },
    { kind: "later", date: "2026-10-23", items: later },
  ],
});

test("dayMinutes a nowPos: osa 7–22", () => {
  assert.equal(dayMinutes(NOW), 870);
  assert.equal(nowPos(at(7), 7, 22), 0);
  assert.equal(nowPos(at(14, 30), 7, 22), 50);
  assert.equal(nowPos(at(22, 30), 7, 22), null);
  assert.equal(nowPos(at(6), 7, 22), null);
});

test("timelineBlocks: ořez na osu, mimo osu se počítá", () => {
  const items = [ev("a", at(9), at(10)), ev("b", at(6), at(8)), ev("c", at(23), at(23, 30)), ev("d", at(21), at(23, 30)), ev("e", at(5), at(6))];
  const r = timelineBlocks(items, 7, 22);
  assert.equal(r.outside, 2);
  assert.deepEqual(r.blocks.map((b) => [b.item.title, Math.round(b.left), Math.round(b.width)]), [["b", 0, 7], ["a", 13, 7], ["d", 93, 7]]);
  assert.deepEqual(timelineBlocks([]), { blocks: [], outside: 0, lanes: 1 });
  // překryv jde do druhého řádku, navazující událost zpátky do prvního
  const o = timelineBlocks([ev("a", at(9), at(11)), ev("b", at(10), at(12)), ev("c", at(11), at(12, 30))]);
  assert.deepEqual(o.blocks.map((b) => [b.item.title, b.lane]), [["a", 0], ["b", 1], ["c", 0]]);
  assert.equal(o.lanes, 2);
});

test("dnešní události: s časem, celodenní, zbývající", () => {
  const c = cal([ev("ráno", at(8), at(9)), ev("odpoledne", at(16), at(17)), ev("celý", at(0), at(23, 59), { allDay: true })]);
  assert.deepEqual(timedToday(c).map((x) => x.title), ["ráno", "odpoledne"]);
  assert.deepEqual(allDayToday(c).map((x) => x.title), ["celý"]);
  assert.equal(remainingToday(c, NOW), 2);
  assert.equal(remainingToday(null, NOW), 0);
});

test("countdown: teď, minuty, hodiny, dny", () => {
  assert.deepEqual(countdown(NOW - 1000, NOW), { kind: "now" });
  assert.deepEqual(countdown(NOW + 25 * 60_000, NOW), { kind: "min", n: 25 });
  assert.deepEqual(countdown(NOW + 61 * 60_000, NOW), { kind: "hour", n: 1 });
  assert.deepEqual(countdown(NOW + 3 * 86_400_000, NOW), { kind: "day", n: 3 });
});

test("nextEvents: probíhající první, skončené pryč, limit", () => {
  const c = cal([ev("hotovo", at(8), at(9)), ev("běží", at(14), at(15), { ongoing: true }), ev("brzy", at(16), at(17))], [ev("později", at(10, 0, 23), at(11, 0, 23))]);
  assert.deepEqual(nextEvents(c, NOW).map((x) => x.title), ["běží", "brzy", "později"]);
  assert.equal(nextEvents(c, NOW, 2).length, 2);
  assert.deepEqual(nextEvents(null, NOW), []);
});

test("monthGrid: celé týdny od pondělí, dnešek a tečky", () => {
  const g = monthGrid(NOW, cal([ev("x", at(9), at(10))]));
  assert.ok(g.length >= 4 && g.length <= 6 && g.every((r) => r.length === 7));
  assert.equal(g[0][0].date, "2026-09-28"); // říjen 2026 začíná ve čtvrtek
  assert.equal(g[0][0].inMonth, false);
  const flat = g.flat();
  assert.equal(flat.filter((c) => c.today).length, 1);
  assert.equal(flat.find((c) => c.today).day, 21);
  assert.deepEqual(flat.filter((c) => c.busy).map((c) => c.date), ["2026-10-21", "2026-10-23"].filter((d) => d === "2026-10-21"));
  assert.equal(flat.filter((c) => c.inMonth).length, 31);
});

test("ciferník: úhly a cesta výseče", () => {
  assert.equal(dialAngle(0), 0);
  assert.equal(dialAngle(720), 180);
  const p = polar(50, 50, 10, 90);
  assert.ok(Math.abs(p.x - 60) < 1e-9 && Math.abs(p.y - 50) < 1e-9);
  assert.match(wedgePath(50, 50, 30, 40, 0, 90), /^M.*A40 40 0 0 1 .*L.*A30 30 0 0 0 .*Z$/);
  assert.match(wedgePath(50, 50, 30, 40, 0, 270), / 0 1 1 /);
});

test("registr vzhledů karty Týden: první výchozí, id jedinečná, neznámé = výchozí", () => {
  assert.equal(WEEK_DESIGNS[0].id, "strip");
  assert.equal(new Set(WEEK_DESIGNS.map((d) => d.id)).size, WEEK_DESIGNS.length);
  for (const d of WEEK_DESIGNS) assert.ok(d.name[0].startsWith("cards.design.week.") && d.line[0] === `${d.name[0]}.line` && d.name[1] && d.line[1]);
  assert.equal(weekDesign(undefined), "strip");
  assert.equal(weekDesign("x"), "strip");
  assert.equal(weekDesign("dial"), "dial");
});
