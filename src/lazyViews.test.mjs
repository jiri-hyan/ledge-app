// Hlídač přednačítání líných pohledů (`lazyViews.tsx`): v klidu po prvním otevření smí PRELOAD tahat jen často
// používané pohledy; průvodce, katalog, vzhled karty a panel „Nová verze“ se načítají až při potřebě (paměť WebView2).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const src = readFileSync(new URL("./lazyViews.tsx", import.meta.url), "utf8");
const block = (name) => {
  const a = src.indexOf(`const ${name}`);
  return a < 0 ? "" : src.slice(a, src.indexOf("];", a));
};

test("PRELOAD bez zřídka používaných pohledů", () => {
  const pre = block("PRELOAD");
  assert.ok(pre.includes("palette/Palette") && pre.includes("notes/NoteEditor"), "paleta a poznámky se přednačítají");
  for (const rare of ["onboarding/Wizard", "layout/Catalog", "layout/CardLook", "cards/UpdatePanel"]) assert.ok(!pre.includes(rare), `${rare} nepatří do PRELOAD`);
});

test("katalog a vzhled karty se přednačítají při úpravách", () => {
  const ed = block("EDIT_VIEWS");
  assert.ok(ed.includes("layout/Catalog") && ed.includes("layout/CardLook"));
});

test("CARD_CHUNKS: lazy jsou těžké / zřídka používané karty, jádro zůstává statické", () => {
  // lazyViews.tsx nejde načíst v Node (TSX), proto zrcadlo seznamu z textu: každá lazy karta má řádek v CARD_CHUNKS
  const chunks = block("CARD_CHUNKS");
  for (const id of ["weather", "calendar", "mail", "github", "system", "usage", "toggles", "winquick", "week", "bigclock"]) assert.ok(chunks.includes(`  ${id}:`), id);
  for (const core of ["projects", "claude", "notes", "media", "clock"]) assert.ok(!chunks.includes(`  ${core}:`),`${core} zůstává statická`);
});
