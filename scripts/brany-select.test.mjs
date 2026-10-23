// Výběr bran podle změněných souborů — tabulkově.
//   node --experimental-strip-types --test scripts/brany-select.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { kindOf, selectGates } from "./brany-select.mjs";

test("kindOf: druh změny podle cesty", () => {
  for (const [f, want] of [
    ["src/App.tsx", "ui"], ["src/notes/md.test.mjs", "ui"], ["src/styles.css", "ui"], ["pin.html", "ui"], ["vite.config.ts", "ui"],
    ["scripts/kde.mjs", "ui"], ["src-tauri/src/lib.rs", "rust"], ["src-tauri/icons/a.png", "rust"],
    ["src-tauri/Cargo.toml", "wide"], ["src-tauri/Cargo.lock", "wide"], ["src-tauri/tauri.conf.json", "wide"],
    ["package.json", "wide"], ["pnpm-lock.yaml", "wide"],
    ["docs/codemap/notes.md", "docs"], ["README.md", "docs"], ["docs/screenshots/a.png", "docs"],
    [".github/workflows/ci.yml", "other"], ["scripts/publish.ps1", "other"], ["LICENSE", "other"],
    ["src" + String.fromCharCode(92) + "notes" + String.fromCharCode(92) + "draft.ts", "ui"], // zpětná lomítka z Windows
  ]) assert.equal(kindOf(f), want, f);
});

test("selectGates: tabulka výběru", () => {
  for (const [files, gates] of [
    [[], []],
    [["src/App.tsx"], ["tsc", "ui"]],
    [["src/a.css", "scripts/x.mjs"], ["tsc", "ui"]],
    [["src-tauri/src/lib.rs"], ["test", "clippy"]],
    [["src-tauri/src/lib.rs", "src-tauri/src/a.rs"], ["test", "clippy"]],
    [["src/App.tsx", "src-tauri/src/lib.rs"], ["test", "clippy", "tsc", "ui"]],
    [["package.json"], ["test", "clippy", "tsc", "ui"]],
    [["src-tauri/Cargo.lock"], ["test", "clippy", "tsc", "ui"]],
    [["src-tauri/tauri.conf.json", "docs/a.md"], ["test", "clippy", "tsc", "ui"]],
    [["docs/codemap/notes.md"], ["ui"]],
    [["CLAUDE.md", "docs/agents/config.md"], ["ui"]],
    [["docs/a.md", "src/App.tsx"], ["tsc", "ui"]],
    [["docs/a.md", "src-tauri/src/lib.rs"], ["test", "clippy", "ui"]],
    [[".github/workflows/ci.yml"], []],
    [[".github/workflows/ci.yml", "src/a.ts"], ["tsc", "ui"]],
  ]) assert.deepEqual(selectGates(files).gates, gates, files.join(" "));
});

test("selectGates: vždy vysvětlí proč", () => {
  for (const files of [[], ["src/a.ts"], ["src-tauri/src/a.rs"], ["package.json"], ["docs/a.md"], ["LICENSE"]]) {
    assert.ok(selectGates(files).why.length > 5, files.join(" "));
  }
});
