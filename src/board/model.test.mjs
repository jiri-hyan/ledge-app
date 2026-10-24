// Tabulkové testy nových widgetů kola 22 (`model.ts`): bez DOM, bez sítě, bez skutečných dat uživatele.
//   node --experimental-strip-types --test src/board/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { barHeights, calState, heat, hourRows, hourSpan, hoursMax, isToday, lastDays, localDate, stepVolume, sumDays, todayLeft, usageState, weekStrip } from "./model.ts";

// 2026-10-21 (středa) 14:30 místního času
const NOW = new Date(2026, 9, 21, 14, 30).getTime();
const day = (date, tokens, cost) => ({ date, tokens, cost });

test("localDate: místní datum", () => {
  assert.equal(localDate(NOW), "2026-10-21");
  assert.equal(localDate(new Date(2026, 0, 5, 0, 0).getTime()), "2026-01-05");
});

test("útrata: posledních n dní, součet, cena jen s cenou", () => {
  const days = [day("2026-10-14", 5, null), day("2026-10-15", 10, 0.5), day("2026-10-21", 20, 1.25)];
  assert.deepEqual(lastDays(days, 2).map((d) => d.date), ["2026-10-15", "2026-10-21"]);
  assert.deepEqual(lastDays(undefined, 7), []);
  const cases = [
    [days, { tokens: 35, cost: 1.75 }],
    [[day("a", 3, null), day("b", 4, null)], { tokens: 7, cost: null }],
    [[], { tokens: 0, cost: null }],
  ];
  for (const [input, want] of cases) assert.deepEqual(sumDays(input), want);
});

test("sloupce: podle ceny, bez ceny podle tokenů, prázdné = nuly", () => {
  assert.deepEqual(barHeights([day("a", 1, 1), day("b", 9, 2)]), [0.5, 1]);
  assert.deepEqual(barHeights([day("a", 1, null), day("b", 4, null)]), [0.25, 1]);
  assert.deepEqual(barHeights([day("a", 0, null), day("b", 0, 0)]), [0, 0]);
});

test("stav karty Claude dnes", () => {
  const cases = [
    [null, "off"],
    [{ state: "off", summary: null }, "off"],
    [{ state: "computing", summary: null }, "first"],
    [{ state: "idle", summary: null }, "first"],
    [{ state: "error", summary: null }, "error"],
    [{ state: "computing", summary: { ranges: [], days: [] } }, "ok"],
    [{ state: "error", summary: { ranges: [], days: [] } }, "ok"],
  ];
  for (const [v, want] of cases) assert.equal(usageState(v), want, JSON.stringify(v));
});

test("časová osa: řádky, dnešek, úrovně a rozsah hodin", () => {
  const t = (at) => Array.from({ length: 24 }, (_, i) => at[i] ?? 0);
  const v = { state: "ok", summary: { ranges: [], days: [day("2026-10-21", 9, 1)], hours: [{ name: "app", tokens: t({ 9: 100, 13: 25 }), requests: 3, other: false }, { name: "zlé", tokens: [1, 2], requests: 1, other: false }] } };
  const rows = hourRows(v);
  assert.equal(rows.length, 1, "řádek s jiným počtem hodin se zahodí");
  assert.deepEqual(hourRows({ state: "ok", summary: { ranges: [], days: [] } }), [], "starý snapshot bez hodin");
  assert.equal(hoursMax(rows), 100);
  assert.equal(hoursMax([]), 0);
  assert.ok(isToday(v, NOW));
  assert.ok(!isToday({ summary: { days: [day("2026-10-20", 1, 1)] } }, NOW), "přehled ze včerejška");
  assert.ok(!isToday(null, NOW));
  const levels = [
    [0, 100, 0],
    [1, 100, 1],
    [33, 100, 1],
    [34, 100, 2],
    [67, 100, 3],
    [100, 100, 4],
    [5, 0, 0],
  ];
  for (const [x, max, want] of levels) assert.equal(heat(x, max), want, `${x}/${max}`);
  assert.deepEqual(hourSpan(rows, 14), [8, 14], "od 8 (nebo první aktivní) do teď");
  assert.deepEqual(hourSpan([{ tokens: t({ 6: 1, 22: 1 }) }], 14), [6, 22]);
  assert.deepEqual(hourSpan([], 3), [3, 3], "bez řádků jen teď (karta osu neukáže)");
});

test("týden: od pondělí, dnešek, víkend, dny s událostí", () => {
  const cal = { status: "ok", more: 0, calendars: [], loading: false, days: [{ kind: "today", date: "2026-10-21", items: [{ start: 1, end: 2 }] }, { kind: "tomorrow", date: "2026-10-22", items: [] }] };
  const w = weekStrip(NOW, cal);
  assert.deepEqual(w.map((d) => d.day), [19, 20, 21, 22, 23, 24, 25]);
  assert.deepEqual(w.map((d) => d.today), [false, false, true, false, false, false, false]);
  assert.deepEqual(w.map((d) => d.weekend), [false, false, false, false, false, true, true]);
  assert.deepEqual(w.map((d) => d.busy), [false, false, true, false, false, false, false], "zítřek bez událostí není tečkovaný");
  // neděle patří ke končícímu týdnu, přelom měsíce
  const sun = weekStrip(new Date(2026, 10, 1, 9).getTime());
  assert.deepEqual(sun.map((d) => d.day), [26, 27, 28, 29, 30, 31, 1]);
  assert.ok(sun[6].today && sun[6].weekend);
});

test("kalendář: stav a zbytek dneška", () => {
  const cases = [
    [undefined, "none"],
    [{ status: "off", days: [] }, "none"],
    [{ status: "unset", days: [] }, "none"],
    [{ status: "unavailable", days: [] }, "unavailable"],
    [{ status: "loading", days: [] }, "loading"],
    [{ status: "idle", days: [{ kind: "today", items: [] }] }, "ok"],
    [{ status: "stale", days: [] }, "ok"],
    [{ status: "ok", days: [] }, "ok"],
  ];
  for (const [c, want] of cases) assert.equal(calState(c), want, JSON.stringify(c));
  const it = (title, start, end, allDay = false) => ({ title, start, end, allDay });
  const h = 3_600_000;
  const cal = { status: "ok", days: [{ kind: "today", items: [it("Ráno", NOW - 5 * h, NOW - 4 * h), it("Teď", NOW - h, NOW + h), it("Svátek", 0, 1, true), it("Večer", NOW + 4 * h, NOW + 5 * h), it("Pozdě", NOW + 6 * h, NOW + 7 * h)] }] };
  assert.deepEqual(todayLeft(cal, NOW).map((x) => x.title), ["Svátek", "Teď", "Večer"]);
  assert.deepEqual(todayLeft(cal, NOW, 5).map((x) => x.title), ["Svátek", "Teď", "Večer", "Pozdě"]);
  assert.deepEqual(todayLeft({ status: "ok", days: [] }, NOW), []);
  assert.deepEqual(todayLeft(null, NOW), []);
});

test("hlasitost: krok a oříznutí", () => {
  const cases = [
    [50, 5, 55],
    [98, 5, 100],
    [3, -5, 0],
    [NaN, 5, 5],
    [42.4, 0, 42],
  ];
  for (const [l, d, want] of cases) assert.equal(stepVolume(l, d), want, `${l}+${d}`);
});
