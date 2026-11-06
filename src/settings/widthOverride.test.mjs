// Přebití předvolby šířky (P2-3): zrcadlo pořadí v `appearance/geom.rs::sized_width`.
//   node --experimental-strip-types --test src/settings/widthOverride.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { clearDragWidth, widthOverride } from "./widthOverride.ts";

test("pořadí přebití: notch > tažení > nic", () => {
  const cases = [
    [{ layout: "classic" }, null],
    [{ layout: "classic", size: {} }, null],
    [{ layout: "classic", size: { width: 0, height: 400 } }, null],
    [{ layout: "classic", size: { width: 1180 } }, { kind: "drag", px: 1180, perPage: false }],
    [{ layout: "notch", size: { width: 1180 } }, { kind: "notch" }],
    [{ layout: "notch" }, { kind: "notch" }],
    // šířky záložek platí jen s perPage
    [{ layout: "classic", size: { pages: { prace: { width: 900 } } } }, null],
    [{ layout: "classic", size: { perPage: true, pages: { prace: { width: 900 } } } }, { kind: "drag", px: 900, perPage: true }],
    [{ layout: "classic", size: { perPage: true, width: 1000, pages: { prace: { width: 900 } } } }, { kind: "drag", px: 1000, perPage: true }],
  ];
  for (const [a, want] of cases) assert.deepEqual(widthOverride(a), want, JSON.stringify(a));
});

test("zrušení přebití nuluje šířky, ale ne výšky ani perPage", () => {
  assert.deepEqual(clearDragWidth(undefined), { width: 0 });
  const out = clearDragWidth({ width: 1180, height: 500, perPage: true, pages: { prace: { width: 900, height: 300 } } });
  assert.deepEqual(out, { width: 0, height: 500, perPage: true, pages: { prace: { width: 0, height: 300 } } });
  assert.equal(widthOverride({ layout: "classic", size: out }), null);
});
