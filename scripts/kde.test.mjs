// kde.mjs: čisté hledání definic (tabulkově). Spuštění: node --experimental-strip-types --test scripts/kde.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { findDefs, firstMention } from "./kde.mjs";

const at = (lines, word) => findDefs(lines, word).map((h) => `${h.tier}:${h.line}`);

test("findDefs: fn/struct = tier 1, přiřazení = tier 2, částečná shoda = tier 3", () => {
  const lines = ["pub fn apply_config(c: Config) {", "struct Config {", "MAX = 3", "fn apply_config_now() {}"];
  assert.deepEqual(at(lines, "apply_config"), ["1:1", "3:4"]);
  assert.deepEqual(at(lines, "Config"), ["3:1", "1:2", "3:4"]); // tier 3 řádky jsou částečné shody (řadí se až za tier 1–2)
  assert.deepEqual(at(lines, "MAX"), ["2:3"]);
});

test("findDefs: camelCase klíč configu najde pole v Rustu (snake_case) i v TS; literály a hodnoty ne", () => {
  const lines = [
    "    pub mute_dirty: bool,", // Rust pole
    "    muteDirty?: boolean;", // TS typ
    "        mute_dirty: false,", // inicializace struktury — není definice
    "  const p = { muteDirty: true };", // objektový literál — není definice
    "    pub keep_warm_s: u64,",
  ];
  assert.deepEqual(at(lines, "muteDirty"), ["2:1", "2:2"]);
  assert.deepEqual(at(lines, "keepWarmS"), ["2:5"]);
});

test("firstMention: celé slovo, ne podřetězec", () => {
  assert.deepEqual(firstMention(["a keepWarmSx", "b `keepWarmS` c"], "keepWarmS"), { line: 2, text: "b `keepWarmS` c" });
  assert.equal(firstMention(["nic"], "keepWarmS"), null);
});
