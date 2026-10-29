// @vitest-environment node
//   node --experimental-strip-types --test src/limits/sources.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { offers, withOffer } from "./sources.ts";

test("nabídky zdrojů jen bez dat a jen pro vypnuté volby", () => {
  const cases = [
    [null, []],
    [{ at: 1, desktop: "", fetch: "" }, []],
    [{ at: null }, ["desktop", "fetch"]],
    [{ at: null, desktop: "", fetch: "" }, ["desktop", "fetch"]],
    [{ at: null, desktop: "none", fetch: "" }, ["fetch"]],
    [{ at: null, desktop: "", fetch: "expired" }, ["desktop"]],
    [{ at: null, desktop: "found", fetch: "pending" }, []],
  ];
  for (const [l, want] of cases) assert.deepEqual(offers(l), want, JSON.stringify(l));
});

test("zapnutí volby nemění nic jiného", () => {
  const cfg = { claude: { port: 47800, limitsPace: true }, strip: { enabled: true } };
  assert.deepEqual(withOffer(cfg, "desktop"), { claude: { port: 47800, limitsPace: true, limitsDesktop: true }, strip: { enabled: true } });
  assert.deepEqual(withOffer(cfg, "fetch").claude.limitsFetch, true);
  assert.equal(cfg.claude.limitsDesktop, undefined, "vstup se nemění");
  assert.deepEqual(withOffer({}, "fetch"), { claude: { limitsFetch: true } });
});
