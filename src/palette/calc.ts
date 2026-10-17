// Kalkulačka v paletě (Ctrl+K): výraz → číslo. Vlastní rekurzivní sestup nad tokeny — žádné
// `eval` ani `Function`. Umí + − × ÷ (i `*`, `/`, `:`, `x`), mocninu `^`/`**`, závorky, procenta
// (`200 + 10%` = 220, `15 % z 340` / `15% of 340`, samotné `10%` = 0,1), funkce sqrt/√, abs, round,
// floor, ceil, ln, log, log2, exp, sin/cos/tan (radiány) a konstanty pi/π, e. Desetinná čárka i tečka,
// mezery mezi tokeny, mezera jako oddělovač tisíců (`1 000 000`). Nevalidní nebo neúplný výraz a
// samotné číslo bez operace = `null` (paleta pak žádný řádek neukáže). Čistá logika bez DOM —
// `calc.test.mjs` (`node --experimental-strip-types --test src/palette/calc.test.mjs`).
import { localeOf, type Lang } from "../i18n.ts";

type Tok = { k: "num"; v: number } | { k: "op"; v: string } | { k: "id"; v: string };

const OPS: Record<string, string> = { "+": "+", "-": "-", "−": "-", "*": "*", "×": "*", "·": "*", "/": "/", "÷": "/", ":": "/", "^": "^", "(": "(", ")": ")", "%": "%" };
const FUNCS: Record<string, (x: number) => number> = {
  sqrt: Math.sqrt,
  abs: Math.abs,
  round: Math.round,
  floor: Math.floor,
  ceil: Math.ceil,
  ln: Math.log,
  log: Math.log10,
  log2: Math.log2,
  exp: Math.exp,
  // goniometrie: šum kolem nuly (sin π = 1e-16) srovnat na 0
  sin: (x) => trig(Math.sin(x)),
  cos: (x) => trig(Math.cos(x)),
  tan: (x) => trig(Math.tan(x)),
};
const CONSTS: Record<string, number> = { pi: Math.PI, e: Math.E };
const OF = new Set(["z", "ze", "of"]);
const trig = (x: number) => (Math.abs(x) < 1e-14 ? 0 : x);

/** Výraz → tokeny; neznámý znak = `null`. */
export function lex(src: string): Tok[] | null {
  // tvrdé mezery jako obyčejné; mezera mezi skupinami po třech číslicích = oddělovač tisíců
  const s = src.replace(/[  ]/g, " ").replace(/(\d) (?=\d{3}(?!\d))/g, "$1");
  const out: Tok[] = [];
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    if (c === " " || c === "\t") {
      i++;
      continue;
    }
    const num = /^(\d+(?:[.,]\d*)?|[.,]\d+)/.exec(s.slice(i));
    if (num) {
      out.push({ k: "num", v: Number(num[1].replace(",", ".")) });
      i += num[1].length;
      continue;
    }
    if (c === "*" && s[i + 1] === "*") {
      out.push({ k: "op", v: "^" });
      i += 2;
      continue;
    }
    if (c in OPS) {
      out.push({ k: "op", v: OPS[c] });
      i++;
      continue;
    }
    if (c === "π") {
      out.push({ k: "id", v: "pi" });
      i++;
      continue;
    }
    if (c === "√") {
      out.push({ k: "id", v: "sqrt" });
      i++;
      continue;
    }
    // „2x3“, „2 x 3“: x bez dalšího písmene je krát
    if ((c === "x" || c === "X") && !/[a-z]/i.test(s[i + 1] ?? "")) {
      out.push({ k: "op", v: "*" });
      i++;
      continue;
    }
    const id = /^[a-z][a-z0-9]*/i.exec(s.slice(i));
    if (id) {
      out.push({ k: "id", v: id[0].toLowerCase() });
      i += id[0].length;
      continue;
    }
    return null;
  }
  return out;
}

class Bad extends Error {}
type Val = { v: number; pct: boolean };

/** Součet / rozdíl se srovnaným šumem při odečtení skoro stejných čísel (0,1 + 0,2 − 0,3 = 0). */
function addSnap(a: number, b: number): number {
  const r = a + b;
  return Math.abs(r) < Math.max(Math.abs(a), Math.abs(b)) * 1e-13 ? 0 : r;
}

/** Hodnota výrazu, nebo `null` (nevalidní, neúplný, bez operace, nekonečno). */
export function evaluate(src: string): number | null {
  const lexed = lex(src);
  if (!lexed || !lexed.length) return null;
  const toks: Tok[] = lexed;
  let i = 0;
  let ops = 0;
  const isOp = (v: string) => {
    const t = toks[i];
    return t?.k === "op" && t.v === v;
  };
  const isOf = () => {
    const t = toks[i];
    return t?.k === "id" && OF.has(t.v);
  };

  function expr(): number {
    let a = term().v;
    while (isOp("+") || isOp("-")) {
      const minus = toks[i++].v === "-";
      ops++;
      const t = term();
      // `200 + 10%` = 200 + 10 % z 200
      const d = t.pct ? a * t.v : t.v;
      a = addSnap(a, minus ? -d : d);
    }
    return a;
  }
  function term(): Val {
    let a = unary();
    while (isOp("*") || isOp("/")) {
      const div = toks[i++].v === "/";
      ops++;
      const b = unary().v;
      if (div && b === 0) throw new Bad();
      a = { v: div ? a.v / b : a.v * b, pct: false };
    }
    return a;
  }
  function unary(): Val {
    if (isOp("-")) {
      i++;
      const x = unary();
      return { v: -x.v, pct: x.pct };
    }
    if (isOp("+")) {
      i++;
      return unary();
    }
    return power();
  }
  function power(): Val {
    const base = postfix();
    if (!isOp("^")) return base;
    i++;
    ops++;
    return { v: base.v ** unary().v, pct: false };
  }
  function postfix(): Val {
    const v = primary();
    if (!isOp("%")) return { v, pct: false };
    i++;
    ops++;
    if (isOf()) {
      i++;
      return { v: (v / 100) * unary().v, pct: false };
    }
    return { v: v / 100, pct: true };
  }
  function primary(): number {
    const t = toks[i];
    if (!t) throw new Bad();
    if (t.k === "num") {
      i++;
      return t.v;
    }
    if (t.k === "op" && t.v === "(") {
      i++;
      const v = expr();
      if (!isOp(")")) throw new Bad();
      i++;
      return v;
    }
    if (t.k === "id") {
      if (t.v in CONSTS) {
        i++;
        return CONSTS[t.v];
      }
      const f = FUNCS[t.v];
      if (f) {
        i++;
        ops++;
        return f(primary());
      }
    }
    throw new Bad();
  }

  try {
    const v = expr();
    if (i !== toks.length || ops === 0 || !Number.isFinite(v)) return null;
    return Object.is(v, -0) ? 0 : v;
  } catch (e) {
    if (e instanceof Bad) return null;
    throw e;
  }
}

/**
 * Číslo podle jazyka UI: cs desetinná čárka, en tečka; `sig` platných číslic (zahodí float šum jako
 * 0,30000000000000004), velmi velká / malá čísla vědecky. `group` = oddělovač tisíců (zobrazení);
 * bez něj se text hodí do schránky (vloží se do tabulky i kalkulačky). Mínus vždy ASCII.
 */
export function formatNum(n: number, lang: Lang, opts: { sig?: number; group?: boolean } = {}): string {
  const sig = opts.sig ?? 12;
  if (!Number.isFinite(n)) return "";
  const r = Number(n.toPrecision(sig));
  const a = Math.abs(r);
  const sci = a !== 0 && (a >= 1e15 || a < 1e-9);
  const f = new Intl.NumberFormat(localeOf(lang), {
    maximumSignificantDigits: sig,
    useGrouping: !sci && opts.group === true,
    ...(sci ? { notation: "scientific" as const } : {}),
  });
  return f.format(r === 0 ? 0 : r).replace(/−/g, "-");
}
