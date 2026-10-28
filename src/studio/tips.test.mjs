import test from "node:test";
import assert from "node:assert/strict";
import { parseVersion, cmpVersion, newsState, TIPS, NEWS } from "./tips.ts";

test("parseVersion", () => {
  for (const [i, o] of [["0.4.1", [0, 4, 1]], ["v1.2", [1, 2]], ["", null], ["abc", null], [undefined, null], ["1.2.3.4.5", null]]) assert.deepEqual(parseVersion(i), o, String(i));
});
test("cmpVersion", () => {
  for (const [a, b, o] of [["0.4.0", "0.4.1", -1], ["0.4", "0.4.0", 0], ["1.0", "0.9.9", 1], ["x", "1.0", null]]) assert.equal(cmpVersion(a, b), o, `${a} ${b}`);
});
test("newsState: poprvé, upgrade, beze změny", () => {
  for (const [seen, cur, o] of [[null, "0.5.0", "first"], ["junk", "0.5.0", "first"], ["0.4.0", "0.5.0", "show"], ["0.5.0", "0.5.0", "none"], ["0.6.0", "0.5.0", "none"], ["0.4.0", undefined, "none"]]) {
    assert.equal(newsState(seen, cur), o, `${seen} → ${cur}`);
  }
});
test("tipy a novinky mají jedinečná id", () => {
  assert.equal(new Set(TIPS.map((x) => x.id)).size, TIPS.length);
  assert.equal(new Set(NEWS.map((x) => x.id)).size, NEWS.length);
});
