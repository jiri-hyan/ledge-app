// Převody jednotek palety (`units.ts`): tvar dotazu, spojky cs/en, rozměry, teplota s posunem,
// jedinečná jména jednotek, text podle jazyka.
//   node --experimental-strip-types --test src/palette/units.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { UNITS, convert, conversionText, unitIndex, unitKey } from "./units.ts";
import "../i18n/en-quick.ts";

const close = (a, b) => Math.abs(a - b) < 1e-9 * Math.max(1, Math.abs(b));

test("převody (tabulka)", () => {
  const cases = [
    ["10 km na mi", 6.2137119223733395, "mi"],
    ["5 ft in cm", 152.4, "cm"],
    ["5 in in cm", 12.7, "cm"],
    ["5 in to cm", 12.7, "cm"],
    ["100 °F na °C", 37.77777777777778, "°C"],
    ["100°F na °C", 37.77777777777778, "°C"],
    ["100 F to C", 37.77777777777778, "°C"],
    ["0 °C na °F", 32, "°F"],
    ["-40 °C na °F", -40, "°F"],
    ["0 K na °C", -273.15, "°C"],
    ["3 GB v MB", 3000, "MB"],
    ["1 GiB na MB", 1073.741824, "MB"],
    ["1024 KiB do MiB", 1, "MiB"],
    ["100 Mbit na MB", 12.5, "MB"],
    ["2 h na min", 120, "min"],
    ["2 hodiny na minuty", 120, "min"],
    ["1 týden na dny", 7, "d"],
    ["1 rok na d", 365.2425, "d"],
    ["60 km/h na m/s", 16.666666666666668, "m/s"],
    ["100 mph to km/h", 160.9344, "km/h"],
    ["1 l na ml", 1000, "ml"],
    ["1 gal na l", 3.785411784, "l"],
    ["8 fl oz na ml", 236.5882365, "ml"],
    ["1 ha to m2", 10000, "m²"],
    ["1 km² na ha", 100, "ha"],
    ["1 lb na kg", 0.45359237, "kg"],
    ["1,5 kg na g", 1500, "g"],
    ["10km na mi", 6.2137119223733395, "mi"],
    ["  10   KM   NA   MI ", 6.2137119223733395, "mi"],
    ["1 m -> cm", 100, "cm"],
    ["10 km na km", 10, "km"],
  ];
  for (const [q, want, sym] of cases) {
    const c = convert(q);
    assert.ok(c, `${q}: žádný převod`);
    assert.ok(close(c.result, want), `${q} → ${c.result}, čekáno ${want}`);
    assert.equal(c.to.sym, sym, q);
  }
});

test("ne-převody → null", () => {
  for (const q of ["", "10 km", "km na mi", "10 km na kg", "10 xyz na km", "10 km mi", "10 km a mi", "10 km na mi navic", "12*3", "100 € na Kč", "10 usd to eur", "abc"]) {
    assert.equal(convert(q), null, JSON.stringify(q));
  }
});

test("jména jednotek jednoznačná (bez diakritiky a velikosti písmen), kladné faktory", () => {
  const seen = new Map();
  for (const u of UNITS) {
    assert.ok(u.f > 0, u.sym);
    for (const n of [u.sym.replace(/\s+/g, ""), ...u.names.split(" ")]) {
      const k = unitKey(n);
      const other = seen.get(k);
      assert.ok(!other || other === u, `„${n}“ je u ${other?.sym} i ${u.sym}`);
      seen.set(k, u);
    }
  }
  assert.equal(unitIndex().size, seen.size);
});

test("text: cs čárka + česká značka, en tečka; do schránky jen číslo", async () => {
  const { setLang } = await import("../i18n.ts");
  assert.deepEqual(conversionText(convert("10 km na mi"), "cs"), { title: "10 km = 6,21371192 mi", copy: "6,21371192" });
  assert.deepEqual(conversionText(convert("10 km to mi"), "en"), { title: "10 km = 6.21371192 mi", copy: "6.21371192" });
  assert.equal(conversionText(convert("14 d na týdny"), "cs").title, "14 d = 2 týd.");
  assert.equal(conversionText(convert("14 d to weeks"), "en").title, "14 d = 2 wk");
  assert.match(conversionText(convert("1 GiB na MB"), "cs").title, /^1 GiB = 1\s073,74182 MB$/u);
  setLang("cs");
});
