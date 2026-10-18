// Emoji v paletě (`emoji.ts` + tabulka `emojidata.ts`): tvar tabulky, anglická jména, hledání česky i
// anglicky bez diakritiky, prefix `:`.
//   node --experimental-strip-types --test src/palette/emoji.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { EMOJI } from "./emojidata.ts";
import { EMOJI_LIMIT, emojiItems, emojiLoaded, emojiQuery, loadEmoji, searchEmoji } from "./emoji.ts";
import { tIn } from "../i18n.ts";

// líná tabulka: před načtením prázdno, pak plná (statický import tabulky výš ji nenačte do `emoji.ts`)
assert.equal(emojiLoaded(), false);
assert.deepEqual(searchEmoji("srdce", "cs"), []);
const p = loadEmoji();
assert.equal(loadEmoji(), p, "jeden slib, jedno načtení");
await p;
assert.equal(emojiLoaded(), true);

const first = (text, lang = "cs") => searchEmoji(text, lang)[0]?.item.emoji;

test("tabulka: rozumná velikost, jedinečné emoji i klíče, tvar, en jméno, kw bez diakritiky", () => {
  assert.ok(EMOJI.length >= 300 && EMOJI.length <= 500, `${EMOJI.length} emoji`);
  const keys = new Set();
  const chars = new Set();
  for (const x of EMOJI) {
    assert.ok(!keys.has(x.key), `klíč ${x.key} dvakrát`);
    assert.ok(!chars.has(x.e), `emoji ${x.e} dvakrát`);
    keys.add(x.key);
    chars.add(x.e);
    assert.match(x.key, /^palette\.emoji\.[a-z][A-Za-z0-9]*$/);
    assert.ok(x.e.trim() && !/[a-z0-9]/i.test(x.e.replace(/\uFE0F|\u200D|\u20E3/g, "")), `${x.key}: není emoji`);
    assert.ok(x.cs.trim(), x.key);
    assert.ok(tIn("en", x.key, "\u0000") !== "\u0000", `${x.key}: chybí anglické jméno v en-emoji.ts`);
    assert.match(x.kw, /^[a-z0-9 ]*$/, `${x.key}: kw jen malá písmena bez diakritiky`);
  }
});

test("prefix : přepíná do emoji režimu", () => {
  assert.equal(emojiQuery(":srdce"), "srdce");
  assert.equal(emojiQuery("  :heart"), "heart");
  assert.equal(emojiQuery(":"), "");
  assert.equal(emojiQuery("srdce"), null);
  assert.equal(emojiQuery("12:30"), null);
});

test("hledání česky, anglicky, bez diakritiky, prefixem", () => {
  const cases = [
    ["srdce", "cs", "❤️"],
    ["heart", "en", "❤️"],
    ["heart", "cs", "❤️"],
    ["pal", "cs", "👍"],
    ["palec dolu", "cs", "👎"],
    ["thumbs up", "en", "👍"],
    ["smich", "cs", "😂"],
    ["kocka", "cs", "🐱"],
    ["kočka", "cs", "🐱"],
    ["pivo", "cs", "🍺"],
    ["rocket", "en", "🚀"],
    ["ohen", "cs", "🔥"],
    ["vlajka cesk", "cs", "🇨🇿"],
    ["hotovo", "cs", "✅"],
  ];
  for (const [q, lang, want] of cases) assert.equal(first(q, lang), want, `:${q} (${lang}) → ${first(q, lang)}`);
  assert.deepEqual(searchEmoji("xyzqwv", "cs"), []);
});

test("bez dotazu: nejpoužívanější v pořadí tabulky, nejvýš limit; jméno v jazyce UI", () => {
  const rows = searchEmoji("", "cs");
  assert.equal(rows.length, EMOJI_LIMIT);
  assert.deepEqual(rows.slice(0, 3).map((r) => r.item.emoji), EMOJI.slice(0, 3).map((x) => x.e));
  const heart = EMOJI.find((x) => x.e === "❤️");
  assert.equal(emojiItems("cs").find((i) => i.emoji === "❤️").title, heart.cs);
  assert.equal(emojiItems("en").find((i) => i.emoji === "❤️").title, "heart");
  // zvýraznění sedí na jméno v jazyce UI
  const r = searchEmoji("srdce", "cs")[0];
  assert.deepEqual(r.ranges, [[0, 5]]);
});
