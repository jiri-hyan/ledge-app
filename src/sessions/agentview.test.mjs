// Tabulkové testy čisté logiky Agent View (`agentview.ts`): popisky waitingFor / stavu úlohy, pořadí
// a potvrzení akcí, kontrola úkolu „Claude na pozadí…“. Nic nespouští ani nečte `~/.claude`.
//   node --experimental-strip-types --test src/sessions/agentview.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { NAME_MAX, PROMPT_MAX, actionLabel, bgKey, confirmText, nameProblem, needsConfirm, promptProblem, shownActions, statusPill, waitText } from "./agentview.ts";
import { setLang } from "../i18n.ts";

// anglické texty hlídá src/i18n.test.mjs (slovník en-agentview.ts)
setLang("cs");

test("waitingFor → český text (všech pět hodnot z docs)", () => {
  const cases = [
    ["permission", "čeká: oprávnění"],
    ["input", "čeká: vstup"],
    ["sandbox", "čeká: sandbox"],
    ["worker", "čeká: worker"],
    ["dialog", "čeká: dialog"],
  ];
  for (const [w, cs] of cases) assert.equal(waitText(w), cs, w);
});

test("pilulka stavu: důvod čekání, skončené úlohy, jinak obecná", () => {
  const job = (state) => ({ id: "7c5dcf5d", state, actions: [] });
  const cases = [
    [{ status: "waiting", waitingFor: "permission", job: null }, ["a", "čeká: oprávnění"]],
    // čeká bez známého důvodu → obecná „čeká na tebe“
    [{ status: "waiting", waitingFor: null, job: job("blocked") }, null],
    [{ status: "ended", waitingFor: null, job: job("failed") }, ["r", "selhala"]],
    [{ status: "ended", waitingFor: null, job: job("stopped") }, ["n", "zastavena"]],
    [{ status: "ended", waitingFor: null, job: job("done") }, ["n", "hotovo"]],
    [{ status: "ended", waitingFor: null, job: job(null) }, null],
    [{ status: "ended", waitingFor: null, job: null }, null],
    // done s živým procesem je nečinná — pilulka obecná
    [{ status: "idle", waitingFor: null, job: job("done") }, null],
    [{ status: "working", waitingFor: "input", job: null }, null],
  ];
  for (const [ss, want] of cases) assert.deepEqual(statusPill(ss), want, JSON.stringify(ss));
});

test("akce: jen nabídnuté Rustem, pevné pořadí, potvrzení u měnících", () => {
  assert.deepEqual(shownActions(null), []);
  assert.deepEqual(shownActions(undefined), []);
  assert.deepEqual(shownActions({ id: "x1y2", state: "done", actions: ["remove", "logs", "attach", "stop", "respawn"] }), ["attach", "logs", "respawn", "stop", "remove"]);
  assert.deepEqual(shownActions({ id: "x1y2", state: "failed", actions: ["attach", "logs", "remove", "exec"] }), ["attach", "logs", "remove"], "neznámou akci neukáže");
  for (const [a, c] of [["attach", false], ["logs", false], ["stop", true], ["remove", true], ["respawn", true]]) {
    assert.equal(needsConfirm(a), c, a);
    assert.ok(actionLabel(a).length > 0, a);
  }
  assert.equal(confirmText("stop", "flaky"), "Zastavit „flaky“?");
  assert.match(confirmText("remove", "flaky"), /worktree/);
  assert.equal(confirmText("logs", "flaky"), "");
});

test("úkol a jméno pro claude --bg: stejná pravidla jako Rust", () => {
  assert.equal(promptProblem("oprav testy"), null);
  assert.equal(promptProblem("  a & b | \"c\" %PATH%  "), null, "metaznaky jsou data");
  assert.ok(promptProblem("   "));
  assert.ok(promptProblem(" --dangerously-skip-permissions"));
  assert.ok(promptProblem("x".repeat(PROMPT_MAX + 1)));
  assert.equal(promptProblem("ž".repeat(PROMPT_MAX)), null, "strop ve znacích");
  assert.equal(nameProblem(""), null);
  assert.equal(nameProblem("flaky-fix"), null);
  assert.ok(nameProblem("-x"));
  assert.ok(nameProblem("n".repeat(NAME_MAX + 1)));
});

test("klávesy pole úkolu", () => {
  const cases = [["Enter", true, "send"], ["Enter", false, null], ["Escape", false, "cancel"], ["a", true, null]];
  for (const [k, ctrl, want] of cases) assert.equal(bgKey(k, ctrl), want, `${k} ${ctrl}`);
});
