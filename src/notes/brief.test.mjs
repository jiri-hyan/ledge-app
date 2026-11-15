// Zadání (`brief.ts`): body [ ] / [~] / [x], snímky, otázky, akce tlačítka, náklad, @ odkazy, kotva snímku.
import { test } from "node:test";
import assert from "node:assert/strict";
import { allAnswered, anchorAt, applyAt, atToken, briefAction, cleanCaption, completesBrief, costText, saveSendOffer, logLines, parseQuestions, parseTasks, resolveAnchor, splitAtRefs, taskLine } from "./brief.ts";

const BODY = [
  "Přidat export do CSV",
  "",
  "- [ ] první bod @src/a.ts:42",
  "- [~] druhý — důkaz: test prošel",
  "  ![tabulka](./s-1.png)",
  "  ![](./s-2.png)",
  "1. [x] třetí",
  "2. [~] bez důkazu",
  "",
  "## Otázky",
  "- ❓ Který oddělovač?",
  "  - ↳ středník",
  "- ❓ Kódování?",
  "",
  "## Log",
  "10:00 start",
  "10:05 hotovo",
].join("\n");

test("parseTasks: stav, text, důkaz, snímky pod bodem", () => {
  const ts = parseTasks(BODY);
  assert.deepEqual(ts.map((t) => [t.line, t.state, t.text, t.evidence, t.images.length]), [
    [2, "open", "první bod @src/a.ts:42", null, 0],
    [3, "review", "druhý", "test prošel", 2],
    [6, "done", "třetí", null, 0],
    [7, "review", "bez důkazu", null, 0],
  ]);
  assert.deepEqual(ts[1].images[0], { line: 4, caption: "tabulka", src: "./s-1.png" });
  assert.equal(ts[1].images[1].caption, "");
});

test("taskLine: Potvrdit zachová důkaz, Vrátit ho zahodí", () => {
  const t = parseTasks(BODY)[1];
  assert.equal(taskLine(t, "done"), "- [x] druhý — důkaz: test prošel");
  assert.equal(taskLine(t, "open"), "- [ ] druhý");
  assert.equal(taskLine({ ...t, indent: 2 }, "review"), "  - [~] druhý — důkaz: test prošel");
});

test("parseQuestions: odpověď pod otázkou, index = pořadí", () => {
  const qs = parseQuestions(BODY);
  assert.deepEqual(qs.map((q) => [q.index, q.text, q.answer]), [[0, "Který oddělovač?", "středník"], [1, "Kódování?", null]]);
  assert.equal(allAnswered(qs), false);
  assert.equal(allAnswered(qs.map((q) => ({ ...q, answer: "x" }))), true);
  assert.equal(allAnswered([]), false);
  assert.deepEqual(parseQuestions("- ❓ mimo sekci"), []);
});

test("logLines: jen sekce Log", () => {
  assert.deepEqual(logLines(BODY), ["10:00 start", "10:05 hotovo"]);
  assert.deepEqual(logLines("nic"), []);
});

test("briefAction: stavy a důvody blokace", () => {
  const n = (o) => ({ project: "app", body: "cíl\n- [ ] a", ...o });
  const cases = [
    [n({}), true, { kind: "send", block: null }],
    [n({ project: null }), true, { kind: "send", block: "project" }],
    [n({}), false, { kind: "send", block: "key" }],
    [n({}), undefined, { kind: "send", block: null }],
    [n({ body: "" }), true, { kind: "send", block: "goal" }],
    [n({ body: "## Otázky" }), true, { kind: "send", block: "goal" }],
    [n({ status: "draft" }), true, { kind: "send", block: null }],
    [n({ status: "running" }), true, { kind: "stop" }],
    [n({ status: "waiting", body: BODY }), true, { kind: "resume", block: "answers" }],
    [n({ status: "waiting", body: "x\n## Otázky\n- ❓ a\n  - ↳ b" }), true, { kind: "resume", block: null }],
    [n({ status: "review" }), true, { kind: "resume", block: null }], // po Vrátit (žádné [~]) jde Pokračovat
    [n({ status: "review", body: BODY }), true, { kind: "resume", block: "review" }], // [~] čeká na potvrzení
    [n({ status: "review", body: "cíl\n- [x] a" }), undefined, { kind: "resume", block: null }],
    [n({ status: "done" }), true, { kind: "none" }],
    [n({ status: "done", body: BODY }), true, { kind: "none" }],
  ];
  for (const [note, key, want] of cases) assert.deepEqual(briefAction(note, key), want);
});

test("completesBrief: jen poslední Potvrdit v review dokončí zadání", () => {
  const ts = (body) => parseTasks(body);
  const one = ts("cíl\n- [~] a — důkaz: x");
  const two = ts("cíl\n- [~] a\n- [~] b");
  const mixed = ts("cíl\n- [x] a\n- [~] b");
  const open = ts("cíl\n- [ ] a\n- [~] b");
  const cases = [
    ["review", one, 1, true],
    ["review", mixed, 2, true],
    ["review", two, 1, false], // druhý bod ještě čeká
    ["review", open, 2, false], // jeden se vrátil / zůstal otevřený
    ["review", one, 5, false], // řádek není bod ke kontrole
    ["review", [], 0, false],
    ["review", mixed, 1, false], // `[x]` už potvrzený, ne ke kontrole
    ["waiting", one, 1, false],
    ["running", one, 1, false],
    ["done", one, 1, false],
    [undefined, one, 1, false],
  ];
  for (const [st, tasks, line, want] of cases) assert.equal(completesBrief(st, tasks, line), want, `${st} ${line}`);
});

test("saveSendOffer: Uložit a poslat jen u zadání, které jde poslat", () => {
  const n = (o) => ({ brief: true, editing: false, dirty: false, project: "app", body: "cíl", ...o });
  const cases = [
    [n({}), true, { show: true, block: null }],
    [n({}), undefined, { show: true, block: null }],
    [n({ project: null }), true, { show: true, block: "project" }],
    [n({ body: "" }), true, { show: true, block: "goal" }],
    [n({}), false, { show: true, block: "key" }],
    [n({ brief: false }), true, { show: false, block: null }], // obyčejná poznámka
    [n({ editing: true, status: "draft" }), true, { show: false, block: null }], // bez úpravy stačí tlačítko panelu
    [n({ editing: true, dirty: true, status: "draft" }), true, { show: true, block: null }],
    [n({ editing: true, dirty: true, status: "running" }), true, { show: false, block: null }],
    [n({ editing: true, dirty: true, status: "waiting" }), true, { show: false, block: null }], // tam je Pokračovat
    [n({ editing: true, dirty: true, status: "done" }), true, { show: false, block: null }],
  ];
  for (const [note, key, want] of cases) assert.deepEqual(saveSendOffer(note, key), want, JSON.stringify(note));
});

test("costText: jen skutečná pole", () => {
  const cases = [[1.234, 5, "$1.23 / $5.00"], [0, undefined, "$0.00"], [undefined, 5, "limit $5.00"], [undefined, undefined, null], [null, null, null], [NaN, undefined, null]];
  for (const [c, b, want] of cases) assert.equal(costText(c, b), want);
});

test("splitAtRefs: cesta, řádek, interpunkce, ne @jmeno ani e-mail", () => {
  const cases = [
    ["viz @src/a.ts:42 a dál", [["viz ", null], ["@src/a.ts:42", ["src/a.ts", 42]], [" a dál", null]]],
    ["(@README.md)", [["(", null], ["@README.md", ["README.md", null]], [")", null]]],
    ["hotovo v @src/b.rs.", [["hotovo v ", null], ["@src/b.rs", ["src/b.rs", null]], [".", null]]],
    ["@jmeno ahoj", [["@jmeno ahoj", null]]],
    ["mail uzivatel@example.com", [["mail uzivatel@example.com", null]]],
    ["@a\\b\\c.ts", [["@a\\b\\c.ts", ["a/b/c.ts", null]]]],
  ];
  for (const [text, want] of cases) assert.deepEqual(splitAtRefs(text).map((p) => [p.text, p.ref ? [p.ref.path, p.ref.line] : null]), want, text);
});

test("atToken + applyAt: token pod kurzorem a jeho náhrada", () => {
  assert.deepEqual(atToken("ahoj @sr", 8), { start: 5, query: "sr" });
  assert.deepEqual(atToken("@", 1), { start: 0, query: "" });
  assert.equal(atToken("a@b", 3), null);
  assert.equal(atToken("ahoj @sr x", 10), null);
  assert.equal(atToken("ahoj", 4), null);
  const tok = atToken("ahoj @sr dál", 8);
  assert.deepEqual(applyAt("ahoj @sr dál", tok, 8, "src/a.ts"), { text: "ahoj @src/a.ts dál", caret: 15 });
  assert.deepEqual(applyAt("@s", { start: 0, query: "s" }, 2, "a.ts"), { text: "@a.ts ", caret: 6 });
});

test("anchorAt: řádek s kurzorem, na prázdném / snímkovém nejbližší nad ním", () => {
  const b = "cíl\n- [ ] a\n  ![x](./1.png)\n\n- [ ] b";
  const at = (c) => anchorAt(b, c)?.line;
  assert.equal(at(0), 0);
  assert.equal(at(8), 1); // uvnitř „- [ ] a“
  assert.equal(at(b.indexOf("![")), 1); // na snímku → bod nad ním
  assert.equal(at(b.indexOf("\n\n") + 1), 1); // prázdný řádek
  assert.equal(at(b.length), 4);
  assert.equal(anchorAt("", 0), null);
  assert.equal(anchorAt("\n\n", 1), null);
});

test("resolveAnchor: řádek platí, jen když tam text ještě je", () => {
  const a = { line: 1, text: "- [ ] a" };
  assert.equal(resolveAnchor("x\n- [ ] a", a), 1);
  assert.equal(resolveAnchor("nový\nx\n- [ ] a", a), 2);
  assert.equal(resolveAnchor("x\ny", a), undefined);
  assert.equal(resolveAnchor("x", null), undefined);
});

test("cleanCaption", () => assert.equal(cleanCaption(" a [b]\nc "), "a b c"));

test("briefError: neznámý příkaz lidsky, jinou chybu nechá", async () => {
  const { briefError } = await import("./brief.ts");
  assert.equal(briefError("Command brief_send not found", "stará verze"), "stará verze");
  assert.equal(briefError("Chybí klíč", "stará verze"), "Chybí klíč");
});

test("imageIn: holé PNG bez popisku a kotvy, jinak objekt Rustu", async () => {
  const { imageIn } = await import("./brief.ts");
  assert.equal(imageIn("AAA"), "AAA");
  assert.equal(imageIn("AAA", ""), "AAA");
  assert.deepEqual(imageIn("AAA", "tabulka", 4), { png: "AAA", caption: "tabulka", anchorLine: 4 });
  assert.deepEqual(imageIn("AAA", "", 0), { png: "AAA", anchorLine: 0 });
  assert.deepEqual(imageIn("AAA", "jen popisek"), { png: "AAA", caption: "jen popisek" });
});
