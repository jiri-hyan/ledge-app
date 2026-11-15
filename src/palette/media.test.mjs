// Hudba a hlasitost v paletě (`media.ts`): kdy se řádky ukážou, co volají.
//   node --experimental-strip-types --test src/palette/media.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { mediaItems, volumeQuery } from "./media.ts";

const mk = (media, volume) => ({ personal: { media, volume } });
const track = (o = {}) => ({ title: "Song", artist: "Band", app: "Spotify", status: "playing", canToggle: true, canPrev: true, canNext: true, ...o });
const vol = (o = {}) => ({ level: 50, muted: false, device: "Reproduktory", ...o });
const calls = [];
const io = { control: (a) => calls.push(["control", a]), volume: (l) => calls.push(["volume", l]), mute: (m) => calls.push(["mute", m]) };
const ids = (s, q = "") => mediaItems(s, q, io).map((i) => i.id);
const run = (s, id, q = "") => (calls.length = 0, mediaItems(s, q, io).find((i) => i.id === id).run(), calls[0]);

test("bez dat nic (žádné divadlo)", () => {
  assert.deepEqual(ids({ personal: undefined }), []);
  assert.deepEqual(ids(mk(null, null)), []);
  assert.deepEqual(ids(mk(track({ status: "stopped" }), null)), []);
});

test("přehrávač: pauza při hraní, přehrát při pauze, další / předchozí podle schopností", () => {
  assert.deepEqual(ids(mk(track(), null)), ["media:toggle", "media:next", "media:prev"]);
  assert.equal(mediaItems(mk(track(), null), "", io)[0].title, "Pauza");
  assert.equal(mediaItems(mk(track({ status: "paused" }), null), "", io)[0].title, "Přehrát");
  assert.deepEqual(ids(mk(track({ canPrev: false, canNext: false }), null)), ["media:toggle"]);
  assert.deepEqual(run(mk(track(), null), "media:next"), ["control", "next"]);
  assert.deepEqual(run(mk(track(), null), "media:toggle"), ["control", "toggle"]);
});

test("hlasitost ±10 se ořízne na 0–100 a mezní řádek se neukáže", () => {
  assert.deepEqual(ids(mk(null, vol())), ["media:up", "media:down", "media:mute"]);
  assert.deepEqual(ids(mk(null, vol({ level: 100 }))), ["media:down", "media:mute"]);
  assert.deepEqual(ids(mk(null, vol({ level: 0 }))), ["media:up", "media:mute"]);
  assert.deepEqual(run(mk(null, vol({ level: 95 })), "media:up"), ["volume", 100]);
  assert.deepEqual(run(mk(null, vol({ level: 5 })), "media:down"), ["volume", 0]);
});

test("ztlumit / zapnout zvuk podle stavu", () => {
  assert.deepEqual(run(mk(null, vol()), "media:mute"), ["mute", true]);
  assert.deepEqual(run(mk(null, vol({ muted: true })), "media:mute"), ["mute", false]);
  assert.equal(mediaItems(mk(null, vol({ muted: true })), "", io).at(-1).title, "Zapnout zvuk");
});

test("číslo v dotazu: „hlasitost 40“ přidá řádek, jen s hlasitostí z jádra", () => {
  for (const [q, n] of [["hlasitost 40", 40], ["Volume 7 %", 7], ["vol 250", 100], ["zvuk 0", 0]]) {
    assert.equal(volumeQuery(q), n, q);
    assert.deepEqual(run(mk(null, vol()), `media:set${n}`, q), ["volume", n]);
  }
  for (const q of ["hlasitost", "hlasitost abc", "40", "hlasitost 4 0", ""]) assert.equal(volumeQuery(q), null, q);
  assert.ok(!ids(mk(null, vol()), "hlasitost").some((i) => i.startsWith("media:set")));
  assert.deepEqual(ids(mk(null, null), "hlasitost 40"), [], "bez dat nic");
});
