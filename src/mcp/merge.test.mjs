// Politika změny configu zvenku (Claude přes MCP) vůči rozpracovanému Nastavení — `merge.ts`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { changedSections, external } from "./merge.ts";
import { canon } from "../settings/model.ts";

const base = { strip: { custom: "" }, actions: [], mcp: { enabled: false }, appearance: { theme: "dark" } };

test("beze změny, bez úprav, sloučení, konflikt", () => {
  assert.deepEqual(external(base, base, structuredClone(base), canon), { action: "same" });
  const fresh = { ...base, actions: [{ id: "posta" }] };
  assert.deepEqual(external(base, structuredClone(base), fresh, canon), { action: "load" });
  // uživatel rozpracoval vzhled, Claude přidal akci → sloučit, nic se neztratí
  const draft = { ...base, appearance: { theme: "nord" } };
  const m = external(base, draft, fresh, canon);
  assert.equal(m.action, "merge");
  assert.deepEqual(m.draft.actions, [{ id: "posta" }]);
  assert.equal(m.draft.appearance.theme, "nord");
  // uživatel rozpracoval tutéž sekci → konflikt
  const mineToo = { ...base, actions: [{ id: "moje" }] };
  assert.deepEqual(external(base, mineToo, fresh, canon), { action: "conflict", sections: ["actions"] });
});

test("sekce, která venku zmizela, zmizí i z draftu; jen sekce, které se liší", () => {
  const fresh = { ...base };
  delete fresh.mcp;
  const draft = { ...base, strip: { custom: "x" } };
  const m = external(base, draft, fresh, canon);
  assert.equal(m.action, "merge");
  assert.equal("mcp" in m.draft, false);
  assert.deepEqual(changedSections({ a: [] }, { a: [] }, canon), []);
  assert.deepEqual(changedSections({ a: 1, b: 2 }, { a: 1, b: 3 }, canon), ["b"]);
});
