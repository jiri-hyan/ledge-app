// Rychlá poznámka: plán kláves (Esc ukládá, Ctrl+Enter ukládá jen s textem).
import { test } from "node:test";
import assert from "node:assert/strict";
import { blurPlan, DICTATE_BLUR_GRACE_MS, quickKeyPlan } from "./quick.ts";

test("quickKeyPlan: klávesa × text", () => {
  const cases = [
    [{ key: "Escape", ctrl: false, hasText: true }, "save-close"],
    [{ key: "Escape", ctrl: false, hasText: false }, "save-close"], // prázdné: jen zavřít (saveDraft vrátí null)
    [{ key: "Enter", ctrl: true, hasText: true }, "save"],
    [{ key: "Enter", ctrl: true, hasText: false }, "none"],
    [{ key: "Enter", ctrl: false, hasText: true }, "none"], // holý Enter = nový řádek
    [{ key: "a", ctrl: true, hasText: true }, "none"],
    [{ key: "Tab", ctrl: false, hasText: true }, "none"],
  ];
  for (const [e, want] of cases) assert.equal(quickKeyPlan(e), want, JSON.stringify(e));
});

test("blurPlan: blur × diktát (L139)", () => {
  const base = { finished: false, busy: false, pinned: false, dictateUntil: 0, now: 1000 };
  const cases = [
    [base, "commit"],
    [{ ...base, pinned: true }, "flush"],
    [{ ...base, finished: true }, "skip"],
    [{ ...base, busy: true }, "skip"],
    [{ ...base, dictateUntil: 1000 + DICTATE_BLUR_GRACE_MS }, "skip"], // diktát právě začal: blur se neuloží
    [{ ...base, dictateUntil: 1001 }, "skip"],
    [{ ...base, dictateUntil: 1000 }, "commit"], // doba vypršela
    [{ ...base, pinned: true, dictateUntil: 5000 }, "skip"],
  ];
  for (const [s, want] of cases) assert.equal(blurPlan(s), want, JSON.stringify(s));
});
