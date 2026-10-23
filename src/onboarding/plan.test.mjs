// Tabulkové testy plánu průvodce (`plan.ts`): které kroky se ukážou, odhad času, řádky souhrnu.
//   node --experimental-strip-types --test src/onboarding/plan.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { STEPS, doneRows, estimateMinutes, visibleSteps } from "./plan.ts";

const ids = (f) => visibleSteps(f).map((s) => s.id);

test("kroky: Claude je poslední před Hotovo a jen s nainstalovaným Claude Code", () => {
  const cases = [
    ["bez Clauda, motivy jsou", { themesReady: true, themeCount: 5, claudeInstalled: false }, ["welcome", "projects", "apps", "links", "look", "done"]],
    ["s Claudem", { themesReady: true, themeCount: 5, claudeInstalled: true }, ["welcome", "projects", "apps", "links", "look", "claude", "done"]],
    ["bez motivů", { themesReady: true, themeCount: 0, claudeInstalled: true }, ["welcome", "projects", "apps", "links", "claude", "done"]],
    ["motivy se ještě načítají", { themesReady: false, themeCount: 0, claudeInstalled: false }, ["welcome", "projects", "apps", "links", "look", "done"]],
  ];
  for (const [name, facts, want] of cases) assert.deepEqual(ids(facts), want, name);
  assert.equal(STEPS.some((s) => s.id === "assist"), false, "krok „S Claudem“ už není");
  assert.equal(STEPS.filter((s) => s.id === "claude").length, 1, "jediný krok Claude");
});

test("odhad času: pracovní kroky po půl minutě, nahoru, aspoň minuta", () => {
  const f = (claudeInstalled, themeCount = 5) => visibleSteps({ themesReady: true, themeCount, claudeInstalled });
  assert.equal(estimateMinutes(f(false)), 2, "4 kroky = 2 min");
  assert.equal(estimateMinutes(f(true)), 3, "5 kroků = 2,5 → 3 min");
  assert.equal(estimateMinutes(f(false, 0)), 2, "3 kroky = 1,5 → 2 min");
  assert.equal(estimateMinutes([]), 1, "aspoň minuta");
  assert.equal(estimateMinutes(STEPS.filter((s) => s.id === "welcome" || s.id === "done")), 1);
});

test("souhrn: jen nenulové řádky, motiv jménem a jen když se uložil", () => {
  const zero = { projects: [], links: 0, theme: null, themeSaved: null, hooks: false };
  const names = (s, hooked = false, theme = null) => doneRows(s, hooked, theme).map((r) => r.kind);
  assert.deepEqual(names(zero), [], "nic → žádný řádek (jedna věta v UI)");
  assert.deepEqual(names({ ...zero, projects: ["A", "B"], links: 3 }), ["projects", "links"]);
  assert.deepEqual(names({ ...zero, links: 1, hooks: true }), ["links", "hooks"]);
  assert.deepEqual(names(zero, true), ["hooks"], "hooky zapojené dřív");
  // motiv: jméno (ne id) a ne když se neuložil / jméno neznáme
  assert.deepEqual(doneRows({ ...zero, theme: "nord", themeSaved: true }, false, "Nord"), [{ kind: "theme", name: "Nord" }]);
  assert.deepEqual(names({ ...zero, theme: "nord", themeSaved: false }, false, "Nord"), [], "neuložený motiv se nehlásí");
  assert.deepEqual(names({ ...zero, theme: "nord", themeSaved: true }, false, null), [], "neznámé jméno = bez řádku, ne surové id");
  const many = doneRows({ ...zero, projects: ["1", "2", "3", "4", "5", "6", "7"] }, false, null)[0];
  assert.equal(many.n, 7);
  assert.equal(many.names, "1, 2, 3, 4, 5, 6…");
});
