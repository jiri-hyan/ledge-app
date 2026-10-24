// Tabulkové testy rozložení šuplíku (`model.ts`): zrcadlo Rustu, normalizace, úpravy karet a záložek,
// katalog, co stránka ukáže, cíl tažení v mřížce.
//   node --experimental-strip-types --test src/layout/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  CARD_IDS, CARD_INFO, DEFAULT_TILES, MAX_PAGES, PARTS, PART_INFO, QUICK_HEIGHTS, UP_PERSONAL_PROJECTS, addCard, addPage, catalog, clone, defaultLayout,
  gridDropIndex, keyMove, pageMoveBlock, moveCard, movePage, moveToPage, nextHeight, nextSize, normalizeLayout, normalizeTiles, pageIdFor, pageKind, pageNameError, pageOf,
  removeCard, removePage, renamePage, restoreCard, restorePage, sameLayout, setHeight, setPartHidden, setSize, setTiles, showProject, shownCards, tilesOf,
  visibleProjects, withLayout, BODY, CARD_SLOTS, movePart, moveToNewPage, orderParts, partsOf, setParts, slotsOf,
} from "./model.ts";
import { fullLayout } from "./fullLayout.mjs";

const ids = (l, page) => pageOf(l, page).cards.map((c) => c.id);

test("zrcadlo Rustu: karty a části jako v layout.rs", () => {
  const src = readFileSync(new URL("../../src-tauri/src/layout.rs", import.meta.url), "utf8");
  const cards = src.match(/pub const CARDS: \[&str; \d+\] = \[([^\]]+)\]/)[1].match(/"([a-z]+)"/g).map((s) => s.slice(1, -1));
  assert.deepEqual(cards, [...CARD_IDS]);
  const block = src.slice(src.indexOf("pub const PARTS"), src.indexOf("];", src.indexOf("pub const PARTS")));
  const parts = [...block.matchAll(/\("([A-Za-z]+)", "([a-z]+)"\)/g)].map((m) => [m[1], m[2]]);
  assert.deepEqual(parts, Object.entries(PARTS));
  // výchozí rozložení = fixture, kterou čte i `layout::tests::default_is_clean_start` (zrcadlo Rustu)
  const d = defaultLayout();
  const fx = JSON.parse(readFileSync(new URL("../../src-tauri/src/fixtures/layout-default.json", import.meta.url), "utf8"));
  assert.deepEqual(d.pages.map((p) => [p.id, p.cards.map((c) => `${c.id}:${c.size}`)]), fx.pages.map((p) => [p.id, p.cards.map((c) => `${c.id}:${c.size}`)]));
  assert.deepEqual(d.pages[0].cards.map((c) => c.id), ["projects", "notes"], "čistý začátek: Práce jen Projekty + Poznámky");
  assert.deepEqual(d.pages[1].cards.map((c) => c.id), ["clock", "media", "notes", "links"]);
  const none = ["calendar", "mail", "github", "system", "weather", "actions", "today"];
  assert.ok(d.pages.every((p) => p.cards.every((c) => !none.includes(c.id))), "karty bez zdroje a duplicitní akce jen v katalogu");
  assert.deepEqual(d.upgrades, fx.upgrades);
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
  const d = fullLayout();
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
  // Právě hraje bez všech částí (skladba, hlasitost, oblíbené) se vrátí celá
  const noMedia = ["nowPlaying", "sound", "favorites"].reduce((l, k) => setPartHidden(l, k, true), removeCard(d, "osobni", "media").layout);
  assert.deepEqual(addCard(noMedia, "osobni", "media").hiddenParts, []);
  assert.deepEqual(addCard(setPartHidden(d, "claudeCosts", true), "osobni", "claude").hiddenParts, ["claudeCosts"], "jiné části zůstanou skryté");

  const m = moveToPage(d, "prace", "repos", "osobni");
  assert.ok(!ids(m, "prace").includes("repos") && ids(m, "osobni").at(-1) === "repos");
  assert.equal(pageOf(m, "osobni").cards.at(-1).size, "m", "velikost jde s kartou");
  // případ 7 (vlna 9): karta, která už na cílové záložce je, dřív ze zdrojové jen zmizela
  const dup = moveToPage(d, "prace", "notes", "osobni");
  assert.equal(dup, d, "už tam je → nic se nezmění, karta nezmizí");
  assert.equal(pageMoveBlock(d, "prace", "notes", "osobni"), "there");
  assert.equal(pageMoveBlock(d, "prace", "repos", "osobni"), null);
  assert.equal(pageMoveBlock(d, "prace", "repos", "prace"), "same");
  assert.equal(pageMoveBlock(d, "prace", "repos", "nikde"), "missing");
  assert.equal(moveToPage(d, "prace", "notes", "nikde"), d);
  assert.equal(JSON.stringify(d), before, "vstup se nemění");
});

test("katalog: co na záložce není a kde to je", () => {
  const d = fullLayout();
  assert.deepEqual(catalog(d, "prace").map((x) => x.id), ["clock", "timer", "weather", "calendar", "links", "media", "github", "system", "mail", "today", "usage", "week", "bigclock", "toggles"]);
  // GitHub, Systém a Pošta jsou v katalogu, ale na žádné záložce (výchozí rozložení se nemění)
  assert.deepEqual(catalog(d, "prace").filter((x) => ["github", "system", "mail", "today"].includes(x.id)).map((x) => x.on), [[], [], [], []]);
  assert.deepEqual(catalog(d, "osobni").find((x) => x.id === "mail").on, []);
  assert.deepEqual(catalog(d, "prace").find((x) => x.id === "weather").on, ["osobni"]);
  const hidden = removeCard(removeCard(d, "prace", "notes").layout, "osobni", "notes").layout;
  assert.deepEqual(catalog(hidden, "prace").find((x) => x.id === "notes").on, [], "nikde = skrytá");
  const withNew = addPage(d, "Hry").layout;
  assert.equal(catalog(withNew, "hry").length, CARD_IDS.length, "prázdná záložka = celý katalog");
});

test("záložky: přidat, id, přejmenovat, smazat + Vrátit, pořadí, limity", () => {
  const d = fullLayout();
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
  const f = (over = {}) => ({ procs: 0, notesEnabled: true, actions: 0, scriptOut: false, shelf: false, media: false, today: false, ...over });
  const d = fullLayout();
  const custom = { id: "hry", name: "Hry", cards: [{ id: "actions", size: "xl" }, { id: "media", size: "m" }] };
  const cases = [
    [pageOf(d, "prace"), f(), ["projects", "claude", "repos", "notes"]],
    [pageOf(d, "prace"), f({ procs: 1, actions: 2, shelf: true }), ["projects", "actions", "shelf", "running", "claude", "repos", "notes"]],
    [pageOf(d, "prace"), f({ scriptOut: true, notesEnabled: false }), ["projects", "actions", "claude", "repos"]],
    [pageOf(d, "osobni"), f(), ["projects", "clock", "timer", "weather", "calendar", "links", "actions", "notes"]],
    [pageOf(d, "osobni"), f({ media: true }), ["projects", "clock", "timer", "weather", "calendar", "media", "links", "actions", "notes"]],
    [custom, f(), ["actions"]],
    // karta Dnes bez řádků se nekreslí, s řádkem ano
    [{ id: "x", name: "X", cards: [{ id: "today", size: "m" }, { id: "clock", size: "s" }] }, f(), ["clock"]],
    [{ id: "x", name: "X", cards: [{ id: "today", size: "m" }, { id: "clock", size: "s" }] }, f({ today: true }), ["today", "clock"]],
  ];
  for (const [p, facts, want] of cases) assert.deepEqual(shownCards(p, facts).map((c) => c.id), want, `${p.id} ${JSON.stringify(facts)}`);
});

test("withLayout mění jen jedno pole", () => {
  const cfg = { drawer: { width: 900 }, projects: [{ id: "a" }] };
  const out = withLayout(cfg, fullLayout());
  assert.equal(out.projects, cfg.projects);
  assert.equal(cfg.layout, undefined);
  assert.deepEqual(out.layout, fullLayout());
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
    // případ 4 (vlna 9): pustit daleko mimo karty = zůstane, kde byla (dřív nejbližší karta = přesun)
    [2, -500, -500, 2],
    [2, 150, 260, 2],
    [0, 150, 230, 2],
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

test("dynamické karty: widget ze skriptu na libovolné záložce, neplatné id pryč, katalog jen vestavěné", () => {
  const raw = {
    pages: [
      { id: "prace", cards: [{ id: "widget:kurz-cnb.ps1", size: "s" }, { id: "widget:..\\zlo.ps1" }, { id: "widget:" }, { id: "widget:kurz-cnb.ps1" }, { id: "widget:docker.ps1" }] },
      { id: "osobni", cards: [{ id: "widget:kurz-cnb.ps1", size: "xl" }] },
    ],
  };
  const l = normalizeLayout(raw);
  assert.deepEqual(pageOf(l, "prace").cards, [{ id: "widget:kurz-cnb.ps1", size: "s" }, { id: "widget:docker.ps1", size: "m" }], "bez velikosti = střední, cesta a prázdné pryč, bez duplicit");
  assert.deepEqual(ids(l, "osobni"), ["widget:kurz-cnb.ps1"], "stejný widget na dvou záložkách");
  const added = addCard(l, "osobni", "widget:ping.ps1");
  assert.deepEqual(pageOf(added, "osobni").cards.at(-1), { id: "widget:ping.ps1", size: "m" });
  const { layout, undo } = removeCard(added, "osobni", "widget:ping.ps1");
  assert.deepEqual(ids(restoreCard(layout, undo), "osobni"), ["widget:kurz-cnb.ps1", "widget:ping.ps1"]);
  assert.ok(catalog(l, "prace").every((x) => !x.id.startsWith("widget:")), "widgety nabízí katalog ze složky (widgets/model.ts)");
  // Rust layout.rs přijímá stejný tvar (is_dynamic → widgets::plan::file_of)
  const rs = readFileSync(new URL("../../src-tauri/src/layout.rs", import.meta.url), "utf8");
  assert.match(rs, /pub fn is_dynamic\(id: &str\) -> bool/);
});

test("výška karty: normalizace, nastavení, kruh ↕, rychlé volby", () => {
  const raw = { pages: [{ id: "a", name: "A", cards: [{ id: "claude", size: "m", h: 3 }, { id: "repos", size: "m", h: 9 }, { id: "notes", size: "xl", h: "2" }] }] };
  const l = normalizeLayout(raw);
  assert.deepEqual(l.pages[0].cards, [{ id: "claude", size: "m", h: 3 }, { id: "repos", size: "m" }, { id: "notes", size: "xl" }], "jen 1–4, číslo");
  const d = fullLayout();
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
  const d = fullLayout();
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

test("zrcadlo Rustu: části karet v pořadí (card_slots) a pseudo-část body", () => {
  const src = readFileSync(new URL("../../src-tauri/src/layout.rs", import.meta.url), "utf8");
  assert.equal(src.match(/pub const BODY: &str = "([a-z]+)"/)[1], BODY);
  const block = src.slice(src.indexOf("pub fn card_slots"), src.indexOf("pub const MAX_PAGES"));
  const rs = Object.fromEntries([...block.matchAll(/"([a-z]+)" => &\[([^\]]+)\]/g)].map((m) => [m[1], m[2].split(",").map((x) => x.trim().replace(/^"|"$/g, "").replace("BODY", BODY))]));
  assert.deepEqual(rs, CARD_SLOTS);
  // každá přetahovatelná část je skrytelná (nebo body) a patří té kartě
  for (const [card, slots] of Object.entries(CARD_SLOTS)) for (const k of slots) assert.ok(k === BODY || PARTS[k] === card, `${card}: ${k}`);
});

test("pořadí částí karty: normalizace, doplnění, výchozí pořadí se nepíše", () => {
  // (popis, uložené, karta, výsledek)
  const cases = [
    ["bez pole = výchozí", undefined, "claude", ["claudeLimits", BODY, "claudeCosts"]],
    ["prohozené", ["claudeCosts", BODY, "claudeLimits"], "claude", ["claudeCosts", BODY, "claudeLimits"]],
    ["neúplné se doplní ve výchozím pořadí", ["claudeCosts"], "claude", ["claudeCosts", "claudeLimits", BODY]],
    ["neznámé a dvojité pryč", ["sound", "nope", "sound", 5, "nowPlaying"], "media", ["sound", "nowPlaying", "favorites"]],
    ["část cizí karty pryč", ["battery"], "media", ["nowPlaying", "sound", "favorites"]],
    ["karta bez částí", ["body"], "notes", []],
    ["nesmysl", "claude", "claude", ["claudeLimits", BODY, "claudeCosts"]],
  ];
  for (const [name, saved, card, want] of cases) assert.deepEqual(orderParts(saved, card), want, name);
  assert.deepEqual(slotsOf("widget:x.js"), []);
  const raw = { pages: [{ id: "osobni", name: "", cards: [{ id: "clock", size: "s", parts: ["battery", "body"] }, { id: "media", size: "m", parts: ["nowPlaying", "sound", "favorites"] }, { id: "notes", size: "xl", parts: ["body"] }] }] };
  const l = normalizeLayout(raw);
  const cs = pageOf(l, "osobni").cards;
  assert.deepEqual(cs[0].parts, ["battery", BODY]);
  assert.ok(!("parts" in cs[1]), "výchozí pořadí = nepsat");
  assert.ok(!("parts" in cs[2]), "karta bez částí = nepsat");
  assert.deepEqual(clone(l).pages[0].cards[0].parts, ["battery", BODY]);
  assert.notEqual(clone(l).pages[0].cards[0].parts, cs[0].parts, "klon je hluboký");
});

test("přesun části: movePart / setParts, výchozí pořadí pole smaže, ostatní karty nedotčené", () => {
  const l = fullLayout();
  const card = (x, page = "prace") => pageOf(x, page).cards.find((c) => c.id === "claude");
  const a = movePart(l, "prace", "claude", 0, 2);
  assert.deepEqual(card(a).parts, [BODY, "claudeCosts", "claudeLimits"]);
  assert.deepEqual(partsOf(card(a)), [BODY, "claudeCosts", "claudeLimits"]);
  assert.ok(!("parts" in card(l)), "původní vstup netknutý");
  const back = movePart(a, "prace", "claude", 2, 0);
  assert.ok(!("parts" in card(back)), "zpět na výchozí = pole pryč");
  assert.deepEqual(partsOf(undefined, "clock"), [BODY, "battery"], "karta bez záznamu = výchozí");
  assert.equal(movePart(l, "prace", "clock", 0, 1), l, "karta na záložce není = beze změny");
  const s = setParts(l, "osobni", "clock", ["battery", BODY]);
  assert.deepEqual(pageOf(s, "osobni").cards.find((c) => c.id === "clock").parts, ["battery", BODY]);
  assert.ok(!pageOf(s, "prace").cards.some((c) => c.parts), "jiná záložka nedotčená");
  assert.ok(!sameLayout(l, a), "pořadí částí je součást rozložení");
  // karta s pořadím se přesune na jinou záložku i s ním
  const m = moveToPage(a, "prace", "claude", "osobni");
  assert.deepEqual(pageOf(m, "osobni").cards.at(-1).parts, [BODY, "claudeCosts", "claudeLimits"]);
});

test("karta přetažená na „+“: nová záložka s kartou, jméno se přejmenuje potom", () => {
  const l = fullLayout();
  const r = moveToNewPage(l, "prace", "claude", "Nová záložka");
  assert.ok(r, "vznikla");
  assert.deepEqual(pageOf(r.layout, r.id).cards.map((c) => c.id), ["claude"]);
  assert.equal(pageOf(r.layout, r.id).name, "Nová záložka");
  assert.ok(!ids(r.layout, "prace").includes("claude"), "ze zdroje pryč");
  assert.equal(r.id, "nova-zalozka");
  // druhá záložka stejného jména dostane jiné id
  const r2 = moveToNewPage(r.layout, "prace", "repos", "Nová záložka");
  assert.equal(r2.id, "nova-zalozka-2");
  // plno záložek = nic, karta zůstane
  let full = l;
  while (full.pages.length < MAX_PAGES) full = addPage(full, `Z${full.pages.length}`).layout;
  assert.equal(moveToNewPage(full, "prace", "claude", "Nová záložka"), null);
  assert.equal(moveToNewPage(l, "prace", "neexistuje", "Nová záložka"), null, "karta tam není");
});
