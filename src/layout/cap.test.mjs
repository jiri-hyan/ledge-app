// Strop řádků karty + „Zobrazit další“ (`cap.ts`) a pořadí session v kartě Claude (`sessions/order.ts`).
//   node --experimental-strip-types --test src/layout/cap.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { capKey, capList } from "./cap.ts";
import { hiddenSummary, orderSessions } from "../sessions/order.ts";

test("strop: nejvýš N řádků, připnuté se nikdy neschovají, 0 = bez stropu, rozbaleno = vše", () => {
  const items = ["a", "b", "c", "d", "e", "f"];
  const pin = (x) => x === "e" || x === "f";
  const cases = [
    ["4 z 6", 4, false, undefined, ["a", "b", "c", "d"], ["e", "f"]],
    ["rozbaleno", 4, true, undefined, items, []],
    ["bez stropu", 0, false, undefined, items, []],
    ["méně než strop", 10, false, undefined, items, []],
    ["připnuté zaberou místo, ale neschovají se", 3, false, pin, ["a", "e", "f"], ["b", "c", "d"]],
    ["připnutých víc než strop: ukážou se všechny", 1, false, pin, ["e", "f"], ["a", "b", "c", "d"]],
  ];
  for (const [name, limit, open, pinned, shown, hidden] of cases) assert.deepEqual(capList(items, limit, open, pinned), { shown, hidden }, name);
  assert.equal(capKey("prace", "claude"), "prace:claude");
  assert.notEqual(capKey("prace", "claude"), capKey("osobni", "claude"), "rozbalení zvlášť pro každou záložku");
});

test("session: čeká na tebe > pracuje > nečinná (poslední aktivita) > skončila", () => {
  const s = (id, status, updatedAt) => ({ id, status, updatedAt, since: null });
  const list = [s("i-old", "idle", 100), s("w1", "working", 50), s("end", "ended", 900), s("i-new", "idle", 300), s("ask", "idle", 10), s("wait", "waiting", 20)];
  // „ask“ je nečinná, ale má otázku Clauda → čeká na tebe
  const got = orderSessions(list, (id) => id === "ask");
  assert.deepEqual(got.map((x) => x.id), ["wait", "ask", "w1", "i-new", "i-old", "end"]);
  // šéf: 10 session, strop 4 — čekající projdou vždy, zbytek se sbalí se souhrnem
  const many = [...Array(8)].map((_, i) => s(`i${i}`, "idle", i)).concat([s("w", "working", 1), s("q", "waiting", 1)]);
  const ord = orderSessions(many, () => false);
  const c = capList(ord, 4, false, (x) => x.status === "waiting");
  assert.deepEqual(c.shown.map((x) => x.id), ["q", "w", "i7", "i6"]);
  assert.deepEqual(hiddenSummary(c.hidden), { working: 0, idle: 6, ended: 0 });
  assert.deepEqual(hiddenSummary([s("a", "working", 1), s("b", "ended", 1), s("c", "idle", 1)]), { working: 1, idle: 1, ended: 1 });
});
