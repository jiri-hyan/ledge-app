// Zrcadlo validace rozložení s Rustem (`validate.ts` ↔ `layout::validate`): obě strany čtou stejnou
// tabulku `src-tauri/src/fixtures/layout-validate.json` (Rust test `validation_mirror_with_ts`).
//   node --experimental-strip-types --test src/layout/validate.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { layoutProblem } from "./validate.ts";
import { defaultLayout, MAX_LIMIT } from "./model.ts";

const cases = JSON.parse(readFileSync(new URL("../../src-tauri/src/fixtures/layout-validate.json", import.meta.url), "utf8"));

test("případ 14: TS odmítne totéž co Rust (stejná tabulka, stejné pole chyby)", () => {
  assert.ok(cases.length >= 10);
  for (const c of cases) assert.equal(layoutProblem(c.layout), c.field, c.name);
  assert.equal(layoutProblem(defaultLayout()), null, "výchozí rozložení je platné");
  const rs = readFileSync(new URL("../../src-tauri/src/layout.rs", import.meta.url), "utf8");
  assert.ok(rs.includes(`pub const MAX_LIMIT: u8 = ${MAX_LIMIT};`), "MAX_LIMIT jako v Rustu");
  assert.ok(rs.includes('include_str!("fixtures/layout-validate.json")'), "Rust čte stejnou tabulku");
});
