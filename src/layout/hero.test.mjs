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
  assert.equal(f.series, undefined, "limit v % nikdy nekreslí řadu útraty");
  const h = heroFacts("claude", { limits: { ...limits(42.7), history5h: [[1, 10], [2, 30], [3, 42]] } }, NOW);
  assert.deepEqual(h.series, [10, 30, 42], "řada = skutečná historie limitu");
  // starý vzorek: číslo zůstane, druhý řádek řekne stáří
  const old = heroFacts("claude", { limits: limits(80, 40 * MIN) }, NOW);
  assert.deepEqual([old.value, old.sub.vars.n, old.series], [80, 40, undefined]);
});

test("Claude: po resetu NEPŘESKOČÍ na útratu — prázdný stav; čekající schválení = normální karta", () => {
  const f = heroFacts("claude", { limits: limits(90, MIN, -MIN), costs: { summary: { days: days(1, 0.37) } } }, NOW);
  assert.equal(f.unit, "");
  assert.deepEqual(f.label, ["cards.hero.limit5h", "Limit Claude 5 h"]);
  assert.deepEqual(f.empty.title, ["cards.hero.noLimits", "Limity zatím nejsou"]);
  assert.equal(f.empty.why[0], "cards.hero.why.reset");
  assert.equal(heroFacts("claude", { pending: [{}], limits: limits(10) }, NOW), null);
  assert.equal(heroFacts("claude", { questions: [{}], limits: limits(10) }, NOW), null);
});

test("Claude: chybějící limity = prázdný stav s důvodem podle zapojení stavového řádku", () => {
  const why = (statusLine, extra = {}) => heroFacts("claude", { limits: { at: null, staleMs: 1, fiveHour: null, statusLine, ...extra }, costs: { summary: { days: days(1, 0.37) } } }, NOW).empty.why[0];
  assert.equal(why("none"), "cards.hero.why.connect");
  assert.equal(why("foreign"), "cards.hero.why.connect");
  assert.equal(why("full"), "cards.hero.why.waiting");
  assert.equal(why("error"), "cards.hero.why.error");
  assert.equal(why(""), "cards.hero.why.none");
  for (const input of [{}, { limits: null }]) {
    const f = heroFacts("claude", input, NOW);
    assert.equal(f.empty.why[0], "cards.hero.why.none", "žádný snapshot limitů");
    assert.equal(f.unit, "", "nikdy cizí číslo");
  }
});

test("Claude: volba hrdiny — týden a útrata dnes", () => {
  const wk = { ...limits(61.9), sevenDay: { pct: 33.4, resetsAt: NOW + 3 * 86_400_000 }, statusLine: "full" };
  const w = heroFacts("claude", { limits: wk }, NOW, "week");
  assert.deepEqual([w.value, w.unit, w.label[0]], [33, "%", "cards.hero.limitWeek"]);
  assert.deepEqual(w.sub.vars, { h: 72, m: 0 });
  assert.ok(heroFacts("claude", { limits: { ...wk, sevenDay: null } }, NOW, "week").empty, "bez týdne = prázdný stav, ne 5 h");
  const c = { summary: { days: days(1, 0.37) } };
  const sp = heroFacts("claude", { limits: wk, costs: c }, NOW, "spend");
  assert.deepEqual([sp.value, sp.digits, sp.unit, sp.label[0]], [0.37, 2, "$", "cards.hero.costToday"]);
  for (const costs of [{ summary: { days: days(1, null) } }, { summary: { days: daysEnding(1, 1, 0.37) } }, { summary: { days: [] } }, { summary: null }, undefined]) {
    const e = heroFacts("claude", { limits: wk, costs }, NOW, "spend");
    assert.equal(e.empty.title[0], "cards.hero.noSpend");
  }
  assert.equal(heroFacts("claude", { limits: wk, costs: c }, NOW).label[0], "cards.hero.limit5h", "výchozí volba = 5 h");
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

test("Claude: důvod chybějících limitů z dalších zdrojů má přednost před stavovým řádkem", () => {
  const why = (extra) => heroFacts("claude", { limits: { at: null, staleMs: 1, fiveHour: null, statusLine: "full", ...extra } }, NOW).empty.why[0];
  const cases = [
    [{ fetch: "expired" }, "cards.hero.why.fetchExpired"],
    [{ fetch: "noLogin" }, "cards.hero.why.fetchNoLogin"],
    [{ fetch: "error" }, "cards.hero.why.fetchError"],
    [{ fetch: "pending" }, "cards.hero.why.fetchPending"],
    [{ fetch: "ok", desktop: "none" }, "cards.hero.why.desktopNone"],
    [{ fetch: "", desktop: "found" }, "cards.hero.why.waiting"],
    [{}, "cards.hero.why.waiting"],
  ];
  for (const [extra, key] of cases) assert.equal(why(extra), key, JSON.stringify(extra));
});
