// Čerstvost počasí: jedno pravidlo (čistá logika bez DOM).
import { test } from "node:test";
import assert from "node:assert/strict";
import { WX_GONE_MS, WX_OLD_MS, weatherAge } from "./weatherAge.ts";

test("weatherAge: čerstvé / stará data / zmizet", () => {
  const NOW = 10_000_000_000;
  const MIN = 60_000;
  const w = (status, ago) => ({ status, fetchedMs: ago == null ? null : NOW - ago });
  const cases = [
    ["čerstvé", w("ok", MIN), "fresh"],
    ["těsně před 30 min", w("ok", WX_OLD_MS - 1), "fresh"],
    ["30 min = stará", w("ok", WX_OLD_MS), "old"],
    ["stale i čerstvé = stará", w("stale", MIN), "old"],
    ["2,5 h = stará, ne čerstvé", w("ok", 150 * MIN), "old"],
    ["3 h = zmizí", w("ok", WX_GONE_MS), "gone"],
    ["stale 3 h = zmizí", w("stale", WX_GONE_MS), "gone"],
    ["bez času = nevíme → zmizí", w("ok", null), "gone"],
    ["nic", undefined, "gone"],
  ];
  for (const [name, x, want] of cases) assert.equal(weatherAge(x, NOW), want, name);
});
