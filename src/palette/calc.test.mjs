// Kalkulačka palety (`calc.ts`): vlastní parser bez eval, procenta, funkce, desetinná čárka i tečka,
// formát podle jazyka bez float šumu.
//   node --experimental-strip-types --test src/palette/calc.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { evaluate, formatNum, lex } from "./calc.ts";

const close = (a, b) => Math.abs(a - b) < 1e-9 * Math.max(1, Math.abs(b));

test("výrazy → hodnota (tabulka)", () => {
  const cases = [
    ["12*(3+4)", 84],
    ["2^10", 1024],
    ["2**10", 1024],
    ["2^3^2", 512],
    ["-2^2", -4],
    ["2^-1", 0.5],
    ["1+2*3", 7],
    ["(1+2)*3", 9],
    ["10/4", 2.5],
    ["10:4", 2.5],
    ["6÷3", 2],
    ["3×4", 12],
    ["2x3", 6],
    ["7 − 2", 5],
    ["1,5 + 1,5", 3],
    ["1.5*2", 3],
    [",5+.5", 1],
    [" 12 * 3 ", 36],
    ["1 000 000 * 2", 2000000],
    ["15 % z 340", 51],
    ["15% of 340", 51],
    ["15 % ze 340", 51],
    ["200 + 10%", 220],
    ["200 - 10%", 180],
    ["50 * 10%", 5],
    ["10%", 0.1],
    ["sqrt(2)", Math.SQRT2],
    ["sqrt 16 + 9", 13],
    ["√9", 3],
    ["sqrt(2)^2", 2],
    ["abs(-3)", 3],
    ["round(2.5)", 3],
    ["floor(2.7)+ceil(2.1)", 5],
    ["log(1000)", 3],
    ["log2(8)", 3],
    ["ln(e)", 1],
    ["exp(0)", 1],
    ["2*pi", 2 * Math.PI],
    ["π*1", Math.PI],
    ["sin(pi)", 0],
    ["cos(0)", 1],
    ["0.1+0.2-0.3", 0],
    ["3 - 3", 0],
  ];
  for (const [src, want] of cases) {
    const got = evaluate(src);
    assert.ok(got !== null && close(got, want), `${src} → ${got}, čekáno ${want}`);
  }
});

test("nevalidní / neúplné / bez operace → null (žádný řádek)", () => {
  for (const src of ["", " ", "12", "-5", "+3", "pi", "e", "1.", "12*(3+4", "12*", "*3", "()", "2 3", "1/0", "sqrt(-1)", "abc", "foo(2)", "2 km", "3 GB v MB", "15 % z", "10 % of", "2(3)", "1;2", "notes", "2e3", "1..2"]) {
    assert.equal(evaluate(src), null, JSON.stringify(src));
  }
});

test("žádné eval ani Function ve zdroji", async () => {
  const { readFileSync } = await import("node:fs");
  const src = readFileSync(new URL("./calc.ts", import.meta.url), "utf8").replace(/\/\/.*$/gm, "");
  assert.doesNotMatch(src, /\beval\s*\(|new\s+Function|Function\s*\(/);
});

test("lexer: čárka i tečka, unicode operátory, neznámý znak = null", () => {
  assert.deepEqual(lex("1,5×2"), [{ k: "num", v: 1.5 }, { k: "op", v: "*" }, { k: "num", v: 2 }]);
  assert.deepEqual(lex("2**3").map((t) => t.v), [2, "^", 3]);
  assert.equal(lex("2 # 3"), null);
  assert.equal(lex("1 ; 2"), null);
});

test("formát podle jazyka, bez šumu, vědecky jen extrémy", () => {
  const cases = [
    [84, "cs", {}, "84"],
    [2.5, "cs", {}, "2,5"],
    [2.5, "en", {}, "2.5"],
    [0.1 + 0.2, "cs", {}, "0,3"],
    [1 / 3, "en", {}, "0.333333333333"],
    [-1.25, "cs", {}, "-1,25"],
    [1048576, "en", {}, "1048576"],
    [1048576, "en", { group: true }, "1,048,576"],
    [6.2137119224, "cs", { sig: 9 }, "6,21371192"],
    [0, "cs", {}, "0"],
    [-0, "en", {}, "0"],
    [1e20, "en", {}, "1E20"],
    [1.5e-12, "cs", {}, "1,5E-12"],
  ];
  for (const [n, lang, opts, want] of cases) assert.equal(formatNum(n, lang, opts), want, `${n} ${lang} ${JSON.stringify(opts)}`);
  // cs oddělovač tisíců je mezera (tvrdá), ne tečka
  assert.match(formatNum(1048576, "cs", { group: true }), /^1\s048\s576$/u);
});
