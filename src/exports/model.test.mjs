// Test `canExport` (kolo 20): je co exportovat?
//   node --experimental-strip-types --test src/exports/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { canExport } from "./model.ts";

test("canExport: přehled útrat nebo okno limitů", () => {
  const cases = [
    ["nic", {}, false],
    ["útrata ještě nespočítaná", { costs: { summary: null }, limits: { fiveHour: null, sevenDay: null } }, false],
    ["spočítaná útrata", { costs: { summary: { ranges: [] } }, limits: {} }, true],
    ["jen 5h okno", { costs: { summary: null }, limits: { fiveHour: { pct: 1 }, sevenDay: null } }, true],
    ["jen týden", { limits: { sevenDay: { pct: 1 } } }, true],
  ];
  for (const [name, s, want] of cases) assert.equal(canExport(s), want, name);
});
