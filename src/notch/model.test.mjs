// Lišta širokého notche (`model.ts`) bez DOM.
//   node --experimental-strip-types --test src/notch/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { notchSections, takenCards, volumeOf, weekStrip } from "./model.ts";

test("sekce jen z karet záložky a jen s daty; pořadí přehrávač | dny | systém | hlasitost", () => {
  const all = { media: true, volume: true };
  const cases = [
    ["všechno", ["system", "clock", "media", "projects"], all, ["player", "days", "system", "volume"]],
    ["nic nehraje = bez přehrávače, hlasitost zůstane", ["media", "clock"], { media: false, volume: true }, ["days", "volume"]],
    ["bez zvukového zařízení", ["media"], { media: true, volume: false }, ["player"]],
    ["karta Právě hraje bez dat = žádná sekce", ["media"], { media: false, volume: false }, []],
    ["kalendář, Dnes, Týden a přepínače zůstávají kartami", ["calendar", "today", "week", "toggles", "notes"], all, []],
    ["velké hodiny = dny a čas", ["bigclock"], all, ["days"]],
    ["prázdná záložka", [], all, []],
  ];
  for (const [name, cards, data, want] of cases) assert.deepEqual(notchSections(cards, data), want, name);
});

test("převzaté karty se pod lištou nekreslí; karta bez viditelné sekce zůstane", () => {
  assert.deepEqual(takenCards(["projects", "media", "clock", "system"], ["player", "days", "system"]), ["media", "clock", "system"]);
  assert.deepEqual(takenCards(["media", "clock"], ["days"]), ["clock"], "media bez dat zůstane kartou (sama se skryje)");
  assert.deepEqual(takenCards(["media"], ["volume"]), ["media"], "stačí jedna sekce karty");
  assert.deepEqual(takenCards(["notes"], ["days"]), []);
  assert.deepEqual(takenCards(["clock", "bigclock", "week"], ["days"]), ["clock", "bigclock"], "obě hodiny v jedné sekci dnů");
});

test("pás dnů: pondělí až neděle, dnešek a víkend", () => {
  const cases = [
    ["sobota 24. 10. 2026", new Date(2026, 9, 24, 16, 34), [19, 20, 21, 22, 23, 24, 25], 5],
    ["pondělí = první", new Date(2026, 9, 19, 8, 0), [19, 20, 21, 22, 23, 24, 25], 0],
    ["neděle = poslední", new Date(2026, 9, 25, 23, 59), [19, 20, 21, 22, 23, 24, 25], 6],
    ["přes konec měsíce", new Date(2026, 9, 30, 12, 0), [26, 27, 28, 29, 30, 31, 1], 4],
    ["přes konec roku", new Date(2026, 11, 31, 12, 0), [28, 29, 30, 31, 1, 2, 3], 3],
  ];
  for (const [name, now, dates, todayAt] of cases) {
    const w = weekStrip(now);
    assert.deepEqual(w.map((d) => d.date), dates, name);
    assert.deepEqual(w.map((d) => d.today), dates.map((_, i) => i === todayAt), name);
    assert.deepEqual(w.map((d) => d.weekend), [false, false, false, false, false, true, true], name);
    assert.deepEqual(w.map((d) => d.dow), [1, 2, 3, 4, 5, 6, 0], name);
  }
});

test("hlasitost z posuvníku", () => {
  for (const [v, want] of [[42, 42], ["42", 42], ["7.6", 8], [-5, 0], [140, 100], ["", null], ["abc", null], [null, null], [undefined, null]]) {
    assert.equal(volumeOf(v), want, String(v));
  }
});

test("šířka notche = Rust NOTCH_PX; rozložení notch zná Rust i výběr stylů", () => {
  const looks = readFileSync(new URL("../theme/looks.ts", import.meta.url), "utf8");
  const rust = readFileSync(new URL("../../src-tauri/src/appearance/look.rs", import.meta.url), "utf8");
  const px = Number(/export const NOTCH_PX = (\d+);/.exec(looks)?.[1]);
  assert.ok(px > 0 && rust.includes(`pub const NOTCH_PX: u32 = ${px};`), `NOTCH_PX ${px}`);
  // grafy: notch kreslí sdílený Sparkline z `src/charts`, ne vlastní křivku
  const bar = readFileSync(new URL("./NotchBar.tsx", import.meta.url), "utf8");
  assert.ok(bar.includes('from "../charts/Sparkline"'), "NotchBar používá src/charts/Sparkline");
  assert.ok(!bar.includes('"./Spark"') && !existsSync(new URL("./Spark.tsx", import.meta.url)), "žádná vlastní křivka v notchi");
});
