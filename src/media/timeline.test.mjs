// Tabulkové testy průběhu skladby a vzhledů karty Právě hraje (`timeline.ts`).
//   node --experimental-strip-types --test src/media/timeline.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { fmtDuration, fraction, live, nextRepeat, nextTickIn, nextVariant, positionAt, seekKey, seekTarget, shownPosition, variantOf, VARIANTS } from "./timeline.ts";

const NOW = 1_760_000_000_000;
const tl = (positionMs, durationMs, updatedMs, rate = 1) => ({ positionMs, durationMs, updatedMs, rate, canSeek: true });

test("pozice: poslední + uplynulý čas × rychlost, pauza stojí, ořez na délku", () => {
  const cases = [
    ["hraje", tl(10_000, 60_000, NOW - 5_000), NOW, true, 15_000],
    ["pauza stojí", tl(10_000, 60_000, NOW - 5_000), NOW, false, 10_000],
    ["ořez na délku", tl(58_000, 60_000, NOW - 9_000), NOW, true, 60_000],
    ["hodiny před hlášením neubírají", tl(10_000, 60_000, NOW + 3_000), NOW, true, 10_000],
    ["rychlost 2×", tl(10_000, 60_000, NOW - 5_000, 2), NOW, true, 20_000],
    ["rychlost 0 = 1×", tl(10_000, 60_000, NOW - 5_000, 0), NOW, true, 15_000],
    ["rychlost NaN = 1×", tl(10_000, 60_000, NOW - 5_000, Number.NaN), NOW, true, 15_000],
    ["záporná pozice = 0", tl(-500, 60_000, NOW), NOW, false, 0],
    ["dlouho po konci", tl(1_000, 60_000, NOW - 3_600_000), NOW, true, 60_000],
  ];
  for (const [name, t, now, playing, want] of cases) assert.equal(positionAt(t, now, playing), want, name);
});

test("po přetočení: cíl, dokud nepřijde nová timeline nebo nevyprší čekání", () => {
  const old = tl(10_000, 200_000, NOW - 4_000);
  const seek = { ms: 90_000, at: NOW - 1_000 };
  const cases = [
    ["bez přetočení = dopočet", old, NOW, true, null, 14_000],
    ["čeká na jádro, hraje dál od cíle", old, NOW, true, seek, 91_000],
    ["čeká na jádro, pauza = cíl", old, NOW, false, seek, 90_000],
    ["nová timeline z jádra má přednost", tl(90_100, 200_000, NOW - 500), NOW, true, seek, 90_600],
    ["vypršelo = dopočet ze staré", old, NOW + 2_000, true, seek, 16_000],
  ];
  for (const [name, t, now, playing, s, want] of cases) assert.equal(shownPosition(t, now, playing, s), want, name);
});

test("čas m:ss a h:mm:ss", () => {
  const cases = [
    [0, "0:00"],
    [999, "0:00"],
    [1_000, "0:01"],
    [102_000, "1:42"],
    [243_999, "4:03"],
    [600_000, "10:00"],
    [3_725_000, "1:02:05"],
    [-5, "0:00"],
    [Number.NaN, "0:00"],
  ];
  for (const [ms, want] of cases) assert.equal(fmtDuration(ms), want, String(ms));
});

test("lišta: podíl, klik na pozici, klávesy", () => {
  assert.equal(fraction(50, 200), 0.25);
  assert.equal(fraction(300, 200), 1);
  assert.equal(fraction(-1, 200), 0);
  assert.equal(fraction(5, 0), 0);
  const clicks = [
    [0, 200, 240_000, 0],
    [100, 200, 240_000, 120_000],
    [250, 200, 240_000, 240_000],
    [-10, 200, 240_000, 0],
    [50, 0, 240_000, 0],
    [50, 200, 0, 0],
  ];
  for (const [x, w, d, want] of clicks) assert.equal(seekTarget(x, w, d), want, `${x}/${w}`);
  const keys = [
    ["ArrowRight", 10_000, 60_000, 15_000],
    ["ArrowLeft", 3_000, 60_000, 0],
    ["ArrowUp", 58_000, 60_000, 60_000],
    ["Home", 30_000, 60_000, 0],
    ["End", 30_000, 60_000, 60_000],
    ["a", 30_000, 60_000, null],
  ];
  for (const [k, pos, d, want] of keys) assert.equal(seekKey(k, pos, d), want, k);
});

test("tik časovače: jen když hraje, na další celou sekundu", () => {
  const cases = [
    [1_500, 1, true, 505],
    [1_000, 1, true, 1_005],
    [1_999, 1, true, 50],
    [1_500, 2, true, 255],
    [1_500, 0, true, 505],
    [1_500, 1, false, null],
  ];
  for (const [pos, rate, playing, want] of cases) assert.equal(nextTickIn(pos, rate, playing), want, `${pos} ${rate} ${playing}`);
  assert.equal(live(true, true), true);
  assert.equal(live(true, false), false, "zavřený šuplík = nic se nehýbe");
  assert.equal(live(false, true), false, "pauza = deska stojí");
});

test("vzhledy: výchozí karta, neznámý = výchozí, přepínání dokola; opakování dokola", () => {
  assert.deepEqual(VARIANTS, ["compact", "card", "cover", "vinyl", "strip"]);
  const cases = [
    [undefined, "card"],
    [null, "card"],
    [{}, "card"],
    [{ variant: "vinyl" }, "vinyl"],
    [{ variant: "strip" }, "strip"],
    [{ variant: "big" }, "card"],
    [{ variant: 3 }, "card"],
  ];
  for (const [look, want] of cases) assert.equal(variantOf(look), want, JSON.stringify(look));
  assert.deepEqual(VARIANTS.map(nextVariant), ["card", "cover", "vinyl", "strip", "compact"]);
  assert.deepEqual(["none", "list", "track"].map(nextRepeat), ["list", "track", "none"]);
});
