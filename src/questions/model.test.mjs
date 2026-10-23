// Tabulkové testy čisté logiky karty otázky Clauda (`model.ts`): co UI pošle do `question_answer`.
//   node --experimental-strip-types --test src/questions/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { answered, emptyDraft, instant, keyTarget, pick, toReply, toggleOther } from "./model.ts";

const q = (multiSelect, labels = ["Testy", "Lint", "Build"]) => ({
  question: "Co spustit?",
  header: "Kroky",
  options: labels.map((label) => ({ label, description: "", preview: null })),
  multiSelect,
});

test("vícevýběr: 2+ kliknutí = všechny zaškrtnuté v odpovědi, druhý klik odškrtne", () => {
  const m = q(true);
  let d = emptyDraft();
  for (const l of ["Testy", "Build"]) d = pick(m, d, l);
  assert.deepEqual(toReply(d), { labels: ["Testy", "Build"], other: null });
  d = pick(m, d, "Lint");
  d = pick(m, d, "Testy");
  assert.deepEqual(toReply(d).labels, ["Build", "Lint"]);
  // klávesy 1–n přepínají stejně jako klik
  for (const k of ["1", "2"]) {
    const t = keyTarget(m, k);
    d = pick(m, d, t.label);
  }
  assert.deepEqual(toReply(d).labels, ["Build", "Testy"]);
  assert.equal(instant([m]), false, "vícevýběr se neodesílá prvním klikem");
});

test("vícevýběr + Jiné…: vlastní text navíc k možnostem, prázdný se nepočítá", () => {
  const m = q(true);
  let d = pick(m, emptyDraft(), "Lint");
  d = toggleOther(m, d);
  assert.deepEqual(toReply({ ...d, other: "   " }), { labels: ["Lint"], other: null });
  assert.deepEqual(toReply({ ...d, other: " řádek 1\nřádek 2 🙂 " }), { labels: ["Lint"], other: "řádek 1\nřádek 2 🙂" });
  // zavřené Jiné… = text se neposílá
  assert.deepEqual(toReply({ ...toggleOther(m, d), other: "x" }), { labels: ["Lint"], other: null });
});

test("jednovýběr: jen poslední klik, Jiné… výběr zruší; prázdná odpověď nejde odeslat", () => {
  const m = q(false);
  let d = pick(m, pick(m, emptyDraft(), "Testy"), "Lint");
  assert.deepEqual(toReply(d).labels, ["Lint"]);
  d = toggleOther(m, d);
  assert.deepEqual(d.labels, []);
  assert.equal(answered(d), false);
  assert.equal(answered({ ...d, other: "vlastní" }), true);
  assert.equal(instant([m]), true);
  assert.equal(instant([m, m]), false, "víc otázek = stránky, ne okamžité odeslání");
});

test("keyTarget: čísla možností, za posledním Jiné…, jinak nic", () => {
  const m = q(false, ["A", "B"]);
  assert.deepEqual(keyTarget(m, "2"), { label: "B" });
  assert.equal(keyTarget(m, "3"), "other");
  for (const k of ["4", "0", "a", "Enter"]) assert.equal(keyTarget(m, k), null);
});

test("„zobrazeno“ jen u držené otázky, otevřeného šuplíku a karty v záběru, jednou", async () => {
  const { shouldReportSeen } = await import("./model.ts");
  const base = { answerable: true, shown: true, inView: true, reported: false };
  const cases = [
    ["vše splněno", base, true],
    ["jen ukázaná (odpovídá se v Claude)", { ...base, answerable: false }, false],
    ["šuplík zavřený / WebView skryté", { ...base, shown: false }, false],
    ["karta mimo záběr", { ...base, inView: false }, false],
    ["už potvrzeno", { ...base, reported: true }, false],
  ];
  for (const [name, s, want] of cases) assert.equal(shouldReportSeen(s), want, name);
});
