//   node --experimental-strip-types --test src/cards/weatherModel.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { wxKind, minutesOf, clockOf, sunPos, hoursFrom, rangePos, curvePoints, nearestIndex, windIndex, dowOf, dayRows, skyId, tempColor } from "./weatherModel.ts";
import { WEATHER_DESIGNS, weatherDesign } from "./weatherDesigns.ts";

test("wxKind: tabulka kódů, neznámé = null", () => {
  for (const [c, k] of [[0, "clear"], [1, "clear"], [2, "partly"], [3, "cloud"], [45, "fog"], [53, "drizzle"], [63, "rain"], [81, "rain"], [73, "snow"], [86, "snow"], [95, "storm"], [99, "storm"], [1234, null], [null, null]]) assert.equal(wxKind(c), k, String(c));
});

test("čas: minuty, hodiny, nesmysl", () => {
  assert.equal(minutesOf("2026-10-16T07:05"), 425);
  assert.equal(minutesOf("x"), null);
  assert.equal(clockOf("2026-10-16T07:05"), "7:05");
  assert.equal(clockOf(null), "—");
});

test("sunPos: před, přes den, po, jiný den, chybí", () => {
  const [r, s] = ["2026-10-16T06:00", "2026-10-16T18:00"];
  assert.deepEqual(sunPos(r, s, "2026-10-16T05:00"), { phase: "before", p: 0 });
  assert.deepEqual(sunPos(r, s, "2026-10-16T12:00"), { phase: "day", p: 0.5 });
  assert.deepEqual(sunPos(r, s, "2026-10-16T19:00"), { phase: "after", p: 1 });
  assert.equal(sunPos(r, s, "2026-10-17T12:00"), null);
  assert.equal(sunPos(null, s, "2026-10-16T12:00"), null);
  assert.equal(sunPos(s, r, "2026-10-16T12:00"), null);
});

test("hoursFrom: od aktuální hodiny, bez hodin bez teploty, limit", () => {
  const h = (time, temp) => ({ time, temp, precipProb: null, code: null });
  const all = [h("2026-10-16T05:00", 1), h("2026-10-16T06:00", 2), h("2026-10-16T07:00", null), h("2026-10-16T08:00", 4)];
  assert.deepEqual(hoursFrom(all, "2026-10-16T06:15").map((x) => x.time), ["2026-10-16T06:00", "2026-10-16T08:00"]);
  assert.equal(hoursFrom(all, "2026-10-16T06:15", 1).length, 1);
  assert.deepEqual(hoursFrom(undefined, "2026-10-16T06:15"), []);
});

test("rangePos, curvePoints, nearestIndex, windIndex, tempColor", () => {
  assert.equal(rangePos(5, 0, 10), 0.5);
  assert.equal(rangePos(-5, 0, 10), 0);
  assert.equal(rangePos(5, 3, 3), 0.5);
  assert.equal(curvePoints([1]), null);
  assert.deepEqual(curvePoints([0, 10, 5]), [{ x: 0, y: 1 }, { x: 0.5, y: 0 }, { x: 1, y: 0.5 }]);
  assert.deepEqual(curvePoints([4, 4]).map((p) => p.y), [0.5, 0.5]);
  assert.equal(nearestIndex(0.5, 5), 2);
  assert.equal(nearestIndex(2, 5), 4);
  assert.equal(nearestIndex(-1, 5), 0);
  assert.equal(nearestIndex(0.3, 0), 0);
  for (const [d, i] of [[0, 0], [359, 0], [90, 2], [225, 5], [-45, 7]]) assert.equal(windIndex(d), i, String(d));
  assert.match(tempColor(-30), /^hsl\(215 /);
  assert.match(tempColor(10), /^hsl\(130 /);
  assert.match(tempColor(20), /^hsl\(55 /);
  assert.match(tempColor(99), /^hsl\(0 /);
});

test("dowOf a dayRows", () => {
  assert.equal(dowOf("2026-10-29"), 4);
  assert.equal(dowOf("bad"), null);
  const w = { code: 3, observed: "2026-10-16T07:00", min: 1, max: 9, days: [{ date: "2026-10-17", min: 2, max: null, code: 1 }, { date: "2026-10-18", min: 3, max: 8, code: 61 }] };
  assert.deepEqual(dayRows(w).map((r) => [r.date, r.today]), [["2026-10-16", true], ["2026-10-18", false]]);
  assert.deepEqual(dayRows({ ...w, min: null, days: undefined }), []);
});

test("skyId a registr vzhledů", () => {
  assert.equal(skyId(63, false), "rain-night");
  assert.equal(skyId(1234, true), "clear-day");
  assert.equal(WEATHER_DESIGNS[0].id, "classic");
  assert.equal(new Set(WEATHER_DESIGNS.map((d) => d.id)).size, WEATHER_DESIGNS.length);
  for (const d of WEATHER_DESIGNS) assert.ok(d.name[0].startsWith("cards.design.weather.") && d.line[0].endsWith(".line") && d.name[1] && d.line[1]);
  assert.equal(weatherDesign(undefined), "classic");
  assert.equal(weatherDesign("nope"), "classic");
  assert.equal(weatherDesign("arc"), "arc");
});
