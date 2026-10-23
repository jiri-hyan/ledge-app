// @vitest-environment node
import { test } from "node:test";
import assert from "node:assert/strict";
import { dirtySummary, isAcked, sigHash } from "./dirty.ts";

const pr = (id, dirty, ahead, muteDirty) => ({ id, name: id, git: dirty == null ? null : { branch: "main", dirty, ahead, behind: 0 }, ...(muteDirty ? { muteDirty } : {}) });

test("souhrn: ztišená repa se nepočítají, ale v seznamu zůstanou", () => {
  const rows = [
    [[], { dirty: 0, unpushed: 0, sig: "", repos: [] }],
    [[pr("orbit", 2, 0)], { dirty: 1, unpushed: 0, sig: "orbit:2:0", repos: ["orbit"] }],
    [[pr("orbit", 2, 1), pr("beacon", 0, 3)], { dirty: 1, unpushed: 2, sig: "orbit:2:1|beacon:0:3", repos: ["orbit", "beacon"] }],
    [[pr("orbit", 2, 1, true), pr("beacon", 0, 3)], { dirty: 0, unpushed: 1, sig: "beacon:0:3", repos: ["orbit", "beacon"] }],
    [[pr("orbit", 0, 0), pr("bez-gitu", null, null)], { dirty: 0, unpushed: 0, sig: "", repos: [] }],
    [[pr("orbit", -1, 0)], { dirty: 0, unpushed: 0, sig: "", repos: [] }],
  ];
  for (const [input, want] of rows) {
    const got = dirtySummary(input);
    assert.deepEqual({ ...got, repos: got.repos.map((x) => x.id) }, want, JSON.stringify(input));
  }
});

test("Přečteno platí jen pro stejný podpis (otisk v configu)", () => {
  const read = sigHash("orbit:2:0");
  assert.match(read, /^[a-z0-9]{1,7}$/, "projde validací gitAlerts.read");
  assert.equal(isAcked("orbit:2:0", read), true);
  assert.equal(isAcked("orbit:3:0", read), false, "změna počtu zvýraznění vrátí");
  assert.equal(isAcked("", ""), false, "nic k upozornění = nic přečteného");
  assert.equal(isAcked("orbit:2:0", undefined), false);
  assert.equal(sigHash(""), "");
  assert.notEqual(sigHash("a:1:0|b:0:1"), sigHash("a:0:1|b:1:0"));
});
