// Nejmenší písmo je 11 px: žádné `font` / `font-size` s natvrdo menším px v CSS (použij `var(--fs-2xs)` = 11 px × --fz).
//   node --experimental-strip-types --test src/ui/fontmin.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const css = (dir) =>
  readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? css(p) : n.endsWith(".css") ? [p] : [];
  });

test("žádné písmo menší než 11 px v CSS", () => {
  const bad = [];
  for (const f of css(root)) {
    readFileSync(f, "utf8").split("\n").forEach((line, i) => {
      for (const m of line.matchAll(/(?:^|[;{\s])(?:font|font-size)\s*:\s*([^;}]*)/g)) {
        if (/max\(var\(--fs-/.test(m[1])) continue; // spodní mez tokenem
        for (const px of m[1].matchAll(/(\d+(?:\.\d+)?)px/g)) {
          // `/ 1.2` řádkování a rozměry za lomítkem nejsou velikost písma; bereme jen první číslo před lomítkem
          if (Number(px[1]) < 11 && !m[1].slice(0, px.index).includes("/")) bad.push(`${f.slice(root.length)}:${i + 1} ${m[0].trim().slice(0, 70)}`);
        }
      }
    });
  }
  assert.deepEqual(bad, [], "použij var(--fs-2xs) (11 px) nebo větší token");
});
