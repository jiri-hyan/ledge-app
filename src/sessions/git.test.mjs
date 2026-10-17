// Tabulkové testy čisté logiky panelu Git (`git.ts`).
//   node --experimental-strip-types --test src/sessions/git.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { badge, canPush, commitReady, defaultPicked, diffModes, hiddenFiles, messageKey, staged, togglePicked, unstaged } from "./git.ts";

const F = (path, kind, x, y) => ({ path, orig: null, kind, x, y, inHead: true, added: null, removed: null, discard: null });
const files = [F("a.rs", "changed", ".", "M"), F("b.rs", "changed", "A", "."), F("c.rs", "changed", "M", "M"), F("k.rs", "conflict", "U", "U"), F("n.txt", "untracked", "?", "?")];

test("písmeno stavu a staged/unstaged", () => {
  for (const [f, want] of [[files[0], "M"], [files[1], "A"], [files[2], "M"], [files[3], "U"], [files[4], "?"], [F("r", "renamed", "R", "."), "R"], [F("d", "changed", ".", "D"), "D"]])
    assert.equal(badge(f), want, f.path);
  assert.deepEqual(files.map(staged), [false, true, true, true, false]);
  assert.deepEqual(files.map(unstaged), [true, false, true, true, true]);
});

test("výběr ke commitu: výchozí bez konfliktů, přepínání drží pořadí", () => {
  assert.deepEqual(defaultPicked(files), ["a.rs", "b.rs", "c.rs", "n.txt"]);
  assert.deepEqual(togglePicked(["a.rs", "c.rs"], "b.rs", files), ["a.rs", "b.rs", "c.rs"]);
  assert.deepEqual(togglePicked(["a.rs", "c.rs"], "a.rs", files), ["c.rs"]);
  assert.deepEqual(togglePicked([], "zmizel.rs", files), []);
  for (const [msg, picked, want] of [["feat: x", ["a.rs"], true], ["   ", ["a.rs"], false], ["x", [], false], ["", [], false]])
    assert.equal(commitReady(msg, picked), want, JSON.stringify([msg, picked]));
});

test("diffy souboru: pracovní složka napřed, pak připravené", () => {
  assert.deepEqual(files.map(diffModes), [[false], [true], [false, true], [false, true], [false]]);
});

test("klávesy pole zprávy", () => {
  for (const [key, ctrl, want] of [["Enter", true, "send"], ["Enter", false, null], ["Escape", false, "cancel"], ["a", true, null]])
    assert.equal(messageKey(key, ctrl), want, `${key} ${ctrl}`);
});

test("skryté soubory a push", () => {
  assert.equal(hiddenFiles({ files: files, total: 205 }), 200);
  assert.equal(hiddenFiles({ files: files, total: 3 }), 0);
  assert.equal(canPush({ push: { kind: "no", why: "x" } }), false);
  assert.equal(canPush({ push: { kind: "upstream", target: "origin/main", count: 2 } }), true);
  assert.equal(canPush({ push: { kind: "setUpstream", remote: "origin", branch: "w", count: null } }), true);
});
