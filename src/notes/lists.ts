// Chytré seznamy v textarea poznámky (editor v šuplíku i lístek na ploše): Enter pokračuje číslováním,
// odrážkou nebo úkolem, Enter na prázdné položce seznam ukončí, Tab / Shift+Tab položky odsadí.
// Čisté funkce nad textem a pozicí kurzoru (bez DOM; test `lists.test.mjs`) — textarea je jen vyvolá.
// Nic jiného čísla nepřepisuje: vložení doprostřed seznamu následující čísla nepřečísluje.

export type ListKind = "bullet" | "number" | "task";
export type ListItem = {
  /** odsazení řádku (mezery / tabulátory) */
  indent: string;
  kind: ListKind;
  /** znak odrážky (`-` `*` `+`) u odrážky a úkolu */
  bullet: string;
  /** číslo a oddělovač (`.` / `)`) u číslované položky */
  num: number;
  delim: string;
  /** délka řádku až po začátek textu položky (odsazení + značka + mezera) */
  prefixLen: number;
  /** šířka značky s mezerou za ní (`- ` = 2, `10. ` = 4) — o tolik se vnořená položka odsadí */
  width: number;
  rest: string;
};

const TASK = /^(\s*)([-*+]) \[[ xX]\](?: (.*))?$/;
const ITEM = /^(\s*)([-*+]|\d+[.)]) (.*)$/;

/** Řádek jako položka seznamu (odrážka, číslo `1.` / `1)`, úkol `- [ ]`), jinak null. */
export function parseItem(line: string): ListItem | null {
  const t = TASK.exec(line);
  if (t) {
    const hasRest = t[3] !== undefined;
    return { indent: t[1], kind: "task", bullet: t[2], num: 0, delim: "", prefixLen: t[1].length + 5 + (hasRest ? 1 : 0), width: 2, rest: t[3] ?? "" };
  }
  const m = ITEM.exec(line);
  if (!m) return null;
  const mark = m[2];
  if (/^\d/.test(mark)) {
    const delim = mark.slice(-1);
    return { indent: m[1], kind: "number", bullet: "", num: Number(mark.slice(0, -1)), delim, prefixLen: m[1].length + mark.length + 1, width: mark.length + 1, rest: m[3] };
  }
  return { indent: m[1], kind: "bullet", bullet: mark, num: 0, delim: "", prefixLen: m[1].length + 2, width: 2, rest: m[3] };
}

/** Začátek řádku, na kterém je `pos`. */
const lineStart = (text: string, pos: number) => (pos <= 0 ? 0 : text.lastIndexOf("\n", pos - 1) + 1);

export type EnterResult = { text: string; caret: number };

/** Enter v textarea: pokračuje seznam (nová položka je vždy neodškrtnutá, odsazení zůstane, text za kurzorem
 *  přejde do nové položky); na PRÁZDNÉ položce značku smaže (seznam skončí). Mimo seznam, nebo s kurzorem
 *  uvnitř značky vrací null — Enter pak dělá, co dělá. */
export function onEnter(text: string, caret: number): EnterResult | null {
  if (!Number.isInteger(caret) || caret < 0 || caret > text.length) return null;
  const ls = lineStart(text, caret);
  let le = text.indexOf("\n", caret);
  if (le < 0) le = text.length;
  const line = text.slice(ls, le);
  const it = parseItem(line.replace(/\r$/, ""));
  if (!it) return null;
  const col = caret - ls;
  if (col < it.prefixLen) return null;
  if (!it.rest.trim()) return { text: text.slice(0, ls) + text.slice(le), caret: ls };
  const marker = it.kind === "number" ? `${it.num + 1}${it.delim}` : it.kind === "task" ? `${it.bullet} [ ]` : it.bullet;
  const head = line.slice(0, col);
  const ins = `\n${it.indent}${marker} `;
  return { text: text.slice(0, ls) + head + ins + text.slice(caret), caret: ls + head.length + ins.length };
}

export type TabResult = { text: string; selStart: number; selEnd: number };

/** Číslo nové číslované položky na odsazení `indent` (řádky nad `idx`): navazuje na sousední položku téhož
 *  seznamu (stejné odsazení a oddělovač), jinak začíná od 1. */
function numberAt(lines: string[], idx: number, indent: number, delim: string): number {
  for (let j = idx - 1; j >= 0; j--) {
    if (!lines[j].trim()) continue;
    const it = parseItem(lines[j]);
    if (!it) return 1;
    const ind = it.indent.length;
    if (ind > indent) continue;
    if (ind < indent) return 1;
    return it.kind === "number" && it.delim === delim ? it.num + 1 : 1;
  }
  return 1;
}

/** Odsazení, na které Tab přesune položku `i`: pod předchozí položku (o šířku její značky — `- ` 2, `1. ` 3,
 *  aby to Markdown opravdu vnořil), k jejímu sourozenci, je-li ta už vnořená hlouběji; už vnořenou dál neposouvá. */
function indentTarget(lines: string[], i: number, cur: number): number {
  for (let j = i - 1; j >= 0; j--) {
    if (!lines[j].trim()) continue;
    const prev = parseItem(lines[j]);
    if (!prev) break;
    const pi = prev.indent.length;
    if (pi > cur) return pi;
    if (pi === cur) return cur + prev.width;
    return cur;
  }
  return cur + 2;
}

/** Odsazení nadřazené položky (nejbližší výš s menším odsazením), 0 když žádná není. */
function parentIndent(lines: string[], i: number, cur: number): number {
  for (let j = i - 1; j >= 0; j--) {
    if (!lines[j].trim()) continue;
    const it = parseItem(lines[j]);
    if (!it) break;
    if (it.indent.length < cur) return it.indent.length;
  }
  return 0;
}

/** Tab / Shift+Tab na položce (nebo označených řádcích): odsadí, resp. předsadí všechny položky seznamu
 *  v rozsahu o stejný krok. Nově vnořená číslovaná položka začne od `1.` (nebo naváže na sourozence).
 *  Bez položky seznamu v dosahu vrací null — Tab pak dělá, co dělá (přesun fokusu). Položka, která už
 *  jít nemůže (první / nejhlubší, nejlevější), vrací text beze změny (Tab nesmí z textarea utéct). */
export function onTab(text: string, selStart: number, selEnd: number, shift: boolean): TabResult | null {
  const a = Math.max(0, Math.min(selStart, selEnd));
  const b = Math.min(text.length, Math.max(selStart, selEnd));
  const lines = text.split("\n");
  const starts: number[] = [];
  let o = 0;
  for (const l of lines) {
    starts.push(o);
    o += l.length + 1;
  }
  const lineAt = (pos: number) => {
    let i = starts.length - 1;
    while (i > 0 && starts[i] > pos) i--;
    return i;
  };
  const i0 = lineAt(a);
  let i1 = lineAt(b);
  if (b > a && starts[i1] === b && i1 > i0) i1--;
  let first = -1;
  for (let i = i0; i <= i1; i++) {
    if (parseItem(lines[i])) {
      first = i;
      break;
    }
  }
  if (first < 0) return null;
  const cur = parseItem(lines[first])!.indent.length;
  const delta = (shift ? parentIndent(lines, first, cur) : indentTarget(lines, first, cur)) - cur;
  if (delta === 0) return { text, selStart, selEnd };

  const out = [...lines];
  const d: number[] = lines.map(() => 0);
  for (let k = i0; k <= i1; k++) {
    const it = parseItem(out[k]);
    if (!it) continue;
    let line = out[k];
    if (delta > 0) {
      line = " ".repeat(delta) + line;
      if (it.kind === "number") {
        const ind = it.indent.length + delta;
        line = line.replace(/\d+/, String(numberAt(out, k, ind, it.delim)));
      }
    } else {
      const strip = Math.min(-delta, it.indent.length);
      line = line.slice(strip);
    }
    d[k] = line.length - out[k].length;
    out[k] = line;
  }
  // výběr řádků začínající na začátku řádku tam i zůstane; samotný kurzor na začátku jde za odsazení
  const range = a < b;
  const map = (pos: number, isLow: boolean) => {
    const L = lineAt(pos);
    let sum = 0;
    for (let k = 0; k < L; k++) sum += d[k];
    const col = pos - starts[L];
    if (range && isLow && col === 0 && d[L] > 0) return pos + sum;
    return pos + sum + (d[L] < 0 ? Math.max(d[L], -col) : d[L]);
  };
  const startIsLow = selStart <= selEnd;
  return { text: out.join("\n"), selStart: map(selStart, startIsLow), selEnd: map(selEnd, !startIsLow) };
}
