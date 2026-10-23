// Tabulka kláves a gest (`shortcuts.ts`): každý záznam má češtinu i angličtinu (slovník `i18n/en-keys.ts`),
// klíče jsou jedinečné, každá skupina má záznam a žádný záznam nemá prázdnou větu.
//   node --experimental-strip-types --test src/ui/shortcuts.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { SHORTCUTS, SHORTCUT_GROUPS } from "./shortcuts.ts";

const en = readFileSync(new URL("../i18n/en-keys.ts", import.meta.url), "utf8");
const enOf = (key) => new RegExp(`^\\s*"${key.replaceAll(".", "\\.")}": "(.+)",$`, "m").exec(en)?.[1];

test("každý záznam a skupina má češtinu i angličtinu", () => {
  for (const s of SHORTCUTS) {
    assert.ok(s.cs.trim().length > 10, `${s.key}: česká věta`);
    assert.ok(enOf(s.key), `${s.key}: chybí anglický záznam v i18n/en-keys.ts`);
  }
  for (const g of SHORTCUT_GROUPS) {
    assert.ok(g.cs && enOf(g.key), `${g.key}: skupina bez cs/en`);
  }
});

test("klíče jsou jedinečné, skupiny neprázdné a známé", () => {
  const keys = SHORTCUTS.map((s) => s.key);
  assert.equal(new Set(keys).size, keys.length, "duplicitní klíč");
  const ids = new Set(SHORTCUT_GROUPS.map((g) => g.id));
  for (const s of SHORTCUTS) assert.ok(ids.has(s.group), `${s.key}: neznámá skupina ${s.group}`);
  for (const g of SHORTCUT_GROUPS) assert.ok(SHORTCUTS.some((s) => s.group === g.id), `skupina ${g.id} je prázdná`);
});

test("klávesy bez mezer navíc a bez češtiny (stejné v obou jazycích)", () => {
  for (const s of SHORTCUTS) assert.equal(s.keys, s.keys.trim(), `${s.key}: keys`);
  for (const s of SHORTCUTS.filter((x) => x.keys)) assert.ok(!/[áčďéěíňóřšťúůýž]/i.test(s.keys), `${s.key}: keys musí být bez češtiny`);
});
