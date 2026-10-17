// Tabulkové testy čisté logiky náhledu diffu (`diff.ts`): sbalený řádek, znaménka, okraj s čísly.
//   node --experimental-strip-types --test src/sessions/diff.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { counts, expandable, gutter, gutterWidth, headKind, sign } from "./diff.ts";

const L = (kind, old, nw, text = "") => ({ kind, old, new: nw, text });

test("sbalený řádek: počty a druh", () => {
  assert.equal(counts({ added: 12, removed: 3 }), "+12 −3");
  assert.equal(counts({ added: 0, removed: 0 }), "+0 −0");
  for (const [shown, want] of [["diff", "counts"], ["newFile", "newFile"], ["unread", "unread"], ["tooBig", "tooBig"], ["binary", "binary"]])
    assert.equal(headKind({ shown }), want, shown);
  assert.equal(expandable({ lines: [] }), false);
  assert.equal(expandable({ lines: [L("add", null, 1)] }), true);
});

test("znaménka řádků", () => {
  for (const [k, want] of [["add", "+"], ["del", "−"], ["ctx", " "], ["gap", "⋯"]]) assert.equal(sign(k), want, k);
});

test("okraj: odebraný = staré číslo, jinak nové; bez čísel prázdný", () => {
  for (const [l, numbered, want] of [
    [L("del", 7, null), true, "7"],
    [L("add", null, 8), true, "8"],
    [L("ctx", 9, 10), true, "10"],
    [L("gap", null, null), true, ""],
    [L("ctx", 9, 10), false, ""],
  ]) assert.equal(gutter(l, numbered), want, `${l.kind} ${numbered}`);
  const lines = [L("ctx", 98, 98), L("del", 99, null), L("add", null, 99), L("add", null, 100)];
  assert.equal(gutterWidth(lines, true), 3);
  assert.equal(gutterWidth(lines, false), 0);
  assert.equal(gutterWidth([], true), 0);
});
