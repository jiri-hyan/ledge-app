import assert from "node:assert/strict";
import { test } from "node:test";
import { severity, shown, statusText, STATUS_URL } from "./incident.ts";

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

test("řádek se ukáže jen pro skutečný incident", () => {
  assert.equal(shown(undefined), false);
  assert.equal(shown(null), false);
  assert.equal(shown({ name: "  ", status: "other", impact: "none", more: 0 }), false);
  assert.equal(shown({ name: "Elevated errors", status: "identified", impact: "major", more: 0 }), true);
});

test("odkaz vede na veřejnou stránku stavu přes https", () => {
  assert.ok(STATUS_URL.startsWith("https://status.anthropic.com"));
});
