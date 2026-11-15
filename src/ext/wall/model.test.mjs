// Scéna tapety wall (E3): čistá logika, bez DOM. `npm run test:ui`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { formatDay, formatDate, formatTime, msToNextMinute, barPlan, contrast, pickInk, avgRgb, BG_STOPS, bgCss, playerView, normLang, photoPlan, textRatio, dimmed, rgbSamples, repColor, coverPlan, isImageUrl, PHOTO_BASE_DIM, PHOTO_MAX_DIM } from "./model.ts";
import { buildWall } from "../../../scripts/wall-build.mjs";

const SUN = new Date(2024, 11, 15, 17, 32, 5); // neděle 15. 12. 2024 (místní čas)

test("formát dne, data a času cs + en", () => {
  const rows = [
    ["cs", "NEDĚLE", "15. PROSINCE 2024"],
    ["en", "SUNDAY", "15 DECEMBER, 2024"],
  ];
  for (const [lang, day, date] of rows) {
    assert.equal(formatDay(SUN, lang), day);
    assert.equal(formatDate(SUN, lang), date);
  }
  assert.equal(formatTime(SUN), "17:32");
  assert.equal(formatTime(new Date(2025, 0, 3, 4, 5)), "04:05");
  assert.equal(formatDay(new Date(2025, 0, 6), "cs"), "PONDĚLÍ");
  assert.equal(formatDate(new Date(2025, 8, 1), "cs"), "1. ZÁŘÍ 2025");
  assert.equal(normLang("en"), "en");
  assert.equal(normLang("de"), "cs");
  assert.equal(normLang(undefined), "cs");
});

test("všech 12 měsíců a 7 dnů má text", () => {
  for (const lang of ["cs", "en"]) {
    for (let m = 0; m < 12; m++) assert.match(formatDate(new Date(2025, m, 10), lang), /^10\.? \p{Lu}+,? 2025$/u);
    for (let d = 0; d < 7; d++) assert.ok(formatDay(new Date(2025, 0, 5 + d), lang).length > 3);
  }
});

test("msToNextMinute míří na další celou minutu", () => {
  assert.equal(msToNextMinute(0), 60050);
  assert.equal(msToNextMinute(59999), 51);
  assert.equal(msToNextMinute(60000 + 30000), 30050);
  assert.equal(msToNextMinute(1000, 0), 59000);
  assert.equal(msToNextMinute(-1000, 0), 1000); // záporný čas nesmí dát > 60 s
});

const tl = (o = {}) => ({ positionMs: 60000, durationMs: 240000, updatedMs: 1_000_000, rate: 1, ...o });

test("barPlan: tabulka", () => {
  const rows = [
    ["hraje, vzorek právě teď", tl(), "playing", 1_000_000, { startPct: 25, animMs: 180000 }],
    ["hraje, vzorek před 60 s", tl(), "playing", 1_060_000, { startPct: 50, animMs: 120000 }],
    ["pauza se nepohybuje", tl(), "paused", 1_090_000, { startPct: 25, animMs: 0 }],
    ["rychlost 2x", tl({ rate: 2 }), "playing", 1_030_000, { startPct: 50, animMs: 60000 }],
    ["za koncem se ořízne", tl(), "playing", 9_000_000, { startPct: 100, animMs: 0 }],
    ["hodiny jdou zpět: neodečítá", tl(), "playing", 900_000, { startPct: 25, animMs: 180000 }],
    ["rate 0 bere jako 1", tl({ rate: 0 }), "playing", 1_000_000, { startPct: 25, animMs: 180000 }],
  ];
  for (const [name, t, status, now, want] of rows) assert.deepEqual(barPlan(t, status, now), want, name);
  assert.equal(barPlan(null, "playing", 0), null);
  assert.equal(barPlan(tl({ durationMs: 0 }), "playing", 0), null);
});

test("kontrast a barva textu", () => {
  assert.equal(Math.round(contrast([255, 255, 255], [0, 0, 0])), 21);
  assert.equal(contrast([10, 10, 10], [10, 10, 10]), 1);
  const dark = pickInk([10, 18, 18]);
  assert.deepEqual(dark.rgb, [255, 255, 255]);
  assert.equal(dark.scrim, 0);
  const light = pickInk([240, 240, 240]);
  assert.deepEqual(light.rgb, [16, 20, 28]);
  // střední šeď: žádná barva nedá 4,5, takže scrim > 0
  const mid = pickInk([120, 120, 120]);
  assert.ok(mid.scrim > 0 && mid.scrim <= 1);
});

test("pozadí scény: průměr zarážek má s bílým textem kontrast >= 4,5", () => {
  const ink = pickInk(avgRgb(BG_STOPS));
  assert.deepEqual(ink.rgb, [255, 255, 255]);
  assert.ok(ink.ratio >= 4.5);
  assert.match(bgCss(BG_STOPS), /^radial-gradient\(.*rgb\(34,58,48\)/);
  assert.deepEqual(avgRgb([[0, 0, 0], [10, 20, 30]]), [5, 10, 15]);
});

test("playerView: bez hudby nic", () => {
  assert.equal(playerView(null), null);
  assert.equal(playerView({}), null);
  assert.equal(playerView({ title: "A", status: "stopped" }), null);
  assert.equal(playerView({ title: "", status: "playing" }), null);
  const v = playerView({ title: "T", artist: "I", status: "paused", canPrev: true });
  assert.deepEqual(v, { title: "T", artist: "I", playing: false, canToggle: true, canPrev: true, canNext: false });
  assert.equal(playerView({ title: "T", status: "playing", canToggle: false }).canToggle, false);
});

test("wall.html je aktuální (node scripts/wall-build.mjs) a vejde se do limitu NavigateToString", () => {
  const html = buildWall();
  const cur = readFileSync(new URL("../../../src-tauri/src/ext/wall/wall.html", import.meta.url), "utf8").split("\r\n").join("\n");
  assert.equal(cur, html, "wall.html je zastaralý: node scripts/wall-build.mjs");
  assert.ok(Buffer.byteLength(html) < 2 * 1024 * 1024);
  for (const k of ["chrome.webview", "'ready'", "visibilitychange", "'init'", "'data'"]) assert.ok(html.includes(k), k);
  assert.ok(!/https?:\/\//.test(html.replace(/xmlns="http:\/\/www\.w3\.org\/2000\/svg"/g, "")), "žádné externí zdroje");
});

test("photoPlan: text nad fotkou má kontrast >= 4,5 (i nejslabší, krytí .7), ztmavení je minimální", () => {
  const grays = [0, 30, 60, 90, 120, 160, 200, 230, 255];
  for (const g of grays) {
    const p = photoPlan([[g, g, g]]);
    assert.ok(p.dim >= PHOTO_BASE_DIM && p.dim <= PHOTO_MAX_DIM, `dim ${p.dim} pro šedou ${g}`);
    assert.ok(p.ratio >= 4.5, `šedá ${g}: kontrast ${p.ratio.toFixed(2)}`);
    // ověření nezávislým výpočtem: po ztmavení ať text opravdu drží
    assert.ok(textRatio(dimmed([g, g, g], p.dim)) >= 4.5 - 1e-9, `šedá ${g}`);
    // minimální: o kousek méně ztmavení už nestačí (kromě případů zastaveného základem)
    if (p.dim > PHOTO_BASE_DIM + 1e-6) assert.ok(textRatio(dimmed([g, g, g], p.dim - 0.01)) < 4.5, `šedá ${g} nešlo méně`);
  }
  // tmavá fotka: jen základní ztmavení; bílá: silně
  assert.equal(photoPlan([[10, 20, 15]]).dim, PHOTO_BASE_DIM);
  assert.ok(photoPlan([[255, 255, 255]]).dim > 0.6);
  // barevné fotky (nebe, tráva, podzim)
  for (const c of [[135, 206, 235], [34, 139, 34], [255, 140, 0], [250, 250, 210]]) {
    assert.ok(photoPlan([c]).ratio >= 4.5, `barva ${c}`);
  }
  // prázdné vzorky = černá, jen základ
  assert.equal(photoPlan([]).dim, PHOTO_BASE_DIM);
});

test("repColor bere 85. percentil jasu, ne průměr; rgbSamples přeskočí průhledné", () => {
  const dark = Array.from({ length: 80 }, () => [10, 10, 10]);
  const bright = Array.from({ length: 20 }, () => [250, 250, 250]);
  assert.deepEqual(repColor([...dark, ...bright]), [250, 250, 250]);
  assert.deepEqual(repColor([...dark]), [10, 10, 10]);
  assert.deepEqual(repColor([]), [0, 0, 0]);
  assert.deepEqual(rgbSamples([1, 2, 3, 255, 9, 9, 9, 0, 4, 5, 6, 128]), [[1, 2, 3], [4, 5, 6]]);
  assert.deepEqual(rgbSamples([1, 2, 3]), []);
});

test("coverPlan: nový klíč = požadavek, stejný = nic, žádný = smazat", () => {
  const cases = [
    [null, "k1", "request"],
    ["k1", "k1", "keep"],
    ["k1", "k2", "request"],
    ["k1", null, "clear"],
    ["k1", undefined, "clear"],
    [null, null, "clear"],
    [null, "", "clear"],
  ];
  for (const [prev, next, want] of cases) assert.equal(coverPlan(prev, next), want, `${prev} -> ${next}`);
});

test("isImageUrl pustí jen data: URL správného typu a délky", () => {
  const png = "data:image/png;base64,AAAA";
  assert.ok(isImageUrl(png, "png", 100));
  assert.ok(!isImageUrl(png, "jpeg", 100), "jiný typ");
  assert.ok(!isImageUrl(png, "png", 10), "příliš dlouhé");
  for (const bad of ["https://evil.example/a.png", "file:///C:/a.png", "javascript:alert(1)", "data:text/html;base64,AAAA", "data:image/svg+xml;base64,AAAA", "", null, 5, undefined]) {
    assert.ok(!isImageUrl(bad, "png", 1000), String(bad));
  }
});
