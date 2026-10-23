// Tabulkové testy předfiltru a převodů nabídky připomínky (`suggest.ts`). Bez transpilace (Node 22.6+).
import { test } from "node:test";
import assert from "node:assert/strict";
import { looksTimed, wantsAsk, toLocalValue, visibleSuggest } from "./suggest.ts";

test("looksTimed: číslo nebo časové slovo, jinak nic", () => {
  const cases = [
    ["Zavolat zítra v 10", true],
    ["ZÍTRA", true],
    ["v pondělí porada", true],
    ["ve čtvrtek", true],
    ["za týden", true],
    ["večer nakoupit", true],
    ["25. 10. odevzdat", true],
    ["ráno", true],
    ["dnes", true],
    ["nákup: mléko, chleba", false],
    ["Zavolat Novákovi", false],
    ["ok", false],
    ["", false],
    ["   ", false],
  ];
  for (const [text, want] of cases) assert.equal(looksTimed(text), want, JSON.stringify(text));
});

test("wantsAsk: jen bez připomínky a jednou pro stejný text", () => {
  assert.equal(wantsAsk("zítra v 10", "", ""), true);
  assert.equal(wantsAsk("zítra v 10", "2026-10-24T10:00", ""), false, "už má připomínku");
  assert.equal(wantsAsk("zítra v 10", "", "zítra v 10"), false, "na tenhle text se už ptalo");
  assert.equal(wantsAsk("zítra v 11", "", "zítra v 10"), true);
  assert.equal(wantsAsk("nic", "", ""), false);
});

test("toLocalValue: lokální čas z ISO s offsetem", () => {
  assert.equal(toLocalValue("2026-10-24T10:00:00+02:00"), "2026-10-24T10:00");
  assert.equal(toLocalValue("2026-12-24T09:30:00+01:00"), "2026-12-24T09:30");
  assert.equal(toLocalValue("blbost"), null);
  assert.equal(toLocalValue(""), null);
});

test("visibleSuggest: skryje zavřený návrh i návrh po nastavení připomínky", () => {
  const s = { label: "zítra 10:00", iso: "2026-10-24T10:00:00+02:00" };
  assert.equal(visibleSuggest(s, "", ""), s);
  assert.equal(visibleSuggest(s, "", s.iso), null);
  assert.equal(visibleSuggest(s, "2026-10-24T10:00", ""), null);
  assert.equal(visibleSuggest(null, "", ""), null);
  assert.equal(visibleSuggest({ label: "x", iso: "?" }, "", ""), null);
});
