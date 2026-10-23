// Chytré seznamy (`lists.ts`): Enter pokračuje / ukončuje, Tab vnořuje. Čisté funkce, tabulkové testy.
// `|` v textu případu značí pozici kurzoru (pro čitelnost).
import { test } from "node:test";
import assert from "node:assert/strict";
import { onEnter, onTab, parseItem } from "./lists.ts";

const split = (s) => ({ text: s.replace("|", ""), caret: s.indexOf("|") });
const mark = (r) => (r ? r.text.slice(0, r.caret) + "|" + r.text.slice(r.caret) : null);

test("parseItem: odrážky, čísla, úkoly; jinak null", () => {
  const cases = [
    ["- a", "bullet", 2],
    ["  * a", "bullet", 4],
    ["+ a", "bullet", 2],
    ["1. a", "number", 3],
    ["12) a", "number", 4],
    ["- [ ] a", "task", 6],
    ["  - [x] a", "task", 8],
    ["- [ ]", "task", 5],
    ["-", null],
    ["---", null],
    ["1.a", null],
    ["text", null],
    ["", null],
    ["2024. rok", "number", 6],
  ];
  for (const [line, kind, prefix] of cases) {
    const it = parseItem(line);
    assert.equal(it ? it.kind : null, kind, line);
    if (it) assert.equal(it.prefixLen, prefix, line);
  }
});

test("Enter: pokračuje seznam", () => {
  const cases = [
    ["1. první|", "1. první\n2. |"],
    ["1) první|", "1) první\n2) |"],
    ["9. devět|", "9. devět\n10. |"],
    ["- a|", "- a\n- |"],
    ["* a|", "* a\n* |"],
    ["+ a|", "+ a\n+ |"],
    ["- [ ] úkol|", "- [ ] úkol\n- [ ] |"],
    ["- [x] hotovo|", "- [x] hotovo\n- [ ] |"],
    ["  1. vnořená|", "  1. vnořená\n  2. |"],
    ["    - hluboko|", "    - hluboko\n    - |"],
    ["předtím\n1. a|\npotom", "předtím\n1. a\n2. |\npotom"],
    // uprostřed položky: zbytek přejde do nové
    ["1. ab|cd", "1. ab\n2. |cd"],
    ["- [ ] ab|cd", "- [ ] ab\n- [ ] |cd"],
    // vložení doprostřed nepřečísluje následující
    ["1. a|\n2. b\n3. c", "1. a\n2. |\n2. b\n3. c"],
  ];
  for (const [inp, want] of cases) assert.equal(mark(onEnter(split(inp).text, split(inp).caret)), want, inp);
});

test("Enter: prázdná položka seznam ukončí (smaže značku)", () => {
  const cases = [
    ["1. a\n2. |", "1. a\n|"],
    ["- a\n- |", "- a\n|"],
    ["- a\n- [ ] |", "- a\n|"],
    ["- [x] |", "|"],
    ["  - |\nza", "|\nza"],
    ["1. |\nza", "|\nza"],
    ["- [ ]|", "|"],
  ];
  for (const [inp, want] of cases) assert.equal(mark(onEnter(split(inp).text, split(inp).caret)), want, inp);
});

test("Enter: mimo seznam a uvnitř značky nedělá nic (null)", () => {
  const cases = [
    "obyčejný text|",
    "|",
    "|- a",
    "-| a",
    "1|. a",
    "--- |",
    "**tučně**|",
    "1.a|",
    "text\n|\n1. a",
  ];
  for (const inp of cases) assert.equal(onEnter(split(inp).text, split(inp).caret), null, inp);
  assert.equal(onEnter("a", 5), null);
  assert.equal(onEnter("a", -1), null);
});

// pomocník: kurzor / výběr jako « » (hranaté závorky patří úkolům `- [ ]`)
const sel = (s) => ({ text: s.replace("«", "").replace("»", ""), a: s.indexOf("«"), b: s.indexOf("»") - 1 });
const showSel = (r) => {
  if (!r) return null;
  const chars = [...r.text];
  const lo = Math.min(r.selStart, r.selEnd), hi = Math.max(r.selStart, r.selEnd);
  chars.splice(hi, 0, "»");
  chars.splice(lo, 0, "«");
  return chars.join("");
};
const tab = (inp, shift) => {
  const { text, a, b } = sel(inp);
  return showSel(onTab(text, a, b, shift));
};

test("Tab: odsadí položku pod předchozí; číslovaná začne od 1.", () => {
  const cases = [
    ["- a\n- «b»", "- a\n  - «b»"],
    ["- a\n- b«»", "- a\n  - b«»"],
    ["- a\n«»- b", "- a\n  «»- b"],
    ["1. a\n2. «b»", "1. a\n   1. «b»"],
    ["1. a\n2. b\n3. «c»", "1. a\n2. b\n   1. «c»"],
    ["- [ ] a\n- [ ] «b»", "- [ ] a\n  - [ ] «b»"],
    // navazuje na již existující vnořené číslování
    ["1. a\n   1. x\n   2. y\n2. «b»", "1. a\n   1. x\n   2. y\n   3. «b»"],
    // už vnořená pod předchozí → beze změny
    ["- a\n  - «b»", "- a\n  - «b»"],
    // první položka bez předchůdce: o 2
    ["- «a»", "  - «a»"],
  ];
  for (const [inp, want] of cases) assert.equal(tab(inp, false), want, inp);
});

test("Tab: výběr více položek posune všechny o stejný krok a očísluje znovu", () => {
  const cases = [
    ["1. a\n«2. b\n3. c»", "1. a\n«   1. b\n   2. c»"],
    ["- a\n«- b\n- c»", "- a\n«  - b\n  - c»"],
    // prázdná / neseznamová řádka ve výběru zůstane
    ["- a\n«- b\n\ntext\n- c»", "- a\n«  - b\n\ntext\n  - c»"],
    // výběr končící na začátku dalšího řádku ten řádek nebere
    ["- a\n«- b\n»- c", "- a\n«  - b\n»- c"],
  ];
  for (const [inp, want] of cases) assert.equal(tab(inp, false), want, inp);
});

test("Shift+Tab: předsadí na úroveň rodiče", () => {
  const cases = [
    ["- a\n  - «b»", "- a\n- «b»"],
    ["1. a\n   1. «b»", "1. a\n1. «b»"],
    ["- a\n  - b\n    - «c»", "- a\n  - b\n  - «c»"],
    ["- a\n«  - b\n  - c»", "- a\n«- b\n- c»"],
    ["  - «a»", "- «a»"],
    ["- «a»", "- «a»"], // už vlevo → beze změny
    ["- [ ] a\n  - [ ] «b»", "- [ ] a\n- [ ] «b»"],
  ];
  for (const [inp, want] of cases) assert.equal(tab(inp, true), want, inp);
});

test("Tab mimo seznam nedělá nic (null) — fokus se přesune normálně", () => {
  assert.equal(onTab("obyčejný text", 3, 3, false), null);
  assert.equal(onTab("a\n\nb", 0, 4, true), null);
  assert.equal(onTab("", 0, 0, false), null);
});
