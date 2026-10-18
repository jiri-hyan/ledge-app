// Pravidlo vlny 7: žádný segmentový přepínač s víc než 4 volbami (přetéká kartu, popisek se zmáčkne) — víc
// voleb = galerie dlaždic nebo rozbalovací výběr. Statická kontrola celého `src` (parser TypeScriptu):
//  - `<Seg … options={…}>` (settings/fields/controls.tsx i panels/apUi.tsx),
//  - prvky s třídou `seg` / `st-seg` (ruční přepínače: editor akce, poznámka…): tlačítka + `<For each={…}>`.
// Počet voleb: literál pole, konstanta v témže souboru, nebo importovaná konstanta z `.ts` modulu (načte se
// a spočítá); `X.map(…)`, `keys(X)`/`Object.keys(X)` se rozbalí. Co nejde spočítat, test shodí — dej volby
// do konstanty. Definice samotných komponent (`For each={p.options}`) se přeskočí, kontrolují se použití.
//   node --experimental-strip-types --test src/settings/segments.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";

const MAX = 4;
/** Vědomé výjimky: soubor → proč to není přepínač voleb. */
const EXEMPT = {
  "layout/Tabs.tsx": "záložky šuplíku jsou uživatelova data (vlastní záložky), ne volby — vypadá jako přepínač, počet určuje uživatel",
};
const SRC = fileURLToPath(new URL("..", import.meta.url));
const walk = (d) => readdirSync(d).flatMap((n) => {
  const p = join(d, n);
  return statSync(p).isDirectory() ? walk(p) : /\.tsx$/.test(n) ? [p] : [];
});
const rel = (p) => relative(SRC, p).split(sep).join("/");

/** Hodnota importované konstanty (jen `.ts` moduly bez JSX). */
async function imported(file, sf, name) {
  for (const st of sf.statements) {
    if (!ts.isImportDeclaration(st) || !st.importClause?.namedBindings || !ts.isNamedImports(st.importClause.namedBindings)) continue;
    const el = st.importClause.namedBindings.elements.find((e) => e.name.text === name);
    if (!el) continue;
    const spec = st.moduleSpecifier.text;
    if (!spec.startsWith(".")) return undefined;
    const base = join(dirname(file), spec);
    const path = [base, `${base}.ts`].find((x) => /\.ts$/.test(x) && existsSync(x));
    if (!path) return undefined;
    const mod = await import(pathToFileURL(path).href);
    return mod[(el.propertyName ?? el.name).text];
  }
  return undefined;
}

/** Počet voleb výrazu (pole / objekt → klíče) nebo null, když nejde spočítat. */
async function count(file, sf, e) {
  if (!e) return null;
  if (ts.isParenthesizedExpression(e) || ts.isAsExpression(e) || ts.isSatisfiesExpression?.(e)) return count(file, sf, e.expression);
  if (ts.isArrayLiteralExpression(e)) return e.elements.length;
  if (ts.isObjectLiteralExpression(e)) return e.properties.length;
  if (ts.isCallExpression(e)) {
    const c = e.expression;
    // X.map(…) / X.filter(…) nezvětší počet
    if (ts.isPropertyAccessExpression(c) && ["map", "filter", "slice"].includes(c.name.text)) return count(file, sf, c.expression);
    // keys(X) / Object.keys(X)
    const fn = ts.isIdentifier(c) ? c.text : ts.isPropertyAccessExpression(c) ? c.name.text : "";
    if (fn === "keys" && e.arguments[0]) return count(file, sf, e.arguments[0]);
    return null;
  }
  if (ts.isIdentifier(e)) {
    let local = null;
    const find = (n) => {
      if (ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) && n.name.text === e.text && n.initializer) local = n.initializer;
      ts.forEachChild(n, find);
    };
    find(sf);
    if (local) return count(file, sf, local);
    const v = await imported(file, sf, e.text);
    if (Array.isArray(v)) return v.length;
    if (v && typeof v === "object") return Object.keys(v).length;
    return null;
  }
  return null;
}

const attr = (el, name) => el.attributes.properties.find((a) => ts.isJsxAttribute(a) && a.name.getText() === name);
const attrExpr = (a) => (a?.initializer && ts.isJsxExpression(a.initializer) ? a.initializer.expression : null);
const classOf = (el) => {
  const a = attr(el, "class");
  return a?.initializer && ts.isStringLiteral(a.initializer) ? a.initializer.text.split(/\s+/) : [];
};
/** `For each={p.options}` / `q.options` = definice komponenty přepínače, ne použití */
const isProps = (e) => !!e && ts.isPropertyAccessExpression(e) && ts.isIdentifier(e.expression) && ["p", "q", "props"].includes(e.expression.text);

async function scan(file) {
  const text = readFileSync(file, "utf8");
  const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const out = [];
  const nodes = [];
  const visit = (n) => {
    nodes.push(n);
    ts.forEachChild(n, visit);
  };
  visit(sf);
  for (const n of nodes) {
    const open = ts.isJsxElement(n) ? n.openingElement : ts.isJsxSelfClosingElement(n) ? n : null;
    if (!open) continue;
    const tag = open.tagName.getText(sf);
    const where = `${rel(file)}:${sf.getLineAndCharacterOfPosition(open.getStart(sf)).line + 1}`;
    if (tag === "Seg") {
      const n2 = await count(file, sf, attrExpr(attr(open, "options")));
      out.push({ where, n: n2 });
    } else if (/^[a-z]/.test(tag) && classOf(open).some((c) => c === "seg" || c === "st-seg") && ts.isJsxElement(n)) {
      let total = 0;
      let skip = false;
      for (const ch of n.children) {
        if (ts.isJsxText(ch) && !ch.text.trim()) continue;
        if (ts.isJsxElement(ch) || ts.isJsxSelfClosingElement(ch)) {
          const o = ts.isJsxElement(ch) ? ch.openingElement : ch;
          if (o.tagName.getText(sf) === "For") {
            const each = attrExpr(attr(o, "each"));
            if (isProps(each)) skip = true;
            else {
              const c = await count(file, sf, each);
              total = c === null || total === null ? null : total + c;
            }
          } else if (total !== null) total += 1;
        } else if (ts.isJsxExpression(ch) && total !== null) total += 1;
      }
      if (!skip) out.push({ where, n: total });
    }
  }
  return out;
}

test("žádný přepínač nemá víc než 4 volby (víc = galerie nebo rozbalovací)", async () => {
  const files = walk(SRC);
  for (const f of Object.keys(EXEMPT)) assert.ok(files.some((x) => rel(x) === f), `výjimka ${f} existuje`);
  const all = (await Promise.all(files.filter((x) => !(rel(x) in EXEMPT)).map(scan))).flat();
  assert.ok(all.length >= 15, `našly se přepínače (${all.length})`);
  const unknown = all.filter((x) => x.n === null).map((x) => x.where);
  assert.deepEqual(unknown, [], "počet voleb nejde spočítat — dej volby do konstanty");
  const big = all.filter((x) => x.n > MAX).map((x) => `${x.where}: ${x.n} voleb`);
  assert.deepEqual(big, []);
});

test("kontrola opravdu počítá (tabulka na ukázkovém kódu)", async () => {
  const tmp = fileURLToPath(new URL("./appearanceModel.ts", import.meta.url));
  const sf = (src) => ts.createSourceFile("x.tsx", src, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const cases = [
    [`const A = [1, 2, 3, 4, 5]; const x = A.map((v) => v);`, "A", 5],
    [`const O = { a: 1, b: 2 }; const x = 0;`, "keys(O)", 2],
    [`import { WIDTH_STEPS } from "./appearanceModel"; const x = 0;`, "WIDTH_STEPS.map((w) => w)", 4],
    [`const x = 0;`, "neznamo", null],
  ];
  for (const [src, expr, want] of cases) {
    const f = sf(`${src}\nconst probe = ${expr};`);
    const decl = f.statements.at(-1).declarationList.declarations[0].initializer;
    assert.equal(await count(tmp, f, decl), want, expr);
  }
});
