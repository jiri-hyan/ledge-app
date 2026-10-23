// Tabulkové testy čtení viditelnosti (`sections.ts`): výchozí stav, starý snapshot bez nových polí,
// shoda klíčů s Rustem a s modelem rozložení (každé pole je karta nebo její část).
//   node --experimental-strip-types --test src/cards/sections.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { CLAUDE_PARTS, SECTION_KEYS, allSectionsOn, sectionOn } from "./sections.ts";
import { CARD_IDS, PARTS } from "../layout/model.ts";

test("výchozí: všechno zapnuté, chybějící pole = zapnuto", () => {
  const all = allSectionsOn();
  assert.deepEqual(Object.keys(all), [...SECTION_KEYS]);
  assert.ok(Object.values(all).every((v) => v === true));
  const cases = [
    // [sections, klíč, zapnuto]
    [undefined, "repos", true],
    [null, "weather", true],
    [{}, "claudeCosts", true],
    // starý snapshot / config jen se 4 poli
    [{ projects: true, running: false, claude: true, notes: true }, "running", false],
    [{ projects: true, running: false, claude: true, notes: true }, "shelf", true],
    [{ repos: false }, "repos", false],
    // jen výslovné false skrývá
    [{ clock: 0 }, "clock", true],
    [{ clock: null }, "clock", true],
  ];
  for (const [sec, k, want] of cases) assert.equal(sectionOn(sec, k), want, `${JSON.stringify(sec)} ${k}`);
});

test("klíče se shodují s Rustem (`config::Sections` v config/sections.rs, camelCase)", () => {
  const src = readFileSync(new URL("../../src-tauri/src/config/sections.rs", import.meta.url), "utf8");
  const body = src.slice(src.indexOf("sections! {"), src.indexOf("}", src.indexOf("sections! {")));
  const rust = [...body.matchAll(/^\s*([a-z_]+)(?: = (?:true|false))?,\s*$/gm)].map((m) => m[1].replace(/_([a-z])/g, (_, c) => c.toUpperCase()));
  assert.deepEqual(rust, [...SECTION_KEYS]);
});

test("každé pole je karta rozložení nebo její část (odvozené z layout)", () => {
  for (const k of SECTION_KEYS) assert.ok(CARD_IDS.includes(k) || k in PARTS, k);
  for (const k of CLAUDE_PARTS) assert.equal(PARTS[k], "claude", k);
});
