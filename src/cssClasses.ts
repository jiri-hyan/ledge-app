// Čistá analýza globálních CSS tříd (bez DOM, bez závislostí). Všechny `src/**/*.css` jsou globální —
// třída „vlastněná“ jedním souborem (stojí na začátku selektoru sama) se nesmí objevit v jiném souboru,
// jinak se pravidla obou oblastí potkají na jednom prvku (kolo 3: `.st-item.proj` v Nastavení dostal
// `flex-direction: column` z dlaždice `.proj` hlavní stránky). Hlídá `cssClasses.test.mjs` (npm run test:ui).

export interface CssRule { selector: string; body: string }
export interface CssFile { file: string; css: string }
export interface Collision { cls: string; owners: string[]; files: string[] }
/** Záměrně sdílená třída: soubory, které ji smějí zmínit (vlastník + rozšíření), a proč. */
export interface Shared { files: string[]; why: string }

const stripComments = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, "");

/** Index párové `}` k `{` na pozici `open` (řetězce v CSS tokenech nejsou, stačí hloubka). */
function closing(css: string, open: number): number {
  let depth = 0;
  for (let i = open; i < css.length; i++) {
    if (css[i] === "{") depth++;
    else if (css[i] === "}" && --depth === 0) return i;
  }
  return css.length;
}

/** Pravidla stylu; `@media`/`@supports`/`@container`/`@layer` se rozbalí, ostatní at-pravidla (keyframes) přeskočí. */
export function cssRules(css: string): CssRule[] {
  const src = stripComments(css);
  const out: CssRule[] = [];
  let i = 0;
  while (i < src.length) {
    const open = src.indexOf("{", i);
    if (open < 0) break;
    const prelude = src.slice(i, open).replace(/^[\s;]+/, "").trim();
    const end = closing(src, open);
    const body = src.slice(open + 1, end);
    if (/^@(media|supports|container|layer)\b/.test(prelude)) out.push(...cssRules(body));
    else if (!prelude.startsWith("@")) for (const s of splitTop(prelude, ",")) out.push({ selector: s.trim(), body: body.trim() });
    i = end + 1;
  }
  return out;
}

/** Rozdělí podle `sep` mimo závorky (`:is(.a, .b)` zůstane celé). */
function splitTop(s: string, sep: string): string[] {
  const parts: string[] = [];
  let depth = 0, from = 0;
  for (let i = 0; i < s.length; i++) {
    if (s[i] === "(") depth++;
    else if (s[i] === ")") depth--;
    else if (s[i] === sep && depth === 0) { parts.push(s.slice(from, i)); from = i + 1; }
  }
  parts.push(s.slice(from));
  return parts.filter((p) => p.trim());
}

const noAttrs = (sel: string) => sel.replace(/\[[^\]]*\]/g, "");
const CLASS = /\.(-?[_a-zA-Z][\w-]*)/g;

/** Všechny třídy, které selektor zmiňuje (i uvnitř `:is()`/`:has()`/`:not()`). */
export function classesIn(selector: string): string[] {
  return [...new Set([...noAttrs(selector).matchAll(CLASS)].map((m) => m[1]))];
}

/** Třída, kterou selektor „vlastní“: jediná třída prvního složeného selektoru, který nějakou třídu má
 *  (`.proj`, `.proj:hover`, `html[x] .shell`, `.st-tabs button`). `.a.b` = modifikace → nic. */
export function ownedClass(selector: string): string | null {
  let s = noAttrs(selector);
  for (let prev = ""; prev !== s;) { prev = s; s = s.replace(/:[\w-]+\([^()]*\)/g, ""); }
  s = s.replace(/::?[\w-]+/g, "");
  for (const compound of s.trim().split(/\s*[>+~]\s*|\s+/)) {
    const cls = [...compound.matchAll(CLASS)].map((m) => m[1]);
    if (cls.length) return cls.length === 1 ? cls[0] : null;
  }
  return null;
}

/** Kolize: třídu vlastní některý soubor a zmiňují ji aspoň dva soubory. Třída z `allow` je kolize jen tehdy,
 *  když ji zmíní soubor mimo `allow[cls].files` (nový sdílející soubor musí do allowlistu s důvodem). */
export function collisions(files: CssFile[], allow: Record<string, Shared> = {}): Collision[] {
  const owners = new Map<string, Set<string>>();
  const mentions = new Map<string, Set<string>>();
  const add = (m: Map<string, Set<string>>, k: string, f: string) => (m.get(k) ?? m.set(k, new Set()).get(k)!).add(f);
  for (const { file, css } of files) {
    for (const r of cssRules(css)) {
      for (const c of classesIn(r.selector)) add(mentions, c, file);
      const own = ownedClass(r.selector);
      if (own) add(owners, own, file);
    }
  }
  const out: Collision[] = [];
  for (const [cls, own] of owners) {
    const all = mentions.get(cls)!;
    const ok = allow[cls]?.files;
    if (all.size < 2 || (ok && [...all].every((f) => ok.includes(f)))) continue;
    out.push({ cls, owners: [...own].sort(), files: [...all].sort() });
  }
  return out.sort((a, b) => a.cls.localeCompare(b.cls));
}

// ── Použité třídy (TSX/TS) vs. definované (CSS) ─────────────────────────────────────────────────────────
// v0.2.7: `.opt` (řádek volby) zmizela ze settings.css při přepisu Nastavení, TSX ji dál přiděloval a nic to
// nechytilo. `usedClasses` čte místa, kde kód třídu PŘIDĚLUJE (`class`/`className` atribut i vlastnost, klíče
// `classList={{…}}`, `classList.add/toggle/replace`); `missingClasses` vrátí ty, které žádný CSS selektor
// nezmiňuje. Název skládaný za běhu (`pill ${x}`, `"st-" + x`, `p.class`) zkontrolovat nejde → `dynamic`.
// Selektory v `querySelector`/`closest`/`classList.contains` třídu jen čtou, nepřidělují — nepočítají se.

export interface Usage { classes: string[]; dynamic: string[] }
export interface SourceFile { file: string; src: string }
export interface Missing { cls: string; files: string[] }

const TOKEN = /^-?[_a-zA-Z][\w-]*$/;
const squash = (s: string) => s.replace(/\s+/g, " ").trim();
const QUOTES = "\"'`";

/** Index ZA koncem literálu, který začíná uvozovkou na `i`; šablona (`) se čte i s vnořenými `${…}`. */
function literalEnd(s: string, i: number): number {
  const q = s[i];
  for (let j = i + 1; j < s.length; j++) {
    if (s[j] === "\\") j++;
    else if (q === "`" && s[j] === "$" && s[j + 1] === "{") j = pairEnd(s, j + 1) - 1;
    else if (s[j] === q) return j + 1;
  }
  return s.length;
}

/** Index ZA párovou závorkou k té na `open` (`{` nebo `(`); řetězce a šablony přeskočí. */
function pairEnd(s: string, open: number): number {
  const [o, c] = s[open] === "(" ? ["(", ")"] : ["{", "}"];
  let depth = 0;
  for (let i = open; i < s.length; i++) {
    if (QUOTES.includes(s[i])) i = literalEnd(s, i) - 1;
    else if (s[i] === o) depth++;
    else if (s[i] === c && --depth === 0) return i + 1;
  }
  return s.length;
}

/** Slova řetězce jako třídy; slovo přilepené k dynamické části (`pill-${x}`, `"st-" + x`) je dynamické. */
function words(text: string, glueL: boolean, glueR: boolean, out: Usage, expr: string) {
  const parts = text.split(/\s+/);
  parts.forEach((w, k) => {
    if (!w) return;
    const glued = (k === 0 && glueL) || (k === parts.length - 1 && glueR);
    if (glued || !TOKEN.test(w)) out.dynamic.push(expr);
    else out.classes.push(w);
  });
}

/** Výraz: literály = třídy (kromě porovnávaných, `x === "dev" ? …`). Když výsledek může být i něco jiného
 *  než literál (`p.class`, `"st-" + x`, volání), je místo dynamické. */
function fromExpr(e: string, out: Usage, expr: string) {
  let skel = "";
  for (let i = 0; i < e.length; i++) {
    if (!QUOTES.includes(e[i])) { skel += e[i]; continue; }
    const end = literalEnd(e, i);
    const before = e.slice(0, i).trimEnd(), after = e.slice(end).trimStart();
    if (/[!=]=$/.test(before) || /^[!=]=/.test(after)) skel += "C";
    else {
      const glueL = before.endsWith("+"), glueR = after.startsWith("+");
      if (e[i] === "`") fromTemplate(e.slice(i + 1, end - 1), glueL, glueR, out, expr);
      else words(e.slice(i + 1, end - 1), glueL, glueR, out, expr);
      skel += "\0";
    }
    i = end - 1;
  }
  // hodnota je vždy literál: `"a"`, `c ? "a" : "b"` (i řetězené), `c && "a"`
  const s = squash(skel);
  const literalOnly = /^\0$/.test(s) || /^[^\0?]+\? ?\0 ?: ?(?:[^\0?]+\? ?\0 ?: ?)*\0$/.test(s) || /^[^\0?]+&& ?\0$/.test(s);
  if (!literalOnly) out.dynamic.push(expr);
}

/** Šablona: statické úseky = slova, `${…}` = výraz; `${…}` nalepený na slovo (bez mezery) je dynamický. */
function fromTemplate(body: string, glueL: boolean, glueR: boolean, out: Usage, expr: string) {
  const stat: string[] = [], dyn: string[] = [];
  let from = 0;
  for (let i = 0; i < body.length; i++) {
    if (body[i] === "\\") i++;
    else if (body[i] === "$" && body[i + 1] === "{") {
      const end = pairEnd(body, i + 1);
      stat.push(body.slice(from, i));
      dyn.push(body.slice(i + 2, end - 1));
      from = end;
      i = end - 1;
    }
  }
  stat.push(body.slice(from));
  const tight = (s: string, atEnd: boolean) => s.length > 0 && !(atEnd ? /\s$/ : /^\s/).test(s);
  stat.forEach((s, k) => words(s, k === 0 ? glueL : tight(s, false), k === stat.length - 1 ? glueR : tight(s, true), out, expr));
  dyn.forEach((d, k) => {
    if (tight(stat[k], true) || tight(stat[k + 1], false)) out.dynamic.push(expr);
    else fromExpr(d, out, expr);
  });
}

/** Klíče objektu `{ on: …, "b-c": … }` (jen nejvyšší úroveň); vypočtený klíč `[x]` je dynamický. */
function objectKeys(body: string, out: Usage, expr: string) {
  const parts: string[] = [];
  let depth = 0, from = 0;
  for (let i = 0; i < body.length; i++) {
    const c = body[i];
    if (QUOTES.includes(c)) i = literalEnd(body, i) - 1;
    else if ("([{".includes(c)) depth++;
    else if (")]}".includes(c)) depth--;
    else if (c === "," && depth === 0) { parts.push(body.slice(from, i)); from = i + 1; }
  }
  parts.push(body.slice(from));
  for (const p of parts.map((x) => x.trim()).filter(Boolean)) {
    const m = /^(?:(["'])([^"']*)\1|([\w$]+))\s*(?::|$)/.exec(p);
    if (m) words(m[2] ?? m[3], false, false, out, expr);
    else out.dynamic.push(expr);
  }
}

/** Třídy, které zdroj přiděluje prvkům, a místa, kde se název skládá za běhu (kód místa, pro allowlist). */
export function usedClasses(src: string): Usage {
  const out: Usage = { classes: [], dynamic: [] };
  // komentáře jen tam, kde nemůžou být uvnitř řetězce (`accept="image/*"` by jinak spolkl kód až k dalšímu `*/`)
  const code = src.replace(/\{\/\*[\s\S]*?\*\/\}|^\s*\/\*[\s\S]*?\*\/|^\s*\/\/.*$/gm, "");
  const site = /\b(class|className|classList)\s*[=:]\s*(?=["'`{])|\bclassList\.(?:add|toggle|replace)(?=\()/g;
  for (let m; (m = site.exec(code));) {
    const at = m.index + m[0].length;
    if (!m[1]) { // classList.add("a", "b") / toggle("a", cond) / replace("a", "b")
      const end = pairEnd(code, at);
      const expr = squash(code.slice(m.index, end));
      const args = code.slice(at + 1, end - 1);
      if (!/^\s*["']/.test(args)) out.dynamic.push(expr);
      for (const l of args.matchAll(/(["'])([^"']*)\1/g)) words(l[2], false, false, out, expr);
      site.lastIndex = end;
      continue;
    }
    const brace = code[at] === "{";
    const end = brace ? pairEnd(code, at) : literalEnd(code, at);
    const expr = squash(code.slice(m.index, end));
    const inner = code.slice(at + 1, end - 1);
    if (m[1] === "classList") {
      if (brace && /^\s*\{[\s\S]*\}\s*$/.test(inner)) objectKeys(inner.trim().slice(1, -1), out, expr);
      else out.dynamic.push(expr);
    } else if (brace) fromExpr(inner, out, expr);
    else if (code[at] === "`") fromTemplate(inner, false, false, out, expr);
    else words(inner, false, false, out, expr);
    site.lastIndex = end;
  }
  return { classes: [...new Set(out.classes)].sort(), dynamic: [...new Set(out.dynamic)] };
}

/** Definované třídy = všechny, které zmiňuje nějaký selektor v CSS (i jako modifikátor nebo v `:is()`). */
export function definedClasses(css: CssFile[]): Set<string> {
  return new Set(css.flatMap((f) => cssRules(f.css).flatMap((r) => classesIn(r.selector))));
}

/** Třídy, které některý zdroj přiděluje, ale žádný CSS selektor nezmiňuje; `allow` = vědomé háčky bez pravidla. */
export function missingClasses(sources: SourceFile[], css: CssFile[], allow: Record<string, string> = {}): Missing[] {
  const defined = definedClasses(css);
  const where = new Map<string, Set<string>>();
  for (const { file, src } of sources) {
    for (const c of usedClasses(src).classes) {
      if (!defined.has(c) && !Object.prototype.hasOwnProperty.call(allow, c)) (where.get(c) ?? where.set(c, new Set()).get(c)!).add(file);
    }
  }
  return [...where].map(([cls, f]) => ({ cls, files: [...f].sort() })).sort((a, b) => a.cls.localeCompare(b.cls));
}
