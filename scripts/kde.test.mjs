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

import { mapRows, outline, renderFile } from "./kde-file.mjs";

test("outline: Rust hlavičky deklarací s čísly řádků, bez těl a bez komentářů", () => {
  const lines = ["// fn ne", "pub struct A {", "    x: u8,", "}", "impl A {", "    pub(crate) async fn go(&self) -> u8 {", "    }", "}", "let fn_x = 1;", "pub enum E { A }"];
  assert.deepEqual(outline(lines, "x.rs").map((o) => `${o.line}:${o.text}`), ["2:pub struct A", "5:impl A", "6:pub(crate) async fn go(&self) -> u8", "10:pub enum E { A }"]);
});

test("outline: TS jen export function|const|type|interface, jiné soubory nic", () => {
  const lines = ["const skryte = 1;", "export function a(x: number) {", "export const b = () => 1;", "export type T = string;", "  export interface I {"];
  assert.deepEqual(outline(lines, "x.ts").map((o) => o.line), [2, 3, 4, 5]);
  assert.deepEqual(outline(lines, "x.css"), []);
});

test("mapRows: přesná cesta vyhrává nad zkráceným zápisem; holý vzor nestačí", () => {
  const core = ["| soubor | účel |", "| `src-tauri/src/app/conf.rs` | plná cesta |", "| `conf.rs` | zkrácená |", "| `src-tauri/src/app/*` | holý vzor |"].join("\n");
  const rows = mapRows([["app.md", core]], "src-tauri/src/app/conf.rs");
  assert.deepEqual(rows.map((r) => r.line), [2]);
  assert.deepEqual(mapRows([["app.md", core]], "src-tauri/src/other/conf.rs").map((r) => r.line), [3]);
  assert.deepEqual(mapRows([["app.md", "| `a.rs` | x |"]], "src/zzz.ts"), []);
});

test("renderFile: hlavička, řádek mapy a osnova; bez řádku mapy to řekne", () => {
  const txt = renderFile("src/a.ts", [{ map: "app.md", line: 4, text: "| `src/a.ts` | účel |" }], ["export const x = 1;"]);
  assert.match(txt, /^# codemapa: src\/a\.ts\ndocs\/codemap\/app\.md:4 {2}\| `src\/a\.ts` \| účel \|\n# osnova \(1 deklarací z 1 řádků\)\nsrc\/a\.ts:1 {2}export const x = 1;$/);
  assert.match(renderFile("src/b.ts", [], []), /žádný řádek v docs\/codemap/);
});
