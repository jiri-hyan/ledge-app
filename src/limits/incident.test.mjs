import assert from "node:assert/strict";
import { test } from "node:test";
import { ageMin, severity, shown, staleMin, statusText, STATUS_URL } from "./incident.ts";

test("závažnost podle dopadu", () => {
  const cases = [["critical", "bad"], ["major", "bad"], ["minor", "warn"], ["none", "info"], ["", "info"], ["cokoliv", "info"]];
  for (const [impact, want] of cases) assert.equal(severity(impact), want, impact);
});

test("popisek stavu: známé kódy a záloha", () => {
  assert.equal(statusText("investigating"), "vyšetřuje se");
  assert.equal(statusText("identified"), "příčina nalezena");
  assert.equal(statusText("monitoring"), "sleduje se");
  assert.equal(statusText("scheduled"), "probíhá");
  assert.equal(statusText(""), "probíhá");
});

const MIN = 60000;
const inc = (over = {}) => ({ name: "Elevated errors", status: "identified", impact: "major", more: 0, okMs: 100 * MIN, ttlMin: 20, ...over });

test("řádek se ukáže jen pro skutečný čerstvý incident", () => {
  const now = 105 * MIN;
  assert.equal(shown(undefined, now), false);
  assert.equal(shown(null, now), false);
  assert.equal(shown(inc({ name: "  " }), now), false);
  assert.equal(shown(inc(), now), true);
});

test("stáří: ukázat / poznámka / skrýt podle prahu", () => {
  // [název, now - okMs v min, shown, staleMin]
  const cases = [["čerstvý", 3, true, null], ["do jednoho odstupu", 10, true, null], ["starší než odstup", 11, true, 11], ["na prahu", 20, true, 20], ["po prahu", 21, false, 21], ["hodiny zpět (záporné stáří)", -5, true, null]];
  for (const [name, age, vis, note] of cases) {
    const now = 100 * MIN + age * MIN;
    assert.equal(shown(inc(), now), vis, name);
    assert.equal(staleMin(inc(), now), note, name);
  }
  assert.equal(ageMin(inc(), 130 * MIN), 30);
});

test("odkaz vede na veřejnou stránku stavu přes https", () => {
  assert.ok(STATUS_URL.startsWith("https://status.anthropic.com"));
});
