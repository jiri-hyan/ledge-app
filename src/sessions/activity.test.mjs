// Karta „Claude aktivita“ (`activity.ts`): které session, pořadí, fáze a kroky. Bez DOM.
//   node --experimental-strip-types --test src/sessions/activity.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { PHASES, baseName, livePhase, activityApproves, inboxShown, orderActivity, shown } from "./activity.ts";

const act = (phase, at) => ({ phase, tool: "", file: null, message: null, at });
const row = (id, status, updatedAt, activity = null) => ({ id, status, updatedAt, activity });

test("fáze: kroky a zrcadlo Rust Phase", () => {
  const rs = readFileSync(new URL("../../src-tauri/src/claude/activity.rs", import.meta.url), "utf8");
  const body = rs.match(/pub enum Phase \{([\s\S]*?)\n\}/)[1];
  const variants = [...body.matchAll(/^\s{4}([A-Z]\w*),/gm)].map((m) => m[1][0].toLowerCase() + m[1].slice(1));
  assert.deepEqual(variants, [...PHASES]);
});

test("baseName", () => {
  for (const [p, b] of [["C:\\r\\src\\main.rs", "main.rs"], ["/r/a.ts", "a.ts"], ["a.md", "a.md"], ["C:/r/dir/", "dir"]]) assert.equal(baseName(p), b);
});

test("prázdno bez sessions a bez aktivity", () => {
  assert.deepEqual(orderActivity([], new Set()), []);
  // session bez hooků (jen registr) a skončená se neukáže; čekající ano i bez aktivity
  assert.equal(shown(row("a", "idle", 1), new Set()), false);
  assert.equal(shown(row("a", "ended", 1, act("done", 1)), new Set()), false);
  assert.equal(shown(row("a", "waiting", 1), new Set(["a"])), true);
});

test("pořadí: čeká > pracuje > zbytek, pak čerstvost fáze", () => {
  const list = [
    row("idle", "idle", 50, act("done", 50)),
    row("work-old", "working", 10, act("editing", 10)),
    row("work-new", "working", 20, act("testing", 30)),
    row("wait", "waiting", 1, act("editing", 1)),
    row("none", "idle", 99),
  ];
  assert.deepEqual(orderActivity(list, new Set(["wait"])).map((r) => r.id), ["wait", "work-new", "work-old", "idle"]);
});

test("livePhase: zastaralá rozpracovaná fáze se neukáže", () => {
  const cases = [
    [row("a", "working", 1, act("editing", 1)), "editing"],
    [row("a", "waiting", 1, act("testing", 1)), "testing"],
    [row("a", "idle", 1, act("done", 1)), "done"],
    [row("a", "idle", 1, act("editing", 1)), null],
    [row("a", "working", 1), null],
  ];
  for (const [r, want] of cases) assert.equal(livePhase(r), want, JSON.stringify(r));
});

test("ActivityCard schvaluje jen bez karty Claude; ClaudeInbox zmizí s kartou claude i activity", () => {
  const on = (...ids) => (id) => ids.includes(id);
  // [karty na záložce, activityApproves, inboxShown]
  const table = [
    [[], true, true],
    [["activity"], true, false],
    [["claude"], false, false],
    [["claude", "activity"], false, false],
    [["notes"], true, true],
  ];
  for (const [ids, ap, inbox] of table) {
    assert.equal(activityApproves(on(...ids)), ap, `approvals ${ids}`);
    assert.equal(inboxShown(on(...ids)), inbox, `inbox ${ids}`);
  }
});

test("hooksMissing: poctivý stav bez hooků", async () => {
  const { hooksMissing } = await import("./activity.ts");
  const table = [
    [{ state: "none", missing: [] }, true],
    [{ state: "partial", missing: ["PostToolUse"] }, true],
    [{ state: "partial", missing: ["PreToolUse"] }, false],
    [{ state: "full", missing: [] }, false],
    [{ state: "error", missing: [] }, false],
    [null, false],
  ];
  for (const [h, want] of table) assert.equal(hooksMissing(h), want, JSON.stringify(h));
});
