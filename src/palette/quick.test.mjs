// Rychlé odpovědi palety (`quick.ts`): co z dotazu vznikne — emoji režim, převod, výpočet, nic.
//   node --experimental-strip-types --test src/palette/quick.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { quick } from "./quick.ts";
import "../i18n/en-quick.ts";

test("směrování dotazu (tabulka)", () => {
  const cases = [
    ["12*(3+4)", "cs", { kind: "calc", title: "= 84", copy: "84" }],
    ["1,5*3", "cs", { kind: "calc", title: "= 4,5", copy: "4,5" }],
    ["1.5*3", "en", { kind: "calc", title: "= 4.5", copy: "4.5" }],
    ["15 % z 340", "cs", { kind: "calc", title: "= 51", copy: "51" }],
    ["10 km na mi", "cs", { kind: "convert", title: "10 km = 6,21371192 mi", copy: "6,21371192" }],
    ["3 GB to MB", "en", { kind: "convert", title: "3 GB = 3,000 MB", copy: "3000" }],
  ];
  for (const [q, lang, want] of cases) assert.deepEqual(quick(q, lang), want, `${q} (${lang})`);
});

test("velké číslo: zobrazení s oddělovačem, do schránky bez", () => {
  const r = quick("2^20", "cs");
  assert.equal(r.kind, "calc");
  assert.match(r.title, /^= 1\s048\s576$/u);
  assert.equal(r.copy, "1048576");
});

test("emoji režim nahradí běžný seznam; běžné hledání = nic", () => {
  const e = quick(":srdce", "cs");
  assert.equal(e.kind, "emoji");
  assert.equal(e.rows[0].item.emoji, "❤️");
  assert.equal(quick(":", "cs").rows.length, 60);
  for (const q of ["", "  ", "notes", "Spustit Ledge", "12", "km na mi", "settings"]) assert.equal(quick(q, "cs"), null, JSON.stringify(q));
});
