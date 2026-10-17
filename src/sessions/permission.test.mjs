// Tabulkové testy čisté logiky doplňků schválení Clauda (`permission.ts`): co UI pošle do `decide`.
//   node --experimental-strip-types --test src/sessions/permission.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_DEST, DESTS, REASON_MAX, alwaysExtra, denyExtra, destFile, reasonKey, reasonOf } from "./permission.ts";

test("výchozí cíl = projekt lokálně, tři cíle v pořadí", () => {
  assert.equal(DEFAULT_DEST, "localSettings");
  assert.deepEqual(DESTS, ["localSettings", "userSettings", "session"]);
});

test("cílový soubor podle dokumentace hooků", () => {
  for (const [d, f] of [
    ["localSettings", ".claude/settings.local.json"],
    ["userSettings", "~/.claude/settings.json"],
    ["session", null],
  ]) assert.equal(destFile(d), f, d);
});

test("důvod: ořez, strop, prázdný = obyčejné zamítnutí", () => {
  for (const [inp, want] of [
    ["", null],
    ["   \n ", null],
    ["  použij pnpm \n", "použij pnpm"],
    ["ž".repeat(REASON_MAX + 50), "ž".repeat(REASON_MAX)],
    ["😀".repeat(REASON_MAX + 1), "😀".repeat(REASON_MAX)],
  ]) assert.equal(reasonOf(inp), want, JSON.stringify(inp).slice(0, 30));
});

test("doplněk decide: zamítnutí bez důvodu jako dřív, s důvodem reason, vždy = jen cíl", () => {
  assert.equal(denyExtra("  "), undefined);
  assert.deepEqual(denyExtra(" jinak "), { reason: "jinak" });
  assert.deepEqual(alwaysExtra("session"), { always: "session" });
});

test("klávesy v poli důvodu", () => {
  for (const [key, shift, text, want] of [
    ["Enter", false, "důvod", "send"],
    ["Enter", false, "  ", null],
    ["Enter", true, "důvod", null],
    ["Escape", false, "", "cancel"],
    ["a", false, "důvod", null],
  ]) assert.equal(reasonKey(key, shift, text), want, `${key} ${shift} ${text}`);
});
