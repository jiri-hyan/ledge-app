// @vitest-environment node
//   node --experimental-strip-types --test src/settings/merged.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { notifyOn, notifyChoice, notifyWrites, limitsSource, limitsWrites } from "./merged.ts";

test("starý config: vypínače oznámení → stejná volba jako dřív (tabulka)", () => {
  const cases = [
    // [cfg, událost, notch, očekávaná volba]
    [{}, "waiting", true, "off"],
    [{ claude: { toast: true } }, "waiting", true, "unfold"],
    [{ claude: { toast: true } }, "waiting", false, "on"],
    [{ claude: { toast: true } }, "done", true, "sides"], // toastDone chybí → řídí se „čeká“
    [{ claude: { toast: true, toastDone: false } }, "done", true, "off"],
    [{ claude: { toast: false, toastDone: true } }, "done", true, "sides"],
    [{ claude: { limitsAlert: true }, notify: { style: { limit: "glow" } } }, "limit", true, "glow"],
    [{}, "limit", true, "off"],
    [{}, "timer", true, "glow"], // timer.toast výchozí zapnuto
    [{ timer: { toast: false } }, "timer", true, "off"],
    [{}, "meeting", true, "drop"], // bez vypínače se ukazuje jen styl
  ];
  for (const [cfg, k, notch, want] of cases) assert.equal(notifyChoice(cfg, k, notch), want, `${JSON.stringify(cfg)} ${k}`);
  assert.equal(notifyOn({}, "meeting"), null);
});

test("zápis volby oznámení: vypínač + styl; vypnutí styl nemaže", () => {
  assert.deepEqual(notifyWrites("waiting", "off"), [["claude.toast", false]]);
  assert.deepEqual(notifyWrites("done", "glow"), [["claude.toastDone", true], ["notify.style.done", "glow"]]);
  assert.deepEqual(notifyWrites("timer", "on"), [["timer.toast", true]]);
  assert.deepEqual(notifyWrites("meeting", "ticker"), [["notify.style.meeting", "ticker"]]);
});

test("zdroj limitů: dva staré přepínače → jedna volba a zpět", () => {
  const cases = [
    [{}, "none"],
    [{ claude: { limitsDesktop: true } }, "desktop"],
    [{ claude: { limitsFetch: true } }, "fetch"],
    [{ claude: { limitsDesktop: true, limitsFetch: true } }, "both"],
  ];
  for (const [cfg, want] of cases) {
    assert.equal(limitsSource(cfg), want);
    const w = Object.fromEntries(limitsWrites(want));
    assert.equal(w["claude.limitsDesktop"], !!cfg.claude?.limitsDesktop);
    assert.equal(w["claude.limitsFetch"], !!cfg.claude?.limitsFetch);
  }
});
