// Tabulkové testy prohlídky při prvním otevření (`tips.ts`): kdy se ukáže, zápis do `ui.tips`, pozice bubliny.
//   node --experimental-strip-types --test src/layout/tips.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { bubblePos, seen, shouldShow, STEPS, TOUR_ID, withTip } from "./tips.ts";

test("kdy se prohlídka ukáže", () => {
  // (popis, config, průvodce otevřený, chyba configu, ukázat)
  const cases = [
    ["nový uživatel / starý config bez ui.tips (po aktualizaci)", { ui: { lang: "cs" } }, false, null, true],
    ["bez sekce ui", {}, false, null, true],
    ["už viděl", { ui: { tips: [TOUR_ID] } }, false, null, false],
    ["viděl jinou (starší) prohlídku", { ui: { tips: ["tour0"] } }, false, null, true],
    ["s průvodcem ne", {}, true, null, false],
    ["chyba configu ne (nešlo by uložit)", {}, false, "config.json: …", false],
    ["config nejde přečíst", null, false, null, false],
    ["nesmysl v tips", { ui: { tips: "tour1" } }, false, null, true],
  ];
  for (const [name, cfg, wiz, err, want] of cases) assert.equal(shouldShow(cfg, wiz, err), want, name);
});

test("withTip: jen ui.tips, bez duplicit, vstup netknutý", () => {
  const cfg = { ui: { lang: "en", tips: ["tour0", 3] }, projects: [{ id: "a" }] };
  const before = JSON.stringify(cfg);
  const out = withTip(cfg);
  assert.deepEqual(out.ui, { lang: "en", tips: ["tour0", TOUR_ID] });
  assert.equal(out.projects, cfg.projects);
  assert.equal(JSON.stringify(cfg), before);
  assert.deepEqual(withTip(out).ui.tips, ["tour0", TOUR_ID], "podruhé beze změny");
  assert.deepEqual(withTip({}).ui, { tips: [TOUR_ID] });
  assert.ok(seen(out) && !seen(cfg));
});

test("tři bubliny: ✎, ⋯, Ctrl+K", () => {
  assert.deepEqual(STEPS.map((s) => s.target), ["edit", "menu", "palette"]);
  for (const s of STEPS) assert.ok(s.title[0].startsWith("cards.tour.") && s.body[1], s.target);
});

test("pozice bubliny: pod cílem, jinak nad; vždy celá v okně", () => {
  const vw = 1000;
  const vh = 600;
  const cases = [
    ["✎ nahoře vpravo", { left: 900, top: 10, width: 30, height: 30 }, { below: true, top: 50, left: 1000 - 8 - 260 }],
    ["uprostřed", { left: 400, top: 100, width: 40, height: 20 }, { below: true, top: 130, left: 290 }],
    ["u spodního okraje = nad", { left: 400, top: 560, width: 40, height: 20 }, { below: false, top: 560 - 10 - 120, left: 290 }],
    ["vlevo u kraje", { left: 0, top: 10, width: 20, height: 20 }, { below: true, top: 40, left: 8 }],
  ];
  for (const [name, r, want] of cases) {
    const got = bubblePos(r, 260, 120, vw, vh);
    assert.deepEqual({ below: got.below, top: got.top, left: got.left }, want, name);
    assert.ok(got.arrow >= 12 && got.arrow <= 248, `${name}: šipka`);
    assert.ok(got.left >= 8 && got.left + 260 <= vw - 8, `${name}: v okně`);
  }
});
