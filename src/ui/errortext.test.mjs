// Tabulkový test pojistky proti technickým chybám (`errortext.ts`).
//   node --experimental-strip-types --test src/ui/errortext.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { humanError, looksTechnical } from "./errortext.ts";

test("co vypadá technicky", () => {
  const cases = [
    ["Only one usage of each socket address (os error 10048)", true],
    ["Přístup byl odepřen. (os error 5)", true],
    ["Access is denied. (0x80070005)", true],
    ["thread 'main' panicked at src/main.rs:12:5", true],
    ["stack backtrace:\n   0: foo", true],
    ["TypeError: x is undefined", true],
    ['{"error":"x","code":5}', true],
    ['[{"a": 1}]', true],
    ["   at foo (app.js:10:5)\n   at bar (app.js:20:1)", true],
    ["Poznámku nejde uložit: soubor právě používá jiná aplikace. Zavři ji a zkus to znovu.", false],
    ["Port 5173 už drží jiný proces.", false],
    ["[Ledge] hotovo", false],
    ["", false],
  ];
  for (const [msg, want] of cases) assert.equal(looksTechnical(msg), want, msg);
});

test("humanError: prázdné, technické a lidské", () => {
  const g = "Něco se nepovedlo.";
  const t = "Něco se nepovedlo — podrobnosti jsou v logu.";
  assert.equal(humanError("", g, t), g);
  assert.equal(humanError(undefined, g, t), g);
  assert.equal(humanError(new Error("Error: Složka neexistuje."), g, t), "Složka neexistuje.");
  assert.equal(humanError(new Error("bind failed (os error 10048)"), g, t), t);
  assert.equal(humanError('{"message":"x"}', g, t), t);
  assert.equal(humanError("Soubor nejde uložit.", g, t), "Soubor nejde uložit.");
});
