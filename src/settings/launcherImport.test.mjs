// @vitest-environment node
// Tabulkové testy importu katalogu Launcheru (`launcherImport.ts`). Smyšlená data (žádná reálná repa).
//   node --experimental-strip-types --test src/settings/launcherImport.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { commandLines, commandsOf, folderId, folderName, importLauncherCatalog, projectOf, withVerdicts } from "./launcherImport.ts";

/** zpětné lomítko (cesty Windows) */
const BS = String.fromCharCode(92);
const catalog = {
  families: { studio: { label: "Studio", mode: "work" }, hry: { label: "Hry", mode: "fun" } },
  items: [
    { id: "orbit-web", name: "Orbit Web", family: "studio", type: "web-app", state: "active", path: "C:/Work/orbit-web", remote: "https://example.com/acme/orbit-web", web: { run: "npm run dev", port: 5180 }, accent: "#3B82F6" },
    { id: "pixel-quest", name: "Pixel Quest", family: "hry", type: "tauri-phaser", state: "active", path: "C:/Games/pixel-quest", actions: [{ id: "play", label: "Hrát", exec: "build/pixel-quest.exe", run: "npm run build", args: ["--windowed"] }] },
    { id: "old-tool", name: "Old Tool", family: "hry", type: "electron-desktop", state: "disabled", path: "C:/Games/old-tool", actions: [{ id: "x", exec: "dist/old.exe" }] },
    { id: "cli-kit", name: "CLI Kit", family: "studio", type: "cli-tool", commands: ["kit --help"] },
    { name: "Bez id", web: { run: "npm start" } },
  ],
};
const cfg = () => ({
  projects: [{ id: "harbor", name: "Harbor", kind: "dev", run: "npm run dev", path: ["C:", "Work", "Harbor", ""].join(BS) }],
  layout: { pages: [{ id: "prace", name: "", cards: [{ id: "notes", size: "m" }, { id: "projects", size: "xl" }] }, { id: "osobni", name: "", cards: [] }] },
});

test("položka katalogu → projekt Ledge", () => {
  assert.deepEqual(projectOf(catalog.items[0]), { id: "orbit-web", name: "Orbit Web", kind: "dev", path: "C:/Work/orbit-web", remote: "https://example.com/acme/orbit-web", run: "npm run dev", port: 5180, accent: "#3b82f6" });
  assert.deepEqual(projectOf(catalog.items[1]), { id: "pixel-quest", name: "Pixel Quest", kind: "app", path: "C:/Games/pixel-quest", passPort: false, openBrowser: false, exe: "build/pixel-quest.exe", run: "npm run build", args: ["--windowed"] });
  assert.equal(projectOf(catalog.items[2]).enabled, false, "vypnutá položka = vypnutý projekt");
  const rows = [
    [catalog.items[3], null, "CLI bez příkazu"],
    [catalog.items[4], null, "bez id"],
    [{ id: "x", web: { run: "" }, actions: [{ run: "build" }] }, null, "akce bez exe"],
    [{ id: "x", type: "web-app", actions: [{ exec: "a.exe" }] }, "app", "web-app bez web.run není dev (validace by chtěla run)"],
    [{ id: "x", web: { run: "vite", port: 70000 }, accent: "modrá" }, "dev", "špatný port a akcent se nepřenesou"],
  ];
  for (const [item, kind, why] of rows) assert.equal(projectOf(item)?.kind ?? null, kind, why);
  const odd = projectOf(rows[4][0]);
  assert.equal("port" in odd || "accent" in odd, false);
});

test("import: přidá nové, přeskočí duplicity a položky bez příkazu, vstup nemění", () => {
  const before = cfg();
  const { nextCfg, added, skipped } = importLauncherCatalog(catalog, before);
  assert.deepEqual(added, ["Orbit Web", "Pixel Quest", "Old Tool"]);
  assert.deepEqual(skipped, ["CLI Kit", "Bez id"]);
  const split = importLauncherCatalog(catalog, before);
  assert.equal(split.duplicates.length + split.noCommand.length, split.skipped.length);
  assert.equal(before.projects.length, 1, "původní config beze změny");
  // znovu = nic nového (id, složka i remote se porovnají)
  const again = importLauncherCatalog(catalog, nextCfg);
  assert.deepEqual(again.added, []);
  const dupPath = importLauncherCatalog({ items: [{ id: "harbor-2", path: "c:/work/harbor", web: { run: "npm run dev" } }] }, before);
  assert.deepEqual(dupPath.added, [], "stejná složka jinými lomítky");
});

test("import: rodiny jako složky na záložce s kartou Projekty, názvy z katalogu", () => {
  const { nextCfg } = importLauncherCatalog(catalog, cfg());
  const card = nextCfg.layout.pages[0].cards.find((c) => c.id === "projects");
  assert.deepEqual(card.tiles.folders, [
    { id: "f-studio", name: "Studio", items: ["orbit-web"] },
    { id: "f-hry", name: "Hry", items: ["pixel-quest", "old-tool"] },
  ]);
  // bez karty Projekty: přidá ji na první záložku; bez rozložení nic nevymýšlí
  const noCard = importLauncherCatalog(catalog, { projects: [], layout: { pages: [{ id: "prace", name: "", cards: [{ id: "notes", size: "m" }] }] } }).nextCfg;
  assert.equal(noCard.layout.pages[0].cards[0].id, "projects");
  assert.equal(importLauncherCatalog(catalog, { projects: [] }).nextCfg.layout, undefined);
});

test("id a název složky", () => {
  const rows = [["studio", "f-studio"], ["Hry a zábava", "f-hry-a-zabava"], ["", "f-slozka"], ["x".repeat(40), "f-" + "x".repeat(22)], ["a---b", "f-a-b"]];
  for (const [fam, want] of rows) assert.equal(folderId(fam), want, fam);
  assert.equal(folderName("studio", catalog.families), "Studio");
  assert.equal(folderName("nastroje", catalog.families), "Nastroje", "bez popisku = id s velkým písmenem");
});

test("pojistka: příkazy jdou ke kontrole, co neprojde, se vypne a v potvrzení má důvod", () => {
  const evil = { items: [...catalog.items, { id: "zlo", name: "Zlo", web: { run: "powershell -enc SQBFAFgA" } }] };
  const { nextCfg, addedIds } = importLauncherCatalog(evil, cfg());
  assert.deepEqual(commandsOf(nextCfg, addedIds).map((c) => c.id), ["orbit-web", "pixel-quest", "old-tool", "zlo"]);
  assert.deepEqual(commandsOf(nextCfg, ["pixel-quest"])[0], { id: "pixel-quest", path: "C:/Games/pixel-quest", run: "npm run build", exe: "build/pixel-quest.exe", args: ["--windowed"] });
  const verdicts = [{ id: "orbit-web", problem: null }, { id: "pixel-quest", problem: null }, { id: "old-tool", problem: null }, { id: "zlo", problem: "příkaz neprošel" }];
  const safe = withVerdicts(nextCfg, addedIds, verdicts);
  const en = (id) => safe.projects.find((p) => p.id === id).enabled;
  assert.deepEqual([en("orbit-web"), en("zlo"), en("harbor")], [undefined, false, undefined], "vypne jen problémové, existující nesahá");
  assert.equal(withVerdicts(nextCfg, addedIds, []).projects.find((p) => p.id === "orbit-web").enabled, false, "bez verdiktu jádra = vypnuto");
  const lines = commandLines(safe, addedIds, verdicts);
  assert.deepEqual(lines[1], { name: "Pixel Quest", command: "npm run build → build/pixel-quest.exe --windowed", problem: null });
  assert.equal(lines[3].problem, "příkaz neprošel");
});

test("import: přeskočené se dělí na duplicity a položky bez příkazu", () => {
  const before = cfg();
  const { nextCfg, skipped, noCommand, duplicates } = importLauncherCatalog(catalog, before);
  assert.deepEqual(noCommand, skipped);
  assert.deepEqual(duplicates, []);
  const again = importLauncherCatalog(catalog, nextCfg);
  assert.ok(again.duplicates.length >= 3, "co už je v projektech, je duplicita");
  assert.equal(again.duplicates.length + again.noCommand.length, again.skipped.length);
});
