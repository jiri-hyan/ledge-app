// Akce nad poznámkou (`lifeactions.ts`): které nabídnout, archivní cesta, zkrácený titulek.
import { test } from "node:test";
import assert from "node:assert/strict";
import { isArchivedPath, lifeActions, shortTitle } from "./lifeactions.ts";

test("nabídka akcí podle stavu", () => {
  const cases = [
    [false, false, ["done", "archive", "delete"]],
    [true, false, ["undone", "archive", "delete"]],
    [false, true, ["unarchive", "delete"]],
    [true, true, ["unarchive", "delete"]],
  ];
  for (const [done, archived, want] of cases) assert.deepEqual(lifeActions(done, archived), want, `${done} ${archived}`);
});

test("cesta v archivu se pozná jen podle poslední složky", () => {
  const cases = [
    ["C:\\n\\app\\_archiv\\a.md", true],
    ["C:/n/app/_archiv/a.md", true],
    ["C:\\n\\app\\_ARCHIV\\a.md", true],
    ["C:\\n\\app\\a.md", false],
    ["C:\\n\\_archiv\\app\\a.md", false],
    ["C:\\n\\app\\_archiv-stary\\a.md", false],
  ];
  for (const [p, want] of cases) assert.equal(isArchivedPath(p), want, p);
});

test("titulek do lišty", () => {
  assert.equal(shortTitle("  Zavolat\n  Řehořovi  "), "Zavolat Řehořovi");
  assert.equal(shortTitle("x".repeat(50)), `${"x".repeat(35)}…`);
  assert.equal(shortTitle(""), "");
});
