// Hlavička poznámky: projekt jen když je, datum vytvoření, prázdné části vypadnou.
import { test } from "node:test";
import assert from "node:assert/strict";
import { headline, headlineParts, shortDate } from "./headline.ts";

const ms = new Date(2026, 9, 26, 14, 3).getTime();

test("shortDate: česky i anglicky, neplatné prázdné", () => {
  assert.equal(shortDate(ms), "26. 10.");
  assert.equal(shortDate(ms, "en"), "10/26");
  assert.equal(shortDate(0), "");
  assert.equal(shortDate(NaN), "");
});

test("headline: Projekt · Název · datum", () => {
  const cases = [
    [{ title: "Plán týdne", createdMs: ms }, "app", "app · Plán týdne · 26. 10."],
    [{ title: "Plán týdne", createdMs: ms }, null, "Plán týdne · 26. 10."],
    [{ title: "Plán týdne", createdMs: 0 }, "app-v2", "app-v2 · Plán týdne"],
    [{ title: "", createdMs: ms }, null, "26. 10."],
    [{ title: "  x  ", createdMs: ms }, "  ", "x · 26. 10."],
  ];
  for (const [n, p, want] of cases) assert.equal(headline(n, p), want);
  assert.deepEqual(headlineParts({ title: "A", createdMs: ms }, "app"), ["app", "A", "26. 10."]);
});

test("firstLine: bez značek a obrázků, prázdné tělo prázdné", async () => {
  const { firstLine } = await import("./headline.ts");
  const cases = [
    ["# Nákup\n- mléko", "Nákup"],
    ["\n![](a.png)\n- [ ] **zavolat** dům", "zavolat dům"],
    ["```\nkód\n```", "kód"],
    ["", ""],
    ["   \n\n", ""],
  ];
  for (const [body, want] of cases) assert.equal(firstLine(body), want, body);
});
