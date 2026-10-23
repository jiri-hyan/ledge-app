// Tabulkové testy oblíbených karty Právě hraje (`favorites.ts`).
//   node --experimental-strip-types --test src/media/favorites.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { MAX_FAVORITES, favoriteError, favoriteSource, musicLinkFrom, renamedFavorite, suggestedName, withFavorite, withoutFavorite } from "./favorites.ts";

// Stejné řádky jako `SOURCE_TABLE` v Rust `mediaconf.rs` (zrcadlo `source_of`): [odkaz, zdroj nebo null]
const SOURCE_TABLE = [
  ["spotify:playlist:37i9dQZF1DXcBWIGoYBM5M", "spotify"],
  ["spotify:album:4aawyAB9vmqN3uQ7FjRGTy", "spotify"],
  ["SPOTIFY:track:abc123", "spotify"],
  ["spotify:playlist:", null],
  ["spotify:playlist", null],
  ["spotify:playlist:abc:play", null],
  ["spotify:radio:abc", null],
  ["spotify:playlist:a-b", null],
  ["https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=x", "spotify"],
  ["https://music.youtube.com/playlist?list=PLabc", "ytMusic"],
  ["https://music.youtube.com:443/watch?v=1", "ytMusic"],
  ["https://www.youtube.com/watch?v=1", "web"],
  ["http://radia.cz/stream", "web"],
  ["https://user:pw@music.youtube.com/", null],
  ["https://music.youtube.com@evil.example/", null],
  ["https://", null],
  ["https:///x", null],
  ["ftp://example.com/a", null],
  ["file:///C:/Windows/notepad.exe", null],
  ["javascript:alert(1)", null],
  ["C:\Windows\notepad.exe", null],
  ["https://example.com/a b", null],
  ["", null],
  ["https://exa_mple.com", null],
];

test("zdroj odkazu: tabulka sdílená s Rust mediaconf", () => {
  for (const [uri, want] of SOURCE_TABLE) assert.equal(favoriteSource(uri), want, uri);
  assert.equal(favoriteSource(`https://example.com/${"a".repeat(2048)}`), null);
});

test("kontrola rozepsané položky", () => {
  const ok = { name: "Ráno", uri: "spotify:playlist:abc" };
  const full = Array.from({ length: MAX_FAVORITES }, (_, i) => ({ name: `n${i}`, uri: ok.uri }));
  const cases = [
    ["v pořádku", ok, [], null],
    ["prázdný název", { ...ok, name: "  " }, [], "name"],
    ["dlouhý název", { ...ok, name: "x".repeat(61) }, [], "nameLong"],
    ["cizí odkaz", { ...ok, uri: "file:///C:/x.exe" }, [], "uri"],
    ["odkaz s mezerami okolo projde (ořízne se)", { ...ok, uri: "  spotify:playlist:abc " }, [], null],
    ["plný seznam", ok, full, "max"],
  ];
  for (const [name, f, list, want] of cases) assert.equal(favoriteError(f, list), want, name);
});

test("přidat, přejmenovat, odebrat", () => {
  const a = { name: "A", uri: "spotify:playlist:a" };
  const b = { name: "B", uri: "https://music.youtube.com/watch?v=1" };
  let l = withFavorite([], { name: " A ", uri: " spotify:playlist:a " });
  assert.deepEqual(l, [a]);
  l = withFavorite(l, b);
  assert.deepEqual(l, [a, b]);
  assert.deepEqual(withFavorite(l, { name: "", uri: a.uri }), l, "neplatná = beze změny");
  assert.deepEqual(renamedFavorite(l, 1, " Rádio "), [a, { ...b, name: "Rádio" }]);
  assert.deepEqual(renamedFavorite(l, 1, "  "), l, "prázdný název se nepřijme");
  assert.deepEqual(withoutFavorite(l, 0), [b]);
  assert.deepEqual(withoutFavorite(l, 5), l);
});

test("odkaz na hudbu ze schránky: nejnovější, ne zkrácený", () => {
  const it = (preview, atMs) => ({ preview, atMs });
  const cases = [
    ["bez historie", null, ""],
    ["prázdná", [], ""],
    ["žádný odkaz na hudbu", [it("https://example.com", 5), it("ahoj", 9)], ""],
    ["Spotify", [it("https://open.spotify.com/playlist/abc", 5)], "https://open.spotify.com/playlist/abc"],
    ["nejnovější vyhrává", [it("spotify:playlist:old", 1), it("https://music.youtube.com/watch?v=2", 7), it("spotify:album:mid", 3)], "https://music.youtube.com/watch?v=2"],
    ["zkrácený náhled se nenabízí", [it("https://open.spotify.com/playlist/abcdefg…", 9)], ""],
  ];
  for (const [name, items, want] of cases) assert.equal(musicLinkFrom(items), want, name);
});

test("předvyplněný název: titul, jinak aplikace", () => {
  assert.equal(suggestedName({ title: "Song", app: "Spotify" }), "Song");
  assert.equal(suggestedName({ title: "", app: "Spotify" }), "Spotify");
  assert.equal(suggestedName(null), "");
  assert.equal(suggestedName({ title: "x".repeat(80) }).length, 60);
});
