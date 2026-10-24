// Lístky 2.0 — výběr stylu (`styles.ts`) a „Schovat do…“ (`hide.ts`): tabulkové testy.
import { test } from "node:test";
import assert from "node:assert/strict";
import { isPinStyle, PIN_STYLES, pinLook } from "./styles.ts";
import { pinColors } from "./model.ts";
import { hidePicks, validHide } from "./hide.ts";

test("pinLook: classic bez barvy = původní vzhled (pinColors)", () => {
  for (const [bg, text] of [[88, 100], [20, 60], [0, 50], [100, 100]]) {
    const old = pinColors(bg, text);
    const v = pinLook("classic", bg, text, null);
    assert.deepEqual([v["--pin-bg"], v["--pin-bar"], v["--pin-fg"], v["--pin-shadow"], v["--pin-tint"]], [old.background, old.bar, old.color, old.shadow, ""], `${bg}/${text}`);
  }
  // neznámý / chybějící styl = classic
  assert.deepEqual(pinLook("neon", 88, 100, null), pinLook("classic", 88, 100, null));
  assert.deepEqual(pinLook(undefined, 88, 100, null), pinLook("classic", 88, 100, null));
});

test("pinLook: každý styl má všechny proměnné, barva se promítne", () => {
  for (const s of PIN_STYLES) {
    const v = pinLook(s, 60, 90, "#ef6b6b");
    for (const k of ["--pin-bg", "--pin-bar", "--pin-fg", "--pin-shadow", "--pin-tint", "--pin-line"]) assert.ok(k in v, `${s} ${k}`);
    assert.equal(v["--pin-tint"], "#ef6b6b", s);
  }
  // papír: tmavý text, výchozí žlutý; proužek: výchozí modrý; ostatní bez barvy nic
  assert.match(pinLook("paper", 88, 100, null)["--pin-fg"], /^rgba\(28, 26, 22/);
  assert.equal(pinLook("paper", 88, 100, null)["--pin-tint"], "#f5c542");
  assert.equal(pinLook("stripe", 88, 100, null)["--pin-tint"], "#7ab8ff");
  assert.equal(pinLook("dark", 88, 100, null)["--pin-tint"], "");
  // text nikdy pod čitelnost ani v tmavém / papíru
  assert.match(pinLook("dark", 0, 0, null)["--pin-fg"], /0\.5\)$/);
  assert.ok(isPinStyle("glass") && !isPinStyle("x"));
});

test("hidePicks: rychlé volby podle času a dne", () => {
  const ids = (d) => hidePicks(d).map((p) => p.id);
  // čtvrtek 23. 10. 2026 10:07
  const thu = new Date(2026, 9, 23, 10, 7);
  assert.deepEqual(ids(thu), ["hour", "evening", "tomorrow", "monday", "week"]);
  assert.deepEqual(hidePicks(thu).map((p) => p.value), ["2026-10-23T11:10", "2026-10-23T18:00", "2026-10-24T08:00", "2026-10-26T08:00", "2026-10-30T08:00"]);
  // večer už bez „dnes večer“; v neděli je pondělí zítra → jen jednou
  assert.deepEqual(ids(new Date(2026, 9, 23, 19, 0)), ["hour", "tomorrow", "monday", "week"]);
  assert.deepEqual(ids(new Date(2026, 9, 25, 9, 0)), ["hour", "evening", "tomorrow", "week"]);
});

test("validHide: jen platný čas v budoucnu", () => {
  const now = new Date(2026, 9, 23, 10, 0);
  const cases = [["2026-10-24T08:00", "2026-10-24T08:00"], ["2026-10-23T09:00", null], ["2026-10-23T10:00", null], ["zítra", null], ["", null]];
  for (const [v, want] of cases) assert.equal(validHide(v, now), want, v);
});
