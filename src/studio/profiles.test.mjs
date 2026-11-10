// Testy osobních profilů (Pillar 1):
//   node --test src/studio/profiles.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { DESIGN_IDS } from "../strip/compose.ts";
import { HOVER_MODES } from "../hover/modes.ts";
import { PROFILES, PROFILE_IDS, activeProfile, applyProfileToConfig, hideCardEverywhere } from "./profiles.ts";

test("Všechny 4 profily mají platný design, hover a buildLayout s neprázdnými stránkami", () => {
  assert.equal(PROFILES.length, 4);
  assert.deepEqual(PROFILE_IDS, ["ai_builder", "power_hw", "focus_zen", "organizer"]);

  const validHover = HOVER_MODES.map((m) => m.id);

  for (const prof of PROFILES) {
    assert.ok(PROFILE_IDS.includes(prof.id), `známé id ${prof.id}`);
    assert.ok(DESIGN_IDS.includes(prof.design), `známý design ${prof.design}`);
    assert.ok(validHover.includes(prof.hover), `známý hover režim ${prof.hover}`);
    assert.ok(typeof prof.icon === "string" && prof.icon.length > 0, "ikona existuje");

    const layout = prof.buildLayout();
    assert.ok(Array.isArray(layout.pages), "pages je pole");
    assert.ok(layout.pages.length >= 2, "alespoň 2 stránky");
    for (const page of layout.pages) {
      assert.ok(page.cards.length > 0, `stránka ${page.id} má karty`);
      for (const card of page.cards) {
        assert.ok(card.id && card.size, "karta má id a velikost");
      }
    }
  }
});

test("applyProfileToConfig({}, id) nastaví profil tak, že activeProfile(next) vrátí dané id pro všechny 4 profily", () => {
  for (const id of PROFILE_IDS) {
    const next = applyProfileToConfig({}, id);
    assert.equal(activeProfile(next), id, `profil ${id} je aktivní`);
  }

  // Změna hover nebo pruhu zruší aktivní profil
  const p0 = applyProfileToConfig({}, "ai_builder");
  assert.equal(activeProfile({ ...p0, drawer: { hover: "compact" } }), null);
  assert.equal(activeProfile({ ...p0, strip: { look: "classic" } }), null);
});

test("hideCardEverywhere(layout, 'weather') odebere kartu počasí ze všech stránek a ostatní zachová", () => {
  const zenProf = PROFILES.find((p) => p.id === "focus_zen");
  assert.ok(zenProf);
  const layout = zenProf.buildLayout();

  // zen má počasí na záložce PERSONAL
  const personalBefore = layout.pages.find((p) => p.id === "osobni");
  assert.ok(personalBefore.cards.some((c) => c.id === "weather"));
  const beforeCount = personalBefore.cards.length;

  const nextLayout = hideCardEverywhere(layout, "weather");
  for (const page of nextLayout.pages) {
    assert.ok(!page.cards.some((c) => c.id === "weather"), `stránka ${page.id} už nemá weather`);
  }

  const personalAfter = nextLayout.pages.find((p) => p.id === "osobni");
  assert.equal(personalAfter.cards.length, beforeCount - 1);
  assert.ok(personalAfter.cards.some((c) => c.id === "bigclock"));
  assert.ok(personalAfter.cards.some((c) => c.id === "media"));
});
