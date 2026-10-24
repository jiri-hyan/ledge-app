// Kostra Nastavení (kolo 21) bez DOM: sekce pokrývají každou stránku z registru právě jednou, Doporučené
// vedou na existující volby, navštívené stránky se drží naživu (čistá `withVisited`), klik na sekci vrátí
// naposledy otevřenou stránku, přepínač Základní / Pokročilé, a skořápka opravdu drží stránky naživu
// (žádné `<For each={[panel()]}>` + `<Dynamic>`, které stránku při přepnutí zničí), náhled po najetí
// nejde na živý šuplík a výška Nastavení je pevná.
//   node --experimental-strip-types --test src/settings/structure.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { FIRST_PAGE, PAGE_ORDER, RECOMMENDED, SECTIONS, pageForSection, sectionOf, withVisited } from "./structure.ts";
import { PANEL_ALIASES, PANEL_META } from "./registry.ts";
import { panelForField, resolveTarget } from "./model.ts";
import { parseMode } from "./mode.ts";

const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");

test("sekce: 8 (Doporučené + 7 podle úkolů), každá stránka z registru právě v jedné", () => {
  assert.deepEqual(SECTIONS.map((s) => s.id), ["doporucene", "vzhled", "suplik", "claude", "akce", "poznamky", "naplose", "aplikace"]);
  const ids = PANEL_META.map((p) => p.id).sort();
  assert.deepEqual([...PAGE_ORDER].sort(), ids, "PAGE_ORDER = registr");
  assert.equal(new Set(PAGE_ORDER).size, PAGE_ORDER.length, "žádná stránka ve dvou sekcích");
  assert.equal(FIRST_PAGE, "doporucene");
  assert.equal(sectionOf("chovani")?.id, "suplik");
  assert.equal(sectionOf("zaciname")?.id, "aplikace");
  assert.equal(sectionOf("neznamo"), null);
  for (const s of SECTIONS) assert.ok(s.pages.length >= 1 && s.desc[1].length > 10, `${s.id}: stránky a věta`);
  // každá sekce má ikonu v navigaci
  const icons = read("icons.tsx");
  for (const s of SECTIONS) assert.ok(new RegExp(`\\b${s.id}: "`).test(icons), `ikona ${s.id}`);
});

test("Doporučené: každý cíl vede na existující stránku a kotvu", () => {
  const ids = PANEL_META.map((p) => p.id);
  const fields = Object.fromEntries([...read("labels.ts").matchAll(/^\s*"?([\w.]+)"?: \{ panel: "(\w+)"/gm)].map((m) => [m[1], m[2]]));
  assert.ok(RECOMMENDED.length >= 8 && RECOMMENDED.length <= 14, "nejčastější, ne všechno");
  for (const r of RECOMMENDED) {
    const to = resolveTarget(r.go, ids, PANEL_ALIASES, (f) => fields[f] ?? panelForField(f));
    assert.ok(to.panel, `${r.go} → stránka`);
    assert.ok(r.text[1].length > 10, `${r.go}: jedna věta, co volba dělá`);
  }
});

test("withVisited: nová na konec, bez duplicit, strop (nejstarší vypadne)", () => {
  assert.deepEqual(withVisited([], "a"), ["a"]);
  assert.deepEqual(withVisited(["a", "b"], "a"), ["a", "b"]);
  assert.deepEqual(withVisited(["a", "b"], "c"), ["a", "b", "c"]);
  assert.deepEqual(withVisited(["a", "b", "c"], "d", 3), ["b", "c", "d"]);
  const same = ["x"];
  assert.equal(withVisited(same, "x"), same, "beze změny stejný seznam (žádné přepočítání For)");
});

test("pageForSection: naposledy otevřená stránka sekce, jinak první", () => {
  const s = SECTIONS.find((x) => x.id === "suplik");
  assert.equal(pageForSection(s, {}), "suplik");
  assert.equal(pageForSection(s, { suplik: "osobni" }), "osobni");
  assert.equal(pageForSection(s, { suplik: "claude" }), "suplik", "cizí stránka se nepoužije");
});

test("parseMode: jen „advanced“ je Pokročilé, cokoli jiného Základní", () => {
  for (const [raw, want] of [["advanced", "advanced"], ["basic", "basic"], [null, "basic"], ["", "basic"], ["ADVANCED", "basic"]]) assert.equal(parseMode(raw), want, String(raw));
});

test("skořápka drží stránky naživu, najetí nemění živý šuplík, výška je pevná", () => {
  const shell = read("../Settings.tsx");
  assert.ok(!/<For each=\{\[panel\(\)\]\}>/.test(shell), "žádné přestavění stránky při přepnutí");
  assert.match(shell, /<For each=\{visited\(\)\}>/, "navštívené stránky");
  assert.match(shell, /hidden=\{active\(\) !== id\}/, "přepnutí jen skryje");
  assert.match(read("../settings.css"), /\.st-panel\[hidden\] \{ display: none/);
  assert.match(read("../settings.css"), /\n {2}height: min\(\d+px, calc\(var\(--drawer-max-h/, "pevná výška");
  const ap = read("panels/Appearance.tsx");
  assert.ok(!/applyAppearance\(peek\(\)/.test(ap), "Vzhled neaplikuje náhled najetí na celý šuplík");
  assert.match(read("PreviewDock.tsx"), /peek\(\) \?\? withThemeDraft/, "najetí jde do náhledu dole");
});
