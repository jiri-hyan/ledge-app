// Propojení poznámek se složkou (`folderlink.ts`): popisky, výchozí hodnoty starého configu, stav.
//   node --experimental-strip-types --test src/notes/folderlink.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_LINK, deleteSentence, directionOptions, dirtyLink, linkCfg, openablePath, stateText, stateTone, writeSentence } from "./folderlink.ts";

test("starý config bez notes.folderLink = vypnuto, oba směry, mazání se nepřenáší", () => {
  for (const notes of [undefined, {}, { folderLink: {} }, { folderLink: { direction: "nesmysl", path: 5 } }]) assert.deepEqual(linkCfg(notes), DEFAULT_LINK);
  assert.deepEqual(linkCfg({ folderLink: { enabled: true, path: "D:\\Vault", direction: "import", propagateDeletes: true } }), { enabled: true, path: "D:\\Vault", direction: "import", propagateDeletes: true });
});

test("věta o zápisu říká, co se kam zapisuje, pro každý směr jinak", () => {
  const dirs = directionOptions().map((o) => o.v);
  assert.deepEqual([...dirs].sort(), ["both", "export", "import"]);
  const s = Object.fromEntries(dirs.map((d) => [d, writeSentence(d)]));
  assert.equal(new Set(Object.values(s)).size, 3);
  assert.match(s.import, /Do složky se nezapíše nic/);
  assert.match(s.export, /nenačtou/);
  assert.match(s.both, /oběma směry/);
  assert.match(deleteSentence(false), /nepřenáší/);
  assert.match(deleteSentence(true), /Archivu/);
  for (const o of directionOptions()) assert.ok(o.label && o.hint);
});

test("stav: tečka a věta; bez snapshotu = vypnuto", () => {
  const rows = [[undefined, "off"], [{ state: "off" }, "off"], [{ state: "idle" }, "ok"], [{ state: "syncing" }, "warn"], [{ state: "error" }, "bad"]];
  for (const [v, tone] of rows) assert.equal(stateTone(v), tone);
  assert.match(stateText({ state: "error" }), /Nejde/);
});

test("rozpracovaná změna se pozná proti uloženému configu", () => {
  const base = JSON.stringify({ notes: { folderLink: { enabled: true, path: "D:\\V", direction: "both", propagateDeletes: false } } });
  assert.equal(dirtyLink(base, { enabled: true, path: "D:\\V", direction: "both", propagateDeletes: false }), false);
  assert.equal(dirtyLink(base, { enabled: true, path: "D:\\V", direction: "export", propagateDeletes: false }), true);
  assert.equal(dirtyLink(JSON.stringify({ notes: {} }), DEFAULT_LINK), false, "chybějící klíč = výchozí");
  assert.equal(dirtyLink("", DEFAULT_LINK), false);
});

test("kopii konfliktu jde otevřít jen když leží v Ledge", () => {
  assert.equal(openablePath({ ledge: "C:\\N\\_obecne\\a (konflikt Ledge 2026-10-24).md" }), "C:\\N\\_obecne\\a (konflikt Ledge 2026-10-24).md");
  assert.equal(openablePath({ ledge: null }), null);
});
