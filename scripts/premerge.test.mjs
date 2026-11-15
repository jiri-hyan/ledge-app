// Čisté funkce premerge.mjs — tabulkově.
//   node --experimental-strip-types --test scripts/premerge.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { isHot, hotFiles, overlap, coreSizes, parseTsc } from "./premerge.mjs";

test("isHot: jádra ui/core/README, placement test, en-*.ts; ne detail a ostatní", () => {
  for (const [f, want] of [
    ["docs/codemap/ui.md", true], ["docs/codemap/core.md", true], ["docs/codemap/README.md", true],
    ["src/settings/placement.test.mjs", true], ["src/i18n/en-notes.ts", true],
    ["docs/codemap/ui-detail.md", false], ["docs/codemap/notes.md", false], ["src/i18n/dicts.ts", false], ["src/App.tsx", false],
  ]) assert.equal(isHot(f), want, f);
  assert.deepEqual(hotFiles(["src/a.ts", "docs/codemap/ui.md"]), ["docs/codemap/ui.md"]);
});

test("overlap: jen soubory ve 2+ větvích; horké výchozně, all=true všechny", () => {
  const by = { a: ["docs/codemap/ui.md", "src/x.ts", "src/x.ts"], b: ["docs/codemap/ui.md", "src/x.ts"], c: ["src/y.ts"] };
  assert.deepEqual(overlap(by), [{ file: "docs/codemap/ui.md", branches: ["a", "b"] }]);
  assert.deepEqual(overlap(by, true).map((o) => o.file), ["docs/codemap/ui.md", "src/x.ts"]);
  assert.deepEqual(overlap({}), []);
});

test("coreSizes: jen jádra nad stropem, detail a podsložky ne", () => {
  const sizes = { "docs/codemap/core.md": 8200, "docs/codemap/ui.md": 8192, "docs/codemap/ui-detail.md": 99999, "docs/codemap/x/y.md": 99999 };
  assert.deepEqual(coreSizes(sizes), [{ file: "docs/codemap/core.md", bytes: 8200, over: 8 }]);
});

test("parseTsc: řádky chyb → záznamy, zdvojené názvy označí dup, ostatní text ignoruje", () => {
  const out = [
    "src/a.ts(10,5): error TS2300: Duplicate identifier 'KitTone'.",
    "src\\b.tsx(3,1): error TS1117: An object literal cannot have multiple properties with the same name.",
    "src/c.ts(1,2): error TS2322: Type 'x' is not assignable to type 'y'.",
    "  Types of property 'z' are incompatible.",
    "Found 3 errors.",
  ].join("\r\n");
  assert.deepEqual(parseTsc(out).map((e) => [e.file, e.line, e.col, e.code, e.dup]), [
    ["src/a.ts", 10, 5, "TS2300", true], ["src/b.tsx", 3, 1, "TS1117", true], ["src/c.ts", 1, 2, "TS2322", false],
  ]);
  assert.deepEqual(parseTsc(""), []);
});
