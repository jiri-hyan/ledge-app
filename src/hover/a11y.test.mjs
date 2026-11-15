// Přístupnost docku a ostrůvku bez DOM: každé <button> v hover/*.tsx má název (aria-label, nebo text z t()),
// a nikde není aria-pressed spolu se zjevně měnícím se aria-label (ARIA: buď stálý popisek + pressed, nebo měnící se bez pressed).
//   node --experimental-strip-types --test src/hover/a11y.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const DIR = fileURLToPath(new URL(".", import.meta.url));
const FILES = readdirSync(DIR).filter((f) => f.endsWith(".tsx"));
const buttons = (src) => [...src.matchAll(/<button\b([^>]*(?:\{[^}]*\}[^>]*)*)>([\s\S]*?)<\/button>/g)].map((m) => ({ attrs: m[1], body: m[2] }));

test("každé tlačítko v hover/ má název", () => {
  const bad = [];
  for (const f of FILES) {
    for (const b of buttons(readFileSync(join(DIR, f), "utf8"))) {
      if (!/aria-label=/.test(b.attrs) && !/\bt\(/.test(b.body) && !/\{label\}|\{more\(/.test(b.body)) bad.push(`${f}: ${b.attrs.trim().slice(0, 60)} → ${b.body.trim().slice(0, 30)}`);
    }
  }
  assert.deepEqual(bad, []);
});

test("aria-pressed se stálým popiskem (aria-label bez ternáru)", () => {
  const bad = [];
  for (const f of FILES) {
    for (const b of buttons(readFileSync(join(DIR, f), "utf8"))) {
      const label = b.attrs.match(/aria-label=\{([^]*?)\}(?=\s|$)/)?.[1] ?? "";
      if (/aria-pressed=/.test(b.attrs) && /\?/.test(label)) bad.push(`${f}: ${label.slice(0, 60)}`);
    }
  }
  assert.deepEqual(bad, []);
});
