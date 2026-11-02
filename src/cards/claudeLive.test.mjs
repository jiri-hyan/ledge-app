import test from "node:test";
import assert from "node:assert/strict";
import { workingList, pickWorking, elapsed, stripIndex, fileParts, clipMessage, doneList, doneAt, openWork } from "./claudeLive.ts";

const r = (id, status, o = {}) => ({ id, status, since: null, updatedAt: 0, uncommitted: [], unpushed: 0, activity: null, ...o });
const act = (phase, at = 0, extra = {}) => ({ phase, tool: "", file: null, message: null, at, ...extra });

test("workingList: jen pracující, nejčerstvější první", () => {
  const l = [r("a", "working", { updatedAt: 1 }), r("b", "idle", { updatedAt: 9 }), r("c", "working", { updatedAt: 5 })];
  assert.deepEqual(workingList(l).map((x) => x.id), ["c", "a"]);
  assert.deepEqual(workingList([]), []);
});

test("pickWorking: vybraná, jinak nejčerstvější, jinak null", () => {
  const l = [r("a", "working", { updatedAt: 1 }), r("c", "working", { updatedAt: 5 })];
  assert.equal(pickWorking(l, "a").id, "a");
  assert.equal(pickWorking(l, "zmizela").id, "c");
  assert.equal(pickWorking(l, null).id, "c");
  assert.equal(pickWorking([r("x", "idle")], "x"), null);
});

test("elapsed: formát času", () => {
  const T = 1_000_000_000;
  for (const [ms, out] of [[0, "0:00"], [4000, "0:04"], [65_000, "1:05"], [3_599_000, "59:59"], [3_600_000, "1:00:00"], [3_725_000, "1:02:05"], [-5000, "0:00"]])
    assert.equal(elapsed(T, T + ms), out, String(ms));
  assert.equal(elapsed(null, T), null);
  assert.equal(elapsed(undefined, T), null);
});

test("stripIndex: fáze z pásu, neznámé nezvýrazní nic", () => {
  assert.equal(stripIndex(r("a", "working", { activity: act("reading") })), 0);
  assert.equal(stripIndex(r("a", "working", { activity: act("editing") })), 1);
  assert.equal(stripIndex(r("a", "working", { activity: act("testing") })), 2);
  assert.equal(stripIndex(r("a", "idle", { activity: act("done") })), 3);
  assert.equal(stripIndex(r("a", "working", { activity: act("done") })), -1);
  assert.equal(stripIndex(r("a", "working", { activity: act("working") })), -1);
  assert.equal(stripIndex(r("a", "working", { activity: act("error") })), -1);
  assert.equal(stripIndex(r("a", "working")), -1);
});

test("fileParts: název a poslední složka", () => {
  assert.deepEqual(fileParts("C:\\r\\src\\main.rs"), { name: "main.rs", dir: "src" });
  assert.deepEqual(fileParts("/a/b/c.ts"), { name: "c.ts", dir: "b" });
  assert.deepEqual(fileParts("x.ts"), { name: "x.ts", dir: null });
  assert.equal(fileParts(""), null);
  assert.equal(fileParts(null), null);
  assert.equal(fileParts("///"), null);
});

test("clipMessage: slití mezer, zkrácení, prázdné", () => {
  assert.equal(clipMessage("  Opraveno\n\n  dvě chyby  "), "Opraveno dvě chyby");
  assert.equal(clipMessage("abcdefghij", 5), "abcd…");
  assert.equal(clipMessage("abcde", 5), "abcde");
  assert.equal(clipMessage("ab cd ef", 4), "ab…");
  assert.equal(clipMessage("   "), null);
  assert.equal(clipMessage(null), null);
  assert.equal(Array.from(clipMessage("😀".repeat(200), 10)).length, 10);
});

test("doneList: hotové nejnovější nahoře, pracující a čekající ne", () => {
  const l = [
    r("old", "idle", { since: 100 }),
    r("new", "idle", { activity: act("done", 900) }),
    r("w", "working", { activity: act("done", 999) }),
    r("q", "waiting"),
    r("e", "ended", { since: 500, unpushed: 1 }),
    r("d", "idle", { since: 50, activity: act("done", 700) }),
  ];
  assert.deepEqual(doneList(l).map((x) => x.id), ["new", "d", "e", "old"]);
  assert.equal(doneAt(l[5]), 700);
  assert.equal(doneAt(r("z", "idle", { updatedAt: 7 })), 7);
});

test("openWork: štítek jen s nedokončenou prací", () => {
  assert.equal(openWork(r("a", "idle")), null);
  assert.deepEqual(openWork(r("a", "idle", { uncommitted: ["a", "b"] })), { files: 2, unpushed: 0 });
  assert.deepEqual(openWork(r("a", "idle", { unpushed: 3 })), { files: 0, unpushed: 3 });
});
