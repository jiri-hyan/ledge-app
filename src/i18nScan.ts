// Statická analýza překladů pro hlídací test `src/i18n.test.mjs` (bez DOM, jen parser TypeScriptu):
// volání `t("klíč", "čeština", …)` / `tp("klíč", n, čeština, …)`, záznamy `registerDict("en", {…})`
// a zbylý natvrdo český text (řetězce a JSX text s diakritikou mimo t()/tp(), komentáře se nepočítají).
// Datové páry `{ key: "a.b", cs: "…" }` a `["a.b", "…"]` (překládané později přes `t(x.key, x.cs)`)
// se počítají jako volání; volání s klíčem mimo literál vrací `keyText` (a `prefix` u šablony).
import ts from "typescript";

export const CZECH = /[áčďéěíňóřšťúůýžÁČĎÉĚÍŇÓŘŠŤÚŮÝŽ]/;
export const KEY = /^[a-z][A-Za-z0-9]*(\.[A-Za-z0-9]+)+$/;
/** Oblasti klíčů (první část klíče). Pole `["a.b", "…"]` je pár jen s klíčem z těchto oblastí — jinak to
 *  bývají data (`["drawer.glass", "vzhled"]` = cesta v configu → panel). */
export const AREAS = ["cards", "sessions", "notes", "palette", "actions", "format", "ui", "appearance", "settings", "questions", "onboarding"];
export const areaKey = (k: string) => KEY.test(k) && AREAS.includes(k.slice(0, k.indexOf(".")));

export type Call = {
  fn: "t" | "tp";
  key: string | null;
  /** české tvary (t: jeden; tp: jeden pevný nebo tři [1, 2–4, 5+]); null = není literál */
  cs: string[] | null;
  line: number;
  /** zdrojový text klíče, když není literál (`m.key`, `` `settings.f.${id}` ``) */
  keyText?: string;
  /** pevný začátek šablony klíče (`settings.f.`) — slovníkové klíče s ním se berou jako použité */
  prefix?: string;
  /** datový pár místo volání */
  pair?: boolean;
  /** volání mimo funkci = vyhodnotí se jednou při importu (jazyk se pak neprojeví) */
  top: boolean;
};
export type Entry = { lang: string; key: string; value: string | null; line: number };
export type Leftover = { line: number; text: string };
export type Scan = { calls: Call[]; dict: Entry[]; leftovers: Leftover[] };
/** Tabulka popisků, jejichž klíč se skládá z id: `const FIELDS = { "id": { label: "čeština", … } }` —
 *  `key(id, vlastnost)` vrátí klíč (nebo null = vlastnost není text). Texty se počítají jako páry. */
export type Table = { table: string; key: (id: string, prop: string) => string | null };

/** Hodnota literálu (i `"a" + "b"`), jinak null. */
function lit(n: ts.Node | undefined): string | null {
  if (!n) return null;
  if (ts.isStringLiteral(n) || ts.isNoSubstitutionTemplateLiteral(n)) return n.text;
  if (ts.isParenthesizedExpression(n)) return lit(n.expression);
  if (ts.isBinaryExpression(n) && n.operatorToken.kind === ts.SyntaxKind.PlusToken) {
    const a = lit(n.left);
    const b = lit(n.right);
    return a !== null && b !== null ? a + b : null;
  }
  return null;
}

const calleeName = (n: ts.CallExpression) => (ts.isIdentifier(n.expression) ? n.expression.text : null);
const isConsole = (n: ts.CallExpression) =>
  ts.isPropertyAccessExpression(n.expression) && ts.isIdentifier(n.expression.expression) && n.expression.expression.text === "console";
const insideFunction = (n: ts.Node) => {
  for (let p = n.parent; p; p = p.parent) if (ts.isFunctionLike(p)) return true;
  return false;
};

function csOf(fn: "t" | "tp", n: ts.CallExpression): string[] | null {
  let a = n.arguments[fn === "t" ? 1 : 2];
  const one = lit(a);
  if (one !== null) return [one];
  while (a && (ts.isAsExpression(a) || ts.isParenthesizedExpression(a))) a = a.expression;
  if (fn === "tp" && a && ts.isArrayLiteralExpression(a) && a.elements.length === 3) {
    const forms = a.elements.map((e) => lit(e));
    return forms.every((f) => f !== null) ? (forms as string[]) : null;
  }
  return null;
}

/** Datový pár klíč + čeština: `{ key: "a.b", cs: "…", … }` nebo `["a.b", "…"]`. Ostatní vlastnosti
 *  objektu se dál procházejí (můžou mít vlastní t()), proto vrací jen dvojici. */
function pairOf(n: ts.Node): [string, string] | null {
  if (!ts.isArrayLiteralExpression(n) || n.elements.length < 2 || n.elements.length > 3) return null;
  // `["klíč", "čeština"]` nebo `[hodnota, "klíč", "čeština"]` (např. akcenty `[hex, key, cs]`)
  const v = n.elements.map((e) => lit(e));
  const i = v.length === 2 ? 0 : 1;
  const k = v[i];
  const cs = v[i + 1];
  return k != null && cs != null && areaKey(k) ? [k, cs] : null;
}
const objectPair = (n: ts.ObjectLiteralExpression): { key: string; cs: string; csNode: ts.Node } | null => {
  let key: string | null = null;
  let cs: ts.Node | null = null;
  for (const p of n.properties) {
    if (!ts.isPropertyAssignment(p) || !ts.isIdentifier(p.name)) continue;
    if (p.name.text === "key") key = lit(p.initializer);
    if (p.name.text === "cs") cs = p.initializer;
  }
  const csText = cs ? lit(cs) : null;
  return key !== null && KEY.test(key) && cs && csText !== null ? { key, cs: csText, csNode: cs } : null;
};

export function scanSource(name: string, src: string, tables: readonly Table[] = []): Scan {
  const sf = ts.createSourceFile(name, src, ts.ScriptTarget.Latest, true, name.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const line = (n: ts.Node) => sf.getLineAndCharacterOfPosition(n.getStart(sf)).line + 1;
  const out: Scan = { calls: [], dict: [], leftovers: [] };
  const skip = new Set<ts.Node>();
  const visit = (n: ts.Node): void => {
    if (skip.has(n) || ts.isImportDeclaration(n) || ts.isExportDeclaration(n) || ts.isLiteralTypeNode(n)) return;
    const table = ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) ? tables.find((x) => x.table === (n.name as ts.Identifier).text) : undefined;
    if (table && ts.isVariableDeclaration(n) && n.initializer && ts.isObjectLiteralExpression(n.initializer)) {
      for (const row of n.initializer.properties) {
        if (!ts.isPropertyAssignment(row) || !ts.isObjectLiteralExpression(row.initializer)) continue;
        const id = lit(row.name as ts.Expression) ?? (ts.isIdentifier(row.name) ? row.name.text : null);
        if (id === null) continue;
        for (const q of row.initializer.properties) {
          if (!ts.isPropertyAssignment(q) || !ts.isIdentifier(q.name)) continue;
          const key = table.key(id, q.name.text);
          const cs = lit(q.initializer);
          if (key === null || cs === null) continue;
          out.calls.push({ fn: "t", key, cs: [cs], line: line(q), top: false, pair: true });
          skip.add(q.initializer);
        }
      }
    }
    if (ts.isObjectLiteralExpression(n)) {
      const op = objectPair(n);
      if (op) {
        out.calls.push({ fn: "t", key: op.key, cs: [op.cs], line: line(n), top: false, pair: true });
        skip.add(op.csNode);
      }
    }
    if (ts.isCallExpression(n)) {
      if (isConsole(n)) return;
      const fn = calleeName(n);
      if (fn === "t" || fn === "tp") {
        const a0 = n.arguments[0];
        const key = lit(a0);
        const call: Call = { fn, key, cs: csOf(fn, n), line: line(n), top: !insideFunction(n) };
        if (key === null && a0) {
          call.keyText = a0.getText(sf);
          if (ts.isTemplateExpression(a0)) call.prefix = a0.head.text;
        }
        out.calls.push(call);
        // proměnné (`{ n: … }`) můžou obsahovat další t() — projít jen je, české argumenty jsou v pořádku
        n.arguments.slice(fn === "t" ? 2 : 3).forEach(visit);
        if (fn === "tp") visit(n.arguments[1]);
        return;
      }
      if (fn === "registerDict") {
        const l = lit(n.arguments[0]) ?? "?";
        const obj = n.arguments[1];
        if (obj && ts.isObjectLiteralExpression(obj)) {
          for (const p of obj.properties) {
            const key = p.name ? lit(p.name) ?? (ts.isIdentifier(p.name) ? p.name.text : null) : null;
            if (key !== null) out.dict.push({ lang: l, key, value: ts.isPropertyAssignment(p) ? lit(p.initializer) : null, line: line(p) });
          }
        }
        return;
      }
    }
    const pair = pairOf(n);
    if (pair) {
      out.calls.push({ fn: "t", key: pair[0], cs: [pair[1]], line: line(n), top: false, pair: true });
      return;
    }
    const text =
      ts.isStringLiteral(n) || ts.isNoSubstitutionTemplateLiteral(n) || ts.isTemplateHead(n) || ts.isTemplateMiddle(n) || ts.isTemplateTail(n)
        ? n.text
        : ts.isJsxText(n)
          ? n.text.trim()
          : null;
    if (text !== null && CZECH.test(text)) out.leftovers.push({ line: line(n), text });
    ts.forEachChild(n, visit);
  };
  visit(sf);
  return out;
}

/** Názvy proměnných `{x}` v textu (seřazené, bez duplicit). */
export const varsOf = (s: string) => [...new Set([...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]))].sort();
/** Počet tučných úseků (`rich`). */
export const boldCount = (s: string) => (s.match(/<b>/g) ?? []).length;
