// Tabulkový test pojistky proti technickým chybám (`errortext.ts`).
//   node --experimental-strip-types --test src/ui/errortext.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
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

// Pojistka: chyby jdou přes `sayError`/`errorText` (humanError), ne `say(String(e), true)`.
test("v src není say(String(e)", () => {
  const bad = [];
  const walk = (d) => {
    for (const x of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, x.name);
      if (x.isDirectory()) walk(p);
      else if (/\.tsx?$/.test(x.name) && /\bsay\(\s*String\(/.test(readFileSync(p, "utf8"))) bad.push(p);
    }
  };
  walk(fileURLToPath(new URL("..", import.meta.url)));
  assert.deepEqual(bad, [], "místo say(String(e), true) použij sayError(e)");
});

// Pojistka 2: chyba nejde do UI jako `String(e)` (obchází `looksTechnical`); povoleno jen v `errorline.ts` (zápis do logu).
test("v src není String(e) mimo errorline.ts", () => {
  const bad = [];
  const walk = (d) => {
    for (const x of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, x.name);
      if (x.isDirectory()) walk(p);
      else if (/\.tsx?$/.test(x.name) && x.name !== "errorline.ts" && /\bString\(\s*(e|err|error)\s*\)/.test(readFileSync(p, "utf8"))) bad.push(p);
    }
  };
  walk(fileURLToPath(new URL("..", import.meta.url)));
  assert.deepEqual(bad, [], "místo String(e) použij errorText(e) z ui/toast");
});
