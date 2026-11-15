// Enter v potvrzení akce v paletě (`confirmkey.ts`): žádný auto-repeat, žádná odpověď dřív než po ~400 ms.
//   node --experimental-strip-types --test src/palette/confirmkey.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CONFIRM_MIN_MS, confirmEnterOk } from "./confirmkey.ts";

test("Enter potvrdí jen záměrně: bez repeatu a po minimální prodlevě", () => {
  const cases = [
    [1000, 1000, false, false], // dvojitý Enter ve stejný okamžik
    [1000, 1000 + CONFIRM_MIN_MS - 1, false, false], // těsně před prodlevou
    [1000, 1000 + CONFIRM_MIN_MS, false, true], // přesně po prodlevě
    [1000, 5000, false, true], // normální potvrzení
    [1000, 5000, true, false], // držený Enter (auto-repeat) nikdy
    [1000, 1000, true, false],
    [1000, 900, false, false], // hodiny skočily zpět
  ];
  for (const [shown, now, repeat, want] of cases) assert.equal(confirmEnterOk(shown, now, repeat), want, `${shown} ${now} ${repeat}`);
});

test("vlastní minimum", () => {
  assert.equal(confirmEnterOk(0, 50, false, 100), false);
  assert.equal(confirmEnterOk(0, 100, false, 100), true);
});
