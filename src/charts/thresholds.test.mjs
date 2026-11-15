// @vitest-environment node
// Jedna tabulka prahů limitu/zátěže: TS = Rust `capsule.rs`, a všechna místa ji používají (N1, kolo 34).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { FULL_PCT, WARN_PCT, toneOf } from "./thresholds.ts";
import { level, resetHint, HINT_FROM_PCT } from "../limits/model.ts";
import { LIMIT_PCT } from "../hover/priority.ts";
import { loadOf } from "../integrations/format.ts";
import { HEAT_FULL, HEAT_WARN } from "../notch/look.ts";

const rd = (p) => readFileSync(new URL(p, import.meta.url), "utf8");

test("zrcadlo: TS prahy = Rust capsule.rs", () => {
  const rs = rd("../../src-tauri/src/looks/capsule.rs");
  assert.equal(Number(/pub const WARN_PCT: f64 = ([\d.]+);/.exec(rs)[1]), WARN_PCT);
  assert.equal(Number(/pub const FULL_PCT: f64 = ([\d.]+);/.exec(rs)[1]), FULL_PCT);
});

test("toneOf: hranice (tabulka)", () => {
  for (const [p, want] of [[0, "ok"], [WARN_PCT - 0.1, "ok"], [WARN_PCT, "warn"], [FULL_PCT - 0.1, "warn"], [FULL_PCT, "bad"], [100, "bad"]]) assert.equal(toneOf(p), want, String(p));
});

test("všechna TS místa čtou tutéž tabulku", () => {
  assert.equal(HEAT_WARN, WARN_PCT);
  assert.equal(HEAT_FULL, FULL_PCT);
  assert.equal(HINT_FROM_PCT, WARN_PCT);
  assert.equal(LIMIT_PCT, FULL_PCT);
  for (const p of [0, 50, 74.9, 75, 80, 89.9, 90, 100]) {
    assert.equal(level(p), toneOf(p), `level ${p}`);
    assert.equal(loadOf(p), { ok: "ok", warn: "warm", bad: "hot" }[toneOf(p)], `loadOf ${p}`);
  }
  const w = (pct) => ({ pct, resetsAt: 10_000_000 });
  assert.equal(resetHint(w(WARN_PCT - 0.1), 1, 2, 1e9, true), null);
  assert.notEqual(resetHint(w(WARN_PCT), 1, 2, 1e9, true), null);
});

test("Rust: žádná holá čísla 70/80/90 u limitů, jen konstanty z capsule.rs", () => {
  const strip = (s) => s.replace(/#\[cfg\(test\)\][\s\S]*$/, "");
  const ed = strip(rd("../../src-tauri/src/looks/edgefx.rs"));
  assert.match(ed, /pub const LIMIT_AT: f64 = super::capsule::WARN_PCT;/);
  const cp = strip(rd("../../src-tauri/src/looks/compose.rs"));
  assert.ok(!/limit(5h|7d)\.is_some_and\(\|p\| p >= \d/.test(cp), "is_active limitů přes capsule::WARN_PCT");
  const ll = rd("../../src-tauri/src/limitslive.rs");
  assert.ok(ll.includes("p >= capsule::FULL_PCT => Rgba::RED") && ll.includes("p >= capsule::WARN_PCT => Rgba::AMBER"), "tečka proužku");
  assert.ok(rd("../../src-tauri/src/limits.rs").includes("p >= crate::looks::capsule::FULL_PCT"), "čas resetu v textu proužku");
});
