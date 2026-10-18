// Tabulkové testy rozložení šuplíku (`model.ts`): zrcadlo Rustu, normalizace, úpravy karet a záložek,
// katalog, co stránka ukáže, cíl tažení v mřížce.
//   node --experimental-strip-types --test src/layout/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  CARD_IDS, CARD_INFO, DEFAULT_TILES, MAX_PAGES, PARTS, PART_INFO, QUICK_HEIGHTS, UP_PERSONAL_PROJECTS, addCard, addPage, catalog, clone, defaultLayout,
  gridDropIndex, keyMove, moveCard, movePage, moveToPage, nextHeight, nextSize, normalizeLayout, normalizeTiles, pageIdFor, pageKind, pageNameError, pageOf,
  removeCard, removePage, renamePage, restoreCard, restorePage, sameLayout, setHeight, setPartHidden, setSize, setTiles, showProject, shownCards, tilesOf,
  visibleProjects, withLayout,
} from "./model.ts";

const ids = (l, page) => pageOf(l, page).cards.map((c) => c.id);

test("zrcadlo Rustu: karty a části jako v layout.rs", () => {
  const src = readFileSync(new URL("../../src-tauri/src/layout.rs", import.meta.url), "utf8");
  const cards = src.match(/pub const CARDS: \[&str; \d+\] = \[([^\]]+)\]/)[1].match(/"([a-z]+)"/g).map((s) => s.slice(1, -1));
  assert.deepEqual(cards, [...CARD_IDS]);
  const block = src.slice(src.indexOf("pub const PARTS"), src.indexOf("];", src.indexOf("pub const PARTS")));
  const parts = [...block.matchAll(/\("([A-Za-z]+)", "([a-z]+)"\)/g)].map((m) => [m[1], m[2]]);
  assert.deepEqual(parts, Object.entries(PARTS));
  // výchozí velikosti = výchozí rozložení v Rustu (Práce: Běží xl, Claude a Repa m)
  const d = defaultLayout();
  assert.deepEqual(d.pages[0].cards.map((c) => `${c.id}:${c.size}`), ["projects:xl", "actions:xl", "shelf:xl", "running:xl", "claude:m", "repos:m", "notes:xl"]);
  assert.deepEqual(d.pages[1].cards.map((c) => `${c.id}:${c.size}`), ["projects:xl", "clock:s", "timer:s", "weather:s", "calendar:s", "media:m", "links:m", "actions:xl", "shelf:xl", "notes:xl"]);
  assert.deepEqual(d.upgrades, [UP_PERSONAL_PROJECTS], "výchozí = jednorázové úpravy už hotové");
  const up = src.match(/pub const UP_PERSONAL_PROJECTS: &str = "([A-Za-z]+)"/)[1];
  assert.equal(up, UP_PERSONAL_PROJECTS);
  const maxH = Number(src.match(/pub const MAX_H: u8 = (\d+);/)[1]);
  assert.equal(maxH, 4);
  for (const id of CARD_IDS) assert.ok(CARD_INFO[id].name[0].startsWith("cards.layout.card.") && CARD_INFO[id].desc[1], id);
  for (const k of Object.keys(PARTS)) assert.ok(PART_INFO[k], k);
});

test("normalizace: obranné čtení", () => {
  const cases = [
    ["nic", undefined, defaultLayout()],
    ["bez záložek", { pages: [] }, defaultLayout()],
    [
      "neznámé pryč",
      { pages: [{ id: "a", name: "A", icon: "🎮", cards: [{ id: "clock", size: "xl" }, { id: "pocasi" }, { id: "clock", size: "s" }, { id: "links", size: "huge" }] }, { id: "a" }, { name: "bez id" }], hiddenParts: ["sound", "git", "sound", 3] },
      { pages: [{ id: "a", name: "A", icon: "🎮", cards: [{ id: "clock", size: "xl" }, { id: "links", size: "m" }] }], hiddenParts: ["sound"] },
    ],
  ];
  for (const [name, raw, want] of cases) assert.deepEqual(normalizeLayout(raw), want, name);
  assert.ok(sameLayout(defaultLayout(), normalizeLayout(JSON.parse(JSON.stringify(defaultLayout())))));
});

test("karty: přesun, velikost, odebrat + Vrátit, přidat, na jinou záložku", () => {
  const d = defaultLayout();
  const before = JSON.stringify(d);
  assert.deepEqual(ids(moveCard(d, "prace", 4, 0), "prace").slice(0, 2), ["claude", "projects"]);
  assert.deepEqual(ids(moveCard(d, "prace", 0, 99), "prace"), ids(d, "prace"), "mimo rozsah = beze změny");
  assert.equal(pageOf(setSize(d, "osobni", "clock", "l"), "osobni").cards.find((c) => c.id === "clock").size, "l");
  assert.deepEqual(["s", "m", "l", "xl"].map(nextSize), ["m", "l", "xl", "s"]);

  const r = removeCard(d, "prace", "claude");
  assert.ok(!ids(r.layout, "prace").includes("claude"));
  assert.deepEqual(ids(restoreCard(r.layout, r.undo), "prace"), ids(d, "prace"), "Vrátit = totéž místo");
  assert.equal(removeCard(d, "prace", "weather").undo, null, "karta tam není");

  assert.deepEqual(ids(addCard(d, "prace", "weather"), "prace").at(-1), "weather");
  assert.equal(pageOf(addCard(d, "prace", "weather"), "prace").cards.at(-1).size, "s", "výchozí velikost");
  assert.equal(addCard(d, "prace", "claude"), d, "už tam je");
  // Právě hraje bez obou částí se vrátí celá
  const noMedia = setPartHidden(setPartHidden(removeCard(d, "osobni", "media").layout, "nowPlaying", true), "sound", true);
  assert.deepEqual(addCard(noMedia, "osobni", "media").hiddenParts, []);
  assert.deepEqual(addCard(setPartHidden(d, "claudeCosts", true), "osobni", "claude").hiddenParts, ["claudeCosts"], "jiné části zůstanou skryté");

  const m = moveToPage(d, "prace", "repos", "osobni");
  assert.ok(!ids(m, "prace").includes("repos") && ids(m, "osobni").at(-1) === "repos");
  assert.equal(pageOf(m, "osobni").cards.at(-1).size, "m", "velikost jde s kartou");
  const dup = moveToPage(d, "prace", "notes", "osobni");
  assert.equal(ids(dup, "osobni").filter((x) => x === "notes").length, 1, "už tam je → jen zmizí ze zdroje");
  assert.equal(moveToPage(d, "prace", "notes", "nikde"), d);
  assert.equal(JSON.stringify(d), before, "vstup se nemění");
});

test("katalog: co na záložce není a kde to je", () => {
  const d = defaultLayout();
  assert.deepEqual(catalog(d, "prace").map((x) => x.id), ["clock", "timer", "weather", "calendar", "links", "media"]);
  assert.deepEqual(catalog(d, "prace").find((x) => x.id === "weather").on, ["osobni"]);
  const hidden = removeCard(removeCard(d, "prace", "notes").layout, "osobni", "notes").layout;
  assert.deepEqual(catalog(hidden, "prace").find((x) => x.id === "notes").on, [], "nikde = skrytá");
  const withNew = addPage(d, "Hry").layout;
  assert.equal(catalog(withNew, "hry").length, CARD_IDS.length, "prázdná záložka = celý katalog");
});

test("záložky: přidat, id, přejmenovat, smazat + Vrátit, pořadí, limity", () => {
  const d = defaultLayout();
  const a = addPage(d, "Hry a hudba", "🎮");
  assert.equal(a.id, "hry-a-hudba");
  assert.deepEqual(pageOf(a.layout, a.id), { id: "hry-a-hudba", name: "Hry a hudba", icon: "🎮", cards: [] });
  const cases = [
    ["Žluťoučký kůň", "zlutoucky-kun"],
    ["Práce", "prace-2"],
    ["!!!", "zalozka"],
  ];
  for (const [name, want] of cases) assert.equal(pageIdFor(d, name), want, name);
  assert.equal(addPage(d, "   "), null, "bez názvu");
  let full = d;
  for (let i = 0; i < MAX_PAGES - 2; i++) full = addPage(full, `Z${i}`).layout;
  assert.equal(full.pages.length, MAX_PAGES);
  assert.equal(addPage(full, "Navíc"), null, "nejvýš 9");

  assert.equal(pageOf(renamePage(d, "prace", "Kancl"), "prace").name, "Kancl");
  assert.equal(pageOf(renamePage(d, "prace", ""), "prace").name, "", "vestavěná smí zpět na výchozí název");
  assert.equal(renamePage(a.layout, a.id, " "), a.layout, "vlastní bez názvu ne");
  assert.equal(pageOf(renamePage(a.layout, a.id, "Hry", ""), a.id).icon, undefined, "ikona pryč");
  assert.deepEqual(pageNameError("x".repeat(40)), ["cards.layout.tab.nameLong", "Název je moc dlouhý"]);

  const r = removePage(d, "osobni");
  assert.deepEqual(r.layout.pages.map((p) => p.id), ["prace"]);
  assert.deepEqual(restorePage(r.layout, r.undo), d);
  assert.equal(removePage(r.layout, "prace").undo, null, "poslední zůstane");
  assert.deepEqual(movePage(d, 1, 0).pages.map((p) => p.id), ["osobni", "prace"]);
  assert.equal(pageOf(d, "smazana").id, "prace", "neznámá = první");
  assert.deepEqual(["prace", "osobni", "hry"].map(pageKind), ["work", "personal", "any"]);
});

test("co stránka ukáže: karty bez dat se schovají samy", () => {
  const f = (over = {}) => ({ procs: 0, notesEnabled: true, actions: 0, scriptOut: false, shelf: false, media: false, ...over });
  const d = defaultLayout();
  const custom = { id: "hry", name: "Hry", cards: [{ id: "actions", size: "xl" }, { id: "media", size: "m" }] };
  const cases = [
    [pageOf(d, "prace"), f(), ["projects", "claude", "repos", "notes"]],
    [pageOf(d, "prace"), f({ procs: 1, actions: 2, shelf: true }), ["projects", "actions", "shelf", "running", "claude", "repos", "notes"]],
    [pageOf(d, "prace"), f({ scriptOut: true, notesEnabled: false }), ["projects", "actions", "claude", "repos"]],
    [pageOf(d, "osobni"), f(), ["projects", "clock", "timer", "weather", "calendar", "links", "actions", "notes"]],
    [pageOf(d, "osobni"), f({ media: true }), ["projects", "clock", "timer", "weather", "calendar", "media", "links", "actions", "notes"]],
    [custom, f(), ["actions"]],
  ];
  for (const [p, facts, want] of cases) assert.deepEqual(shownCards(p, facts).map((c) => c.id), want, `${p.id} ${JSON.stringify(facts)}`);
});

test("withLayout mění jen jedno pole", () => {
  const cfg = { drawer: { width: 900 }, projects: [{ id: "a" }] };
  const out = withLayout(cfg, defaultLayout());
  assert.equal(out.projects, cfg.projects);
  assert.equal(cfg.layout, undefined);
  assert.deepEqual(out.layout, defaultLayout());
});

test("tažení v mřížce: nejbližší střed; Alt+šipky", () => {
  // 2×2 mřížka po 100 px
  const r = [
    { x: 0, y: 0, w: 100, h: 100 },
    { x: 100, y: 0, w: 100, h: 100 },
    { x: 0, y: 100, w: 200, h: 100 },
    { x: 200, y: 0, w: 100, h: 200 },
  ];
  const cases = [
    [0, 10, 10, 0],
    [0, 160, 40, 1],
    [0, 60, 170, 2],
    [1, 280, 150, 3],
    [2, -500, -500, 0],
  ];
  for (const [from, x, y, want] of cases) assert.equal(gridDropIndex(r, from, x, y), want, `${x},${y}`);
  assert.equal(gridDropIndex([], 3, 0, 0), 3);
  const keys = [
    ["ArrowLeft", 1, 3, 0],
    ["ArrowUp", 0, 3, null],
    ["ArrowRight", 1, 3, 2],
    ["ArrowDown", 2, 3, null],
    ["Enter", 1, 3, null],
  ];
  for (const [k, i, n, want] of keys) assert.equal(keyMove(k, i, n), want, `${k} ${i}`);
});

test("výška karty: normalizace, nastavení, kruh ↕, rychlé volby", () => {
  const raw = { pages: [{ id: "a", name: "A", cards: [{ id: "claude", size: "m", h: 3 }, { id: "repos", size: "m", h: 9 }, { id: "notes", size: "xl", h: "2" }] }] };
  const l = normalizeLayout(raw);
  assert.deepEqual(l.pages[0].cards, [{ id: "claude", size: "m", h: 3 }, { id: "repos", size: "m" }, { id: "notes", size: "xl" }], "jen 1–4, číslo");
  const d = defaultLayout();
  const before = JSON.stringify(d);
  const set = setHeight(d, "prace", "claude", 2);
  assert.equal(pageOf(set, "prace").cards.find((c) => c.id === "claude").h, 2);
  assert.ok(!("h" in pageOf(setHeight(set, "prace", "claude", undefined), "prace").cards.find((c) => c.id === "claude")), "auto = pole pryč");
  assert.equal(JSON.stringify(d), before, "vstup se nemění");
  assert.deepEqual([undefined, 1, 2, 3, 4].map(nextHeight), [1, 2, 3, 4, undefined]);
  assert.deepEqual(QUICK_HEIGHTS.map((q) => q.h), [undefined, 2, 4]);
  // výška jde s kartou na jinou záložku i přes Vrátit
  const m = moveToPage(set, "prace", "claude", "osobni");
  assert.equal(pageOf(m, "osobni").cards.at(-1).h, 2);
  const r = removeCard(set, "prace", "claude");
  assert.equal(pageOf(restoreCard(r.layout, r.undo), "prace").cards.find((c) => c.id === "claude").h, 2);
});

test("zástupci (karta Projekty): volby dlaždic, skryté projekty, výchozí = bez pole", () => {
  const cases = [
    ["nic", undefined, DEFAULT_TILES],
    ["část", { size: "small", git: false }, { size: "small", label: true, git: false, hidden: [] }],
    ["nesmysl", { size: "huge", label: "ano", hidden: ["a", 3, "a", ""] }, { size: "tile", label: true, git: true, hidden: ["a"] }],
  ];
  for (const [name, raw, want] of cases) assert.deepEqual(normalizeTiles(raw), want, name);
  const d = defaultLayout();
  const s = setTiles(d, "osobni", { size: "large" });
  assert.deepEqual(tilesOf(pageOf(s, "osobni").cards[0]), { ...DEFAULT_TILES, size: "large" });
  assert.equal(pageOf(s, "prace").cards[0].tiles, undefined, "jiná záložka beze změny");
  assert.equal(pageOf(setTiles(s, "osobni", { size: "tile" }), "osobni").cards[0].tiles, undefined, "zpět na výchozí = pole pryč");
  const h = showProject(showProject(d, "osobni", "harbor", false), "osobni", "kino", false);
  assert.deepEqual(tilesOf(pageOf(h, "osobni").cards[0]).hidden, ["harbor", "kino"]);
  assert.deepEqual(tilesOf(pageOf(showProject(h, "osobni", "harbor", true), "osobni").cards[0]).hidden, ["kino"]);
  const projects = [{ id: "harbor" }, { id: "kino" }, { id: "ledge" }];
  assert.deepEqual(visibleProjects(projects, tilesOf(pageOf(h, "osobni").cards[0])).map((p) => p.id), ["ledge"]);
  assert.deepEqual(visibleProjects(projects, DEFAULT_TILES).length, 3, "výchozí = všechny (i nový projekt)");
  // volby přežijí normalizaci, klon a přesun na jinou záložku
  const n = normalizeLayout(JSON.parse(JSON.stringify(h)));
  assert.deepEqual(tilesOf(pageOf(n, "osobni").cards[0]).hidden, ["harbor", "kino"]);
  const c = clone(h);
  c.pages[1].cards[0].tiles.hidden.push("x");
  assert.deepEqual(tilesOf(pageOf(h, "osobni").cards[0]).hidden, ["harbor", "kino"], "klon je hluboký");
});

test("značka jednorázových úprav přežije normalizaci i uložení (jinak by se odebrané Projekty vracely)", () => {
  const raw = { pages: [{ id: "osobni", name: "", cards: [{ id: "clock", size: "s" }] }], hiddenParts: [], upgrades: ["personalProjects", 7, "personalProjects"] };
  const l = normalizeLayout(raw);
  assert.deepEqual(l.upgrades, ["personalProjects"]);
  assert.deepEqual(withLayout({ ui: {} }, l).layout.upgrades, ["personalProjects"]);
  assert.deepEqual(clone(l).upgrades, ["personalProjects"]);
  assert.ok(!("upgrades" in normalizeLayout({ pages: [{ id: "a", name: "A", cards: [] }] })), "chybí = nepsat");
  assert.ok(!sameLayout(l, { ...l, upgrades: [] }), "značka je součást rozložení");
});
