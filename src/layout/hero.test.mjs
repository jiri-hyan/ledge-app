// @vitest-environment node
// „Číslo jako hrdina“ (`hero.ts`): jen skutečná čísla, poctivé null (karta pak vypadá jako dřív), čekající
// schválení má přednost, sparkline útraty z dnů.
//   node --experimental-strip-types --test src/layout/hero.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { costSeries, heroFacts } from "./hero.ts";
import { localDate } from "../board/model.ts";

const NOW = 1_800_000_000_000;
const MIN = 60_000;
const limits = (pct, atAgo = MIN, resetIn = 130 * MIN) => ({ at: NOW - atAgo, staleMs: 15 * MIN, fiveHour: { pct, resetsAt: resetIn == null ? null : NOW + resetIn } });
const DAY = 86_400_000;
// poslední den = dnešek (místní datum z NOW), starší dny před ním; `endAgo` posune celý přehled do minulosti
const days = (...c) => daysEnding(0, ...c);
const daysEnding = (endAgo, ...c) => c.map((cost, i) => ({ date: localDate(NOW - (endAgo + c.length - 1 - i) * DAY), tokens: 1000 * (i + 1), cost }));

test("bez dat = null (karta se kreslí jako dřív)", () => {
  assert.equal(heroFacts("claude", {}, NOW), null);
  assert.equal(heroFacts("claude", { limits: { at: null, staleMs: 1, fiveHour: null }, costs: { summary: null } }, NOW), null);
  assert.equal(heroFacts("weather", { personal: { weather: { status: "unset", label: "", data: null } } }, NOW), null);
  assert.equal(heroFacts("weather", { personal: { weather: { status: "unavailable", label: "x", data: { temp: 3, min: null, max: null } } } }, NOW), null);
  assert.equal(heroFacts("projects", { limits: limits(40) }, NOW), null, "karta bez hlavního čísla");
});

test("Claude: limit 5 h, reset, sparkline útraty", () => {
  const f = heroFacts("claude", { limits: limits(42.7), costs: { summary: { days: days(0.5, 1.25, 2) } } }, NOW);
  assert.equal(f.value, 42, "dolů — 42,7 % není 43");
  assert.equal(f.unit, "%");
  assert.deepEqual(f.label, ["cards.hero.limit5h", "Limit Claude 5 h"]);
  assert.deepEqual(f.sub, { text: ["cards.hero.reset", "reset za {h} h {m} min"], vars: { h: 2, m: 10 } });
  assert.deepEqual(f.series, [0.5, 1.25, 2]);
  // starý vzorek: číslo zůstane, druhý řádek řekne stáří
  const old = heroFacts("claude", { limits: limits(80, 40 * MIN) }, NOW);
  assert.deepEqual([old.value, old.sub.vars.n, old.series], [80, 40, undefined]);
});

test("Claude: po resetu (nové procento neznáme) útrata dnes; čekající schválení = normální karta", () => {
  const f = heroFacts("claude", { limits: limits(90, MIN, -MIN), costs: { summary: { days: days(1, 0.37) } } }, NOW);
  assert.deepEqual([f.value, f.digits, f.unit, f.label[0]], [0.37, 2, "$", "cards.hero.costToday"]);
  assert.equal(heroFacts("claude", { limits: limits(90, MIN, -MIN), costs: { summary: { days: days(1, null) } } }, NOW), null, "dnešek bez ceny");
  // přehled, jehož poslední den není dnešek, o dnešku nic netvrdí (útrata včerejška není „dnes“)
  const stale = heroFacts("claude", { limits: limits(90, MIN, -MIN), costs: { summary: { days: daysEnding(1, 1, 0.37) } } }, NOW);
  assert.equal(stale, null, "poslední den je včerejšek");
  assert.equal(heroFacts("claude", { limits: limits(90, MIN, -MIN), costs: { summary: { days: [] } } }, NOW), null, "bez dnů");
  assert.equal(heroFacts("claude", { pending: [{}], limits: limits(10) }, NOW), null);
  assert.equal(heroFacts("claude", { questions: [{}], limits: limits(10) }, NOW), null);
});

test("Počasí: teplota a dnešní rozsah", () => {
  const w = (data, status = "ok") => ({ personal: { weather: { status, label: "Brno", data } } });
  assert.deepEqual(heroFacts("weather", w({ temp: 12.6, min: 4.4, max: 15.5 }), NOW), {
    value: 13, digits: 0, unit: "°C", label: ["cards.hero.weather", "Venku"], sub: { text: ["cards.hero.minmax", "dnes {min}° až {max}°"], vars: { min: 4, max: 16 } },
  });
  assert.equal(heroFacts("weather", w({ temp: -2, min: null, max: null }, "stale"), NOW).sub, undefined);
  assert.equal(heroFacts("weather", w({ temp: NaN, min: null, max: null }), NOW), null);
});

test("řada útraty: cena, jinak tokeny; posledních n dní", () => {
  assert.deepEqual(costSeries(days(1, 2, 3), 2), [2, 3]);
  assert.deepEqual(costSeries(days(null, 0, null)), [1000, 2000, 3000], "bez ceny podle tokenů");
  assert.deepEqual(costSeries(undefined), []);
});
