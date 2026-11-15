// Čistá logika bloku Rozšíření: stav řádku, formát RAM / CPU („—“ bez měření), seznam `extensions`.
//   node --experimental-strip-types --test src/ext/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { extState, fmtCpu, fmtRam, stateTone, withExt } from "./model.ts";

const MB = 1024 ** 2;

test("extState: vypnuto / čeká / běží / pauza / selhalo", () => {
  const cases = [
    [{ enabled: false, phase: "idle" }, "off"],
    [{ enabled: true, phase: "idle" }, "waiting"],
    [{ enabled: true, phase: "starting" }, "starting"],
    [{ enabled: true, phase: "running" }, "running"],
    [{ enabled: true, phase: "paused" }, "paused"],
    [{ enabled: true, phase: "backoff" }, "backoff"],
    [{ enabled: true, phase: "failed" }, "failed"],
    [{ enabled: true, phase: "incompatible" }, "incompatible"],
  ];
  for (const [r, want] of cases) assert.equal(extState(r), want, JSON.stringify(r));
});

test("stateTone: zelená jen běžící, červená selhání", () => {
  const cases = [["running", "ok"], ["failed", "bad"], ["incompatible", "bad"], ["paused", "warn"], ["starting", "warn"], ["backoff", "warn"], ["waiting", "warn"], ["off", ""]];
  for (const [s, want] of cases) assert.equal(stateTone(s), want, s);
});

test("fmtRam: bez měření „—“, jinak MB / GB", () => {
  const c = (ramBytes) => ({ ramBytes, cpuPct: null, procs: 1, outsideJob: 0 });
  assert.equal(fmtRam(null), "—");
  assert.equal(fmtRam(undefined), "—");
  assert.equal(fmtRam(c(62 * MB)), "62 MB");
  assert.equal(fmtRam(c(8.5 * MB)), "8.5 MB");
  assert.equal(fmtRam(c(1.5 * 1024 * MB)), "1.5 GB");
  assert.equal(fmtRam(c(0)), "0 B");
});

test("fmtCpu: bez dvou vzorků „—“, skoro nula „< 0,1“, jinak zlomek jen u malých hodnot", () => {
  const c = (cpuPct) => ({ ramBytes: 1, cpuPct, procs: 1, outsideJob: 0 });
  const cases = [
    [null, "—"],
    [c(null), "—"],
    [c(NaN), "—"],
    [c(0), "< 0.1 %"],
    [c(0.04), "< 0.1 %"],
    [c(0.05), "0.1 %"],
    [c(0.25), "0.3 %"],
    [c(3.14), "3.1 %"],
    [c(12.4), "12 %"],
    [c(100), "100 %"],
  ];
  for (const [x, want] of cases) assert.equal(fmtCpu(x), want, JSON.stringify(x));
});

test("withExt: zapnutí bez duplicit, vypnutí, cizí id zůstane", () => {
  assert.deepEqual(withExt([], "wall", true), ["wall"]);
  assert.deepEqual(withExt(["wall"], "wall", true), ["wall"]);
  assert.deepEqual(withExt(["wall"], "wall", false), []);
  assert.deepEqual(withExt(["future", "wall"], "wall", false), ["future"]);
  assert.deepEqual(withExt(["future"], "wall", true), ["future", "wall"]);
  assert.deepEqual(withExt([], "wall", false), []);
});
