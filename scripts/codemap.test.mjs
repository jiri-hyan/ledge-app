// Hniloba codemap: čisté pravidla (tabulkově) + jeden test nad skutečným stromem.
//   node --experimental-strip-types --test scripts/codemap.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { readdirSync, readFileSync } from "node:fs";
import { expandBraces, isPathToken, lintCodemaps, lintMarkdown, parseTables, pathExists } from "./codemap-lint.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

// Výjimky z kontroly na skutečném stromu — jen výslovně, s důvodem. Prázdné = žádná mapa nelže.
const ALLOW_FILES = new Set([]);
const ALLOW_SYMBOLS = new Set([]);

const entries = [
  "src-tauri/src/notes/mod.rs",
  "src-tauri/src/notes/md.rs",
  "src-tauri/src/config/mod.rs", // modul rozdělený do složky: `config.rs` v mapách musí projít
  "src-tauri/src/calendar/conf.rs",
  "src/types/index.ts", // totéž pro `types.ts`
  "src/cards/sections.ts",
  "src/hover/Peek.tsx",
  "src/styles.css",
  "src-tauri/src/notes",
  "src-tauri/src/config",
  "src/hover",
];

test("expandBraces rozbalí {a,b} i vnořeně", () => {
  assert.deepEqual(expandBraces("a/b.rs"), ["a/b.rs"]);
  assert.deepEqual(expandBraces("a/{b,c}.rs"), ["a/b.rs", "a/c.rs"]);
  assert.deepEqual(expandBraces("{a,b}/{c,d}"), ["a/c", "a/d", "b/c", "b/d"]);
});

test("isPathToken: cesta má lomítko nebo příponu, ne mezeru", () => {
  for (const [t, want] of [
    ["src/a.rs", true], ["mod.rs", true], ["a.css", true], ["hover/*", true], ["a.mjs", true],
    ["Foo", false], ["ledge.exe --mcp", false], ["notes::md", false], ["README.md", false],
  ]) assert.equal(isPathToken(t), want, t);
});

test("pathExists: přesná, zkrácená, glob, mod.rs/index.ts", () => {
  for (const [p, want] of [
    ["src-tauri/src/notes/md.rs", true],
    ["src-tauri/src/notes/{zzz,yyy}.rs", false],
    ["notes/md.rs", true], // zkrácená cesta (přípona cesty)
    ["md.rs", true],
    ["notes/nope.rs", false],
    ["src-tauri/src/notes/{md,mod}.rs", true],
    ["src-tauri/src/notes/{md,zzz}.rs", true], // stačí jedna z rozbalených variant
    ["notes/*", true],
    ["hover/*", true],
    ["cards/sec*.ts", true],
    ["zzz/*", false],
    ["config.rs", true], // existuje config/mod.rs
    ["src-tauri/src/config.rs", true],
    ["types.ts", true], // existuje types/index.ts
    ["src/types.ts", true],
    ["src-tauri/src/nope.rs", false],
    ["src/hover/", true],
  ]) assert.equal(pathExists(p, entries), want, p);
});

test("parseTables: hlavička, oddělovač, escapované |", () => {
  const BS = String.fromCharCode(92);
  const md = ["text", "", "| soubor | symboly |", "|---|---|", "| `a.rs` | `x` `y` |", "| a" + BS + "|b | z |", "", "po"].join("\n");
  const [t] = parseTables(md);
  assert.deepEqual(t.header, ["soubor", "symboly"]);
  assert.equal(t.rows.length, 2);
  assert.equal(t.rows[0].line, 5);
  assert.deepEqual(t.rows[0].cells, ["`a.rs`", "`x` `y`"]);
  assert.deepEqual(t.rows[1].cells, ["a|b", "z"]);
});

test("lintMarkdown: chybějící soubor a symbol, allowlist, ignorování ostatních sloupců", () => {
  const ctx = { entries, words: new Set(["to_html", "Real"]) };
  const md = [
    "| soubor | odpovědnost | klíčové symboly |",
    "|---|---|---|",
    "| `src-tauri/src/notes/md.rs` | `Vymyšlené` text | `to_html` `Real` |",
    "| `notes/gone.rs` · `md.rs` | něco | `Zmizelo` `a::b` `Draft*` `to_html` |",
    "| `zzz/*` | | |",
    "",
    "| oblast | hlavní soubory |",
    "|---|---|",
    "| x | `nope/neni.rs` |",
  ].join("\n");
  assert.deepEqual(lintMarkdown(md, ctx).map((f) => `${f.kind}:${f.value}:${f.line}`),
    ["file:notes/gone.rs:4", "symbol:Zmizelo:4", "file:zzz/*:5"]);
  const allowed = lintMarkdown(md, { ...ctx, allowFiles: new Set(["notes/gone.rs", "zzz/*"]), allowSymbols: new Set(["Zmizelo"]) });
  assert.deepEqual(allowed, []);
});

test("skutečný strom: codemapy neodkazují na neexistující soubory ani symboly", () => {
  // pojistka proti tichému rozbití parseru: mapy musí mít stovky kontrolovaných řádků
  const dir = path.join(root, "docs", "codemap");
  let rows = 0;
  for (const n of readdirSync(dir).filter((x) => x.endsWith(".md"))) {
    for (const t of parseTables(readFileSync(path.join(dir, n), "utf8"))) {
      if (t.header.some((h) => h.startsWith("soubor"))) rows += t.rows.length;
    }
  }
  assert.ok(rows > 300, `parser našel jen ${rows} řádků tabulek souborů`);
  const found = lintCodemaps(root, { allowFiles: ALLOW_FILES, allowSymbols: ALLOW_SYMBOLS });
  assert.deepEqual(found.map((f) => `${f.file}:${f.line}  ${f.kind}  ${f.value}`), []);
});

import { MAX_CORE_BYTES, MAX_LINE_CHARS, budgetFindings } from "./codemap-lint.mjs";

test("rozpočet jádra: strop bajtů, délka řádku, odkaz na detail; -detail je vyňatý", () => {
  const kinds = (name, md, det) => budgetFindings(name, md, new Set(det)).map((f) => f.kind);
  assert.deepEqual(kinds("app.md", "# ok\n", []), []);
  assert.deepEqual(kinds("app.md", "x".repeat(MAX_CORE_BYTES + 1), []), ["size", "line"]);
  assert.deepEqual(kinds("app.md", "y".repeat(MAX_LINE_CHARS + 1), []), ["line"]);
  assert.deepEqual(kinds("app.md", "# bez odkazu\n", ["app-detail.md"]), ["detail-link"]);
  assert.deepEqual(kinds("app.md", "viz [d](app-detail.md)\n", ["app-detail.md"]), []);
  assert.deepEqual(kinds("app-detail.md", "x".repeat(50000), ["app-detail.md"]), []);
});

test("skutečný strom: každé jádro mapy ≤ 8 KB a každý řádek ≤ 600 znaků", () => {
  const dir = path.join(root, "docs", "codemap");
  const names = new Set(readdirSync(dir));
  const bad = [];
  for (const n of names) {
    if (!n.endsWith(".md")) continue;
    for (const f of budgetFindings(n, readFileSync(path.join(dir, n), "utf8"), names)) bad.push(`${n}:${f.line} ${f.kind} ${f.value}`);
  }
  assert.deepEqual(bad, []);
});
