// Tabulkové testy pruhu kontextových návrhů (`model.ts`): výběr z reálných událostí, zavření na 24 h, schránka.
//   node --experimental-strip-types --test src/suggest/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CLIP_FRESH_MS, DEV_FRESH_MS, DISMISS_MS, GH_FRESH_MS, MAX_CHIPS, clipUrl, devUrl, dismiss, hostOf, isDismissed, isKnown, knownLinks, normUrl, pick, sameSuggestions } from "./model.ts";

const NOW = 1_800_000_000_000;
const ON = { clipboard: true, devServer: true, github: true, reminders: true, githubPulse: true };
const pr = (n, repo = "a/b") => ({ repo, number: n, title: `PR ${n}`, url: `https://github.com/${repo}/pull/${n}` });
const base = () => ({
  on: { ...ON },
  reminders: [],
  github: { review: [], failing: [], fetchedMs: NOW - 1000 },
  procs: [],
  projects: [{ id: "web", name: "Web" }],
  clip: null,
  known: [],
});
const kinds = (list) => list.map((s) => s.kind);

test("bez zdroje žádný chip", () => {
  assert.deepEqual(pick(base(), new Map(), NOW), []);
});

test("jednotlivé zdroje: co se ukáže a s jakými daty", () => {
  const withIn = (f) => f(base());
  const cases = [
    ["připomínka", withIn((i) => ({ ...i, reminders: [{ path: "C:\\n\\a.md", title: "Zavolat", remindMs: NOW - 5 }] })), { kind: "reminder", title: "Zavolat", path: "C:\\n\\a.md", more: 0 }],
    ["spadlé CI", withIn((i) => ({ ...i, github: { ...i.github, failing: [pr(7)] } })), { kind: "ghCi", title: "a/b#7", num: 7, detail: "PR 7", url: "https://github.com/a/b/pull/7", more: 0 }],
    ["PR k revizi", withIn((i) => ({ ...i, github: { ...i.github, review: [pr(3), pr(4)] } })), { kind: "ghReview", title: "a/b#3", more: 1 }],
    ["dev server", withIn((i) => ({ ...i, procs: [{ id: "web", phase: "ready", port: 5173, url: "http://localhost:5173/", readyAt: NOW - 1000 }] })), { kind: "dev", title: "localhost:5173", detail: "Web", url: "http://localhost:5173/", more: 0 }],
    ["adresa ve schránce", withIn((i) => ({ ...i, clip: { url: "https://www.example.com/a?b=1", atMs: NOW - 10 } })), { kind: "clip", title: "example.com", url: "https://www.example.com/a?b=1" }],
  ];
  for (const [name, input, want] of cases) {
    const got = pick(input, new Map(), NOW);
    assert.equal(got.length, 1, name);
    for (const [k, v] of Object.entries(want)) assert.equal(got[0][k], v, `${name}: ${k}`);
  }
});

test("vypnutý zdroj nic neukáže", () => {
  const full = {
    ...base(),
    reminders: [{ path: "p", title: "t", remindMs: 1 }],
    github: { review: [pr(1)], failing: [pr(2)], fetchedMs: NOW },
    procs: [{ id: "web", phase: "ready", port: 3000, url: null, readyAt: NOW }],
    clip: { url: "https://example.com", atMs: NOW },
  };
  assert.equal(pick(full, new Map(), NOW).length, 3, "strop 3");
  const off = (k) => pick({ ...full, on: { ...ON, [k]: false } }, new Map(), NOW);
  assert.ok(!kinds(off("reminders")).includes("reminder"));
  assert.ok(!kinds(off("github")).some((k) => k.startsWith("gh")));
  assert.ok(!kinds(pick({ ...full, on: { ...ON, devServer: false } }, new Map(), NOW)).includes("dev"));
  // schránka je za třemi důležitějšími; vypnuté ostatní ji pustí dál
  const some = pick({ ...full, on: { ...ON, reminders: false, github: false, devServer: false } }, new Map(), NOW);
  assert.deepEqual(kinds(some), ["clip"]);
  assert.deepEqual(kinds(pick({ ...full, on: { ...ON, clipboard: false } }, new Map(), NOW)), ["reminder", "ghCi", "ghReview"]);
});

test("pořadí důležitosti a strop", () => {
  const input = {
    ...base(),
    reminders: [{ path: "p", title: "t", remindMs: 1 }],
    github: { review: [pr(1)], failing: [pr(2)], fetchedMs: NOW },
    procs: [{ id: "web", phase: "running", port: 3000, url: null, readyAt: NOW }],
    clip: { url: "https://example.com", atMs: NOW },
  };
  const got = pick(input, new Map(), NOW);
  assert.equal(got.length, MAX_CHIPS);
  assert.deepEqual(kinds(got), ["reminder", "ghCi", "ghReview"]);
});

test("GitHub: stará data se nenabízí, bez načtení nic", () => {
  const gh = (fetchedMs) => ({ ...base(), github: { review: [pr(1)], failing: [], fetchedMs } });
  const cases = [
    ["čerstvé", NOW - 1000, 1],
    ["na hraně hodiny", NOW - GH_FRESH_MS, 1],
    ["starší než hodina", NOW - GH_FRESH_MS - 1, 0],
    ["nikdy nenačteno", null, 0],
    ["hodiny jdou zpět o půl minuty", NOW + 30_000, 1],
    ["hodiny daleko v budoucnu", NOW + 3_600_000, 0],
  ];
  for (const [name, at, n] of cases) assert.equal(pick(gh(at), new Map(), NOW).length, n, name);
});

test("dev server: jen běžící s portem a krátce po naběhnutí; nejnovější vyhrává", () => {
  const p = (o) => ({ id: "web", phase: "ready", port: 3000, url: null, readyAt: NOW - 1000, ...o });
  const cases = [
    ["naběhl", [p({})], 1],
    ["startuje", [p({ phase: "starting", readyAt: null })], 0],
    ["spadl", [p({ phase: "failed" })], 0],
    ["bez portu", [p({ port: null })], 0],
    ["běží dlouho", [p({ readyAt: NOW - DEV_FRESH_MS - 1 })], 0],
    ["na hraně", [p({ readyAt: NOW - DEV_FRESH_MS })], 1],
  ];
  for (const [name, procs, n] of cases) assert.equal(pick({ ...base(), procs }, new Map(), NOW).length, n, name);
  const two = pick({ ...base(), procs: [p({ id: "a", port: 1111, readyAt: NOW - 5000 }), p({ id: "b", port: 2222, readyAt: NOW - 100 })] }, new Map(), NOW);
  assert.equal(two[0].title, "localhost:2222");
  assert.equal(two[0].more, 1);
  assert.equal(two[0].detail, "b", "projekt bez názvu → id");
});

test("adresa dev serveru: jen loopback, jinak localhost:PORT", () => {
  const cases = [
    ["http://localhost:5173/", 5173, "http://localhost:5173/"],
    ["http://127.0.0.1:8080", 8080, "http://127.0.0.1:8080"],
    ["http://[::1]:3000/x", 3000, "http://[::1]:3000/x"],
    ["https://evil.example/", 3000, "http://localhost:3000"],
    ["http://localhost.evil.example:3000/", 3000, "http://localhost:3000"],
    [null, 4000, "http://localhost:4000"],
  ];
  for (const [url, port, want] of cases) assert.equal(devUrl(url, port), want, String(url));
});

test("zavření: klíč na 24 h, pak se vrátí; prošlé záznamy odpadnou", () => {
  const input = { ...base(), github: { review: [pr(3), pr(4)], failing: [], fetchedMs: NOW } };
  let d = dismiss(new Map(), "gh-review:a/b#3", NOW);
  assert.ok(isDismissed(d, "gh-review:a/b#3", NOW + DISMISS_MS - 1));
  assert.ok(!isDismissed(d, "gh-review:a/b#3", NOW + DISMISS_MS));
  const got = pick(input, d, NOW);
  assert.equal(got[0].title, "a/b#4", "zavřené PR se přeskočí, další je na řadě");
  assert.equal(got[0].more, 0);
  d = dismiss(d, "gh-review:a/b#4", NOW);
  assert.deepEqual(pick(input, d, NOW), [], "zavřené všechny");
  const later = NOW + DISMISS_MS + 5;
  assert.equal(pick({ ...input, github: { ...input.github, fetchedMs: later } }, d, later).length, 1, "po 24 h se vrátí");
  const pruned = dismiss(d, "x", later);
  assert.deepEqual([...pruned.keys()], ["x"], "prošlé pryč");
  assert.equal(d.size, 2, "původní mapa netknutá");
});

test("zavřená připomínka se vrátí po přeplánování (jiný termín = jiný klíč)", () => {
  const r = (remindMs) => ({ ...base(), reminders: [{ path: "p.md", title: "t", remindMs }] });
  const d = dismiss(new Map(), "rem:p.md|100", NOW);
  assert.deepEqual(pick(r(100), d, NOW), []);
  assert.equal(pick(r(200), d, NOW).length, 1);
});

test("normalizace adres a „už mám mezi odkazy“", () => {
  const cases = [
    ["https://www.Example.com/a/", "example.com/a"],
    ["http://example.com:8080/a?x=1#h", "example.com:8080/a?x=1"],
    ["https://example.com", "example.com"],
    ["ftp://example.com", null],
    ["ahoj", null],
    ["", null],
  ];
  for (const [u, want] of cases) assert.equal(normUrl(u), want, u);
  const known = ["https://example.com/a", "https://github.com/x"];
  assert.ok(isKnown("https://www.example.com/a/", known));
  assert.ok(!isKnown("https://example.com/b", known));
  assert.ok(!isKnown("nonsense", known));
  assert.equal(hostOf("https://www.example.com/a"), "example.com");
  assert.equal(hostOf("nic"), "");
  assert.deepEqual(
    knownLinks([{ kind: "url", target: "https://a.cz" }, { kind: "folder", target: "C:\\x" }, { kind: "web", target: "https://b.cz" }], [{ kind: "web", target: "https://c.cz" }, { kind: "folder", target: "C:\\y" }]),
    ["https://a.cz", "https://b.cz", "https://c.cz"],
  );
  assert.deepEqual(knownLinks(undefined, undefined), []);
});

test("schránka: rozhoduje nejnovější položka, celá adresa, čerstvá", () => {
  const it = (preview, atMs, chars = preview.length) => ({ preview, atMs, chars });
  const cases = [
    ["bez historie", null, null],
    ["prázdná", [], null],
    ["adresa", [it("https://example.com/a", NOW - 100)], "https://example.com/a"],
    ["nejnovější není adresa → nic (starší adresa už není ve schránce)", [it("https://example.com/a", NOW - 500), it("ahoj", NOW - 100)], null],
    ["nejnovější je adresa", [it("ahoj", NOW - 500), it("https://example.com/a", NOW - 100)], "https://example.com/a"],
    ["text s adresou uvnitř", [it("koukni na https://example.com", NOW)], null],
    ["zkrácený náhled", [it("https://example.com/aaaa…", NOW, 300)], null],
    ["jiné schéma", [it("file:///C:/x", NOW)], null],
    ["stará", [it("https://example.com/a", NOW - CLIP_FRESH_MS - 1)], null],
    ["na hraně", [it("https://example.com/a", NOW - CLIP_FRESH_MS)], "https://example.com/a"],
  ];
  for (const [name, items, want] of cases) assert.equal(clipUrl(items, NOW)?.url ?? null, want, name);
});

test("schránka: nabídka po čase zmizí i bez nové události", () => {
  const input = { ...base(), clip: { url: "https://example.com/a", atMs: NOW } };
  assert.equal(pick(input, new Map(), NOW + CLIP_FRESH_MS).length, 1);
  assert.equal(pick(input, new Map(), NOW + CLIP_FRESH_MS + 1).length, 0);
});

test("schránka: adresa, kterou už mám, se nenabízí", () => {
  const input = { ...base(), clip: { url: "https://example.com/a", atMs: NOW }, known: ["https://www.example.com/a/"] };
  assert.deepEqual(pick(input, new Map(), NOW), []);
  assert.equal(pick({ ...input, known: ["https://other.cz"] }, new Map(), NOW).length, 1);
  const d = dismiss(new Map(), "clip:example.com/a", NOW);
  assert.deepEqual(pick({ ...input, known: [] }, d, NOW), [], "zavřená adresa se nevrací");
});

test("shoda chipů: tik času je nepřekreslí, změna obsahu ano", () => {
  const input = { ...base(), github: { review: [pr(3), pr(4)], failing: [], fetchedMs: NOW } };
  const a = pick(input, new Map(), NOW);
  const b = pick(input, new Map(), NOW + 1000);
  assert.notEqual(a[0], b[0], "objekty jsou pokaždé nové");
  assert.ok(sameSuggestions(a, b));
  assert.ok(!sameSuggestions(a, pick(input, dismiss(new Map(), a[0].key, NOW), NOW)), "zavřený chip");
  assert.ok(!sameSuggestions(a, []), "jiný počet");
  assert.ok(sameSuggestions([], []));
});
