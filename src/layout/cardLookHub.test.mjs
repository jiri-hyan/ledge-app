// Jeden panel vzhledu karty (kolo 13): „Vzhled karty…“ (`CardLook.tsx`) je rozcestník — u Právě hraje používá
// miniatury a uložení varianty z `media/VariantPicker.tsx` (stejné pole `media.variant` jako Nastavení → Osobní
// a ikonka v rohu karty), u Projektů a Akcí otevírá panel vzhledu dlaždice. Staré vstupy jsou zkratky ke
// stejnému zdroji. Statická kontrola zapojení (testy chování jsou u čisté logiky `look.ts`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const src = (f) => readFileSync(new URL(`../${f}`, import.meta.url), "utf8");

test("panel Vzhled karty: varianta přehrávače a dlaždice vedou do jednoho zdroje", () => {
  const panel = src("layout/CardLook.tsx");
  assert.match(panel, /from "\.\.\/media\/VariantPicker"/, "miniatury a uložení varianty se berou z VariantPicker");
  assert.match(panel, /saveVariant\(v, p\.cfgError\)/, "klik na variantu ukládá media.variant");
  assert.match(panel, /openLook\(\{ kind: it\.kind/, "dlaždice otevře panel vzhledu zástupce");
  assert.match(panel, /p\.tg\.card === "media"/);
  assert.match(panel, /p\.tg\.card === "projects" \|\| p\.tg\.card === "actions"/);
});

test("zkratky: ikonka v rohu přehrávače i Nastavení vedou k tomu samému poli a panelu", () => {
  const picker = src("media/VariantPicker.tsx");
  assert.match(picker, /openCardLook\(page, "media"/, "nabídka v rohu karty otevře panel Vzhled karty");
  assert.match(picker, /export async function saveVariant/, "jediné uložení varianty");
  const settings = src("media/MediaSettings.tsx");
  assert.match(settings, /c\.set\("media\.variant", v\)/, "Nastavení mění totéž pole");
  assert.match(settings, /med-cardnote/, "a odkazuje na panel Vzhled karty");
  assert.match(src("MainPage.tsx"), /<CardLookPanel page=\{page\(\)\.id\} s=\{p\.s\} \/>/, "panel dostane snapshot (varianta, dlaždice)");
});
