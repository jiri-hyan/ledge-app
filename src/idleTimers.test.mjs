// Hlídač „v klidu nic netikne“ (bez DOM): `npm run test:ui`. Rozpočet Ledge je idle CPU < 0,1 % a se zavřeným
// šuplíkem se nic nepočítá — proto smí `setInterval` ve zdrojích existovat jen na seznamu níže a ten, který
// kreslí hodiny v hlavičce (`App.tsx`), musí vzniknout až uvnitř efektu, který se při skrytém šuplíku vrátí
// dřív, než časovač založí. Nový periodický časovač = nové rozhodnutí tady (a důvod, proč neběží v klidu).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const SRC = fileURLToPath(new URL(".", import.meta.url));

// soubor → proč smí mít setInterval
const ALLOWED = {
  "App.tsx": "hodiny v hlavičce; vzniká jen při viditelném šuplíku (efekt s `view() === \"hidden\"` → return)",
  "questions/QuestionCard.tsx": "odpočet otázky Clauda; komponenta existuje jen, dokud je otázka na kartě",
};

function walk(dir) {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
}

const posix = (p) => relative(SRC, p).split(sep).join("/");
const sources = walk(SRC).filter((p) => /\.(ts|tsx)$/.test(p) && !/\.test\./.test(p));

test("setInterval jen na seznamu povolených míst", () => {
  const found = sources.filter((p) => /\bsetInterval\s*\(/.test(readFileSync(p, "utf8"))).map(posix).sort();
  assert.deepEqual(found, Object.keys(ALLOWED).sort(), "nový nebo zmizelý setInterval — zkontroluj idle CPU a uprav seznam");
});

test("hodiny v App.tsx tikají jen při viditelném šuplíku", () => {
  const src = readFileSync(join(SRC, "App.tsx"), "utf8");
  const effect = /createEffect\(\(\) => \{\s*if \(view\(\) === "hidden"\) return;[\s\S]*?setInterval\([\s\S]*?clearInterval/.exec(src);
  assert.ok(effect, "setInterval v App.tsx musí být v efektu, který se při skrytém šuplíku vrátí před založením časovače");
  assert.equal(src.match(/setInterval\s*\(/g).length, 1, "v App.tsx je jediný časovač");
});
