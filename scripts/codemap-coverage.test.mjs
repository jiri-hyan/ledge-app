// Pokrytí codemap: čistá pravidla (tabulkově, vč. negativních případů) + test nad skutečným stromem.
//   node --experimental-strip-types --test scripts/codemap-coverage.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { readdirSync, readFileSync } from "node:fs";
import { COVERAGE_ALLOW, coverageGaps, needsNaming, pathTokens, tokenCovers, unitOf } from "./codemap-coverage.mjs";
import { loadTree } from "./codemap-lint.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("unitOf: modul = první úroveň src-tauri/src a src, testy a .d.ts mimo", () => {
  for (const [f, want] of [
    ["src-tauri/src/lib.rs", "src-tauri/src/lib.rs"],
    ["src-tauri/src/notes/md.rs", "src-tauri/src/notes"],
    ["src-tauri/src/commands/media.rs", "src-tauri/src/commands"],
    ["src/cards/Head.tsx", "src/cards"],
    ["src/api.ts", "src/api.ts"],
    ["src/hover/hover.css", "src/hover"],
    ["src/hover/Peek.test.mjs", null],
    ["src/cssClasses.test.mjs", null],
    ["src/vite-env.d.ts", null],
    ["scripts/kde.mjs", null],
    ["docs/codemap/core.md", null],
  ]) assert.equal(unitOf(f), want, f);
});

test("needsNaming: produkční zdroj ano; testy, fixtures, mod.rs, index, css ne", () => {
  for (const [f, want] of [
    ["src-tauri/src/notes/md.rs", true],
    ["src/cards/Head.tsx", true],
    ["src/layout/fullLayout.mjs", true],
    ["src-tauri/src/notes/mod.rs", false],
    ["src-tauri/src/config/tests.rs", false],
    ["src-tauri/src/mcp/notes_tests.rs", false],
    ["src-tauri/src/claude/testkit.rs", false],
    ["src-tauri/src/fixtures/x.json", false],
    ["src/types/index.ts", false],
    ["src/hover/hover.css", false],
    ["src/hover/Peek.test.mjs", false],
    ["src/vite-env.d.ts", false],
    ["scripts/kde.mjs", false],
  ]) assert.equal(needsNaming(f), want, f);
});

test("pathTokens: backtick cesty, rozbalené {a,b}, bez ./ a koncového /", () => {
  const md = "`a/{b,c}.rs` `./x/` `Foo` `notes::md` `d.ts` `src/` `src-tauri/src/` text";
  assert.deepEqual(pathTokens(md).sort(), ["a/b.rs", "a/c.rs", "d.ts", "x"]); // kořeny (`src/`) nejmenují modul
});

test("tokenCovers: kořen / src-tauri/src / src, mod.rs, index.ts, glob; named = ne holá složka/*", () => {
  for (const [tok, file, named, want] of [
    ["src-tauri/src/notes/md.rs", "src-tauri/src/notes/md.rs", false, true],
    ["notes/md.rs", "src-tauri/src/notes/md.rs", true, true],
    ["md.rs", "src-tauri/src/notes/md.rs", true, false], // zkrácené jen od kořene; zkratku řeší coverageGaps
    ["config.rs", "src-tauri/src/config/mod.rs", true, true],
    ["types.ts", "src/types/index.ts", true, true],
    ["notes/*", "src-tauri/src/notes/md.rs", false, true],
    ["notes/*", "src-tauri/src/notes/md.rs", true, false], // holý glob nejmenuje soubor
    ["notes/*.rs", "src-tauri/src/notes/md.rs", true, false],
    ["i18n/en-*.ts", "src/i18n/en-cards.ts", true, true], // vzor s literálním kusem jmenuje
    ["src/hover", "src/hover/Peek.tsx", false, true], // složka kryje modul…
    ["src/hover", "src/hover/Peek.tsx", true, false], // …ale nejmenuje soubor
    ["notes/md.rs", "src-tauri/src/notesync/md.rs", false, false],
  ]) assert.equal(tokenCovers(tok, file, named), want, `${tok} → ${file} (named=${named})`);
});

const FILES = [
  "src-tauri/src/lib.rs",
  "src-tauri/src/notes/mod.rs",
  "src-tauri/src/notes/md.rs",
  "src-tauri/src/notes/edit.rs",
  "src-tauri/src/commands/media.rs",
  "src-tauri/src/commands/timer.rs",
  "src-tauri/src/timer.rs",
  "src/cards/Head.tsx",
  "src/cards/head.css",
  "src/projects/state.ts",
  "src/tilelook/state.ts",
];
const MAP = [
  "| soubor | co |",
  "|---|---|",
  "| `lib.rs` · `timer.rs` · `commands/media.rs` | jádro |",
  "| `notes/*` (`notes/md.rs`) | poznámky |",
  "| `cards/*` | karty |",
  "| `src/projects/state.ts` | stav |",
].join("\n");

test("coverageGaps: pokryté neruší; chybějící soubor, modul i holý glob se ohlásí", () => {
  // notes/edit.rs je jen pod holým `notes/*`; commands/timer.rs a Head.tsx jen pod globem / jiným souborem;
  // timer.rs (kořenový) nesmí krýt commands/timer.rs; tilelook/state.ts nikdo nejmenuje a jeho modul není v mapě
  const g = coverageGaps(FILES, [MAP], {});
  assert.deepEqual(g.missing, ["src/tilelook"]);
  assert.deepEqual(g.unnamed, [
    "src-tauri/src/commands/timer.rs",
    "src-tauri/src/notes/edit.rs",
    "src/cards/Head.tsx",
    "src/tilelook/state.ts",
  ]);
  assert.deepEqual(g.staleAllow, []);
});

test("coverageGaps: přidaný modul bez mapy selže (negativní případ)", () => {
  const before = coverageGaps(FILES, [MAP], {});
  const after = coverageGaps([...FILES, "src-tauri/src/novy.rs", "src/novacarta/Karta.tsx"], [MAP], {});
  assert.deepEqual(after.missing.filter((m) => !before.missing.includes(m)), ["src-tauri/src/novy.rs", "src/novacarta"]);
  assert.ok(after.unnamed.includes("src-tauri/src/novy.rs"));
});

test("coverageGaps: zkrácený zápis platí jen v mapě, která modul jmenuje celou cestou, a jen když nic nepojmenuje přesně", () => {
  const files = ["src-tauri/src/accounts/oauth.rs", "src-tauri/src/accounts/mod.rs", "src-tauri/src/timer.rs", "src-tauri/src/commands/timer.rs"];
  const ok = ["| `accounts/*` | x |", "| `oauth.rs` | zkráceně |", "| `timer.rs` · `commands/timer.rs` | x |"].join("\n");
  assert.deepEqual(coverageGaps(files, [ok], {}).unnamed, []);
  // `oauth.rs` v mapě, která accounts nejmenuje, nekryje nic; `timer.rs` nekryje commands/timer.rs
  const bad = ["| `oauth.rs` | zkráceně |", "| `timer.rs` | x |", "| `lib.rs` | x |"].join("\n");
  const g = coverageGaps(files, [bad], {});
  assert.deepEqual(g.unnamed, ["src-tauri/src/accounts/oauth.rs", "src-tauri/src/commands/timer.rs"]);
});

test("coverageGaps: výjimka skryje nález, zbytečná výjimka je chyba", () => {
  const all = [...FILES, "src-tauri/src/novy.rs"];
  const g = coverageGaps(all, [MAP], { "src-tauri/src/novy.rs": "test", "src/tilelook": "test" });
  assert.ok(!g.missing.includes("src-tauri/src/novy.rs") && !g.missing.includes("src/tilelook"));
  const stale = coverageGaps(FILES, [MAP], { "src-tauri/src/lib.rs": "už pokryté", "src/neexistuje": "není", "src/cards/Head.tsx": "x" });
  assert.deepEqual(stale.staleAllow.sort(), ["src-tauri/src/lib.rs", "src/neexistuje"]);
});

test("skutečný strom: každý modul a každý produkční zdroj má řádek v nějaké codemapě", () => {
  const dir = path.join(root, "docs", "codemap");
  const mds = readdirSync(dir).filter((x) => x.endsWith(".md")).map((n) => readFileSync(path.join(dir, n), "utf8"));
  const { files } = loadTree(root);
  assert.ok(files.length > 500, `strom má jen ${files.length} souborů`); // pojistka proti tichému rozbití procházení
  const g = coverageGaps(files, mds, COVERAGE_ALLOW);
  assert.deepEqual(g.missing, [], "moduly bez mapy — jmenuj je v docs/codemap/*.md (nebo výjimka s důvodem v COVERAGE_ALLOW)");
  assert.deepEqual(g.unnamed, [], "soubory bez řádku v mapě — jmenuj je výslovně v tabulce mapy své oblasti");
  assert.deepEqual(g.staleAllow, [], "zbytečné výjimky v COVERAGE_ALLOW");
});
