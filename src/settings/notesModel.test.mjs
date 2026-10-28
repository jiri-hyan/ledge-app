import test from "node:test";
import assert from "node:assert/strict";
import { kindIdLocked } from "./notesModel.ts";

test("zámek Id kategorie: vygenerované otevřené, vlastní zamčené", () => {
  const cases = [
    ["kategorie-1", false],
    ["kategorie-12", false],
    ["napad", true],
    ["kategorie-", true],
    ["kategorie-1a", true],
    ["", true],
  ];
  for (const [id, want] of cases) assert.equal(kindIdLocked(id), want, id);
});
