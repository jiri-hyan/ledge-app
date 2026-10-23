// @vitest-environment node
// Tabulkové testy zástupců projektů (`model.ts`): co a odkud (tooltip), co udělá klik podle místa (víc složek =
// výběr, chybějící složka = oprava), předvolba ve výběru složek, zápis `path` / `run` do configu, selhání.
//   node --experimental-strip-types --test src/projects/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { tipState, clickOf, commitText, failInfo, freshest, healthText, healthTone, isLive, liveProcs, placeOf, placeProblem, preselect, samePath, tileHealth, tileTip, PROJECT_NOTES, notesHidden, projectNotesQuery, whatOf, whereOf, withProjectFields } from "./model.ts";

const NOW = 1_792_250_979_530;
const pr = (o = {}) => ({ id: "orbit", name: "Orbit", kind: "dev", accent: "#2563eb", monogram: "O", logoKey: null, path: "C:\\Work\\orbit-v2", pathOk: true, clonable: false, cloning: false, github: null, links: [], git: null, run: "npm run dev:staging", port: 5210, remote: "acme/orbit", app: null, place: { kind: "ok" }, ...o });
const pc = (phase, o = {}) => ({ id: "orbit", phase, pid: 1, port: 5210, url: null, startedAt: NOW, readyAt: null, exitCode: null, message: null, lastLine: null, errors: 0, ...o });

test("místo: starší jádro bez place → podle pathOk", () => {
  assert.deepEqual(placeOf({ pathOk: true }), { kind: "ok" });
  assert.deepEqual(placeOf({ pathOk: false }), { kind: "none" });
  assert.deepEqual(placeOf({ pathOk: false, place: { kind: "choose", count: 2 } }), { kind: "choose", count: 2 });
});

test("klik na dlaždici: víc složek = výběr, chybí složka = oprava, běží = otevřít, jinak spustit", () => {
  const cases = [
    ["jasné místo", pr(), null, "launch"],
    ["víc složek", pr({ place: { kind: "choose", count: 2 }, pathOk: false, path: null }), null, "choose"],
    ["path neexistuje", pr({ place: { kind: "missing", path: "C:\\Old\\orbit" }, pathOk: false }), null, "fix"],
    ["nic na disku (dev)", pr({ place: { kind: "none" }, pathOk: false, path: null }), null, "fix"],
    ["aplikace podle appName bez složky", pr({ kind: "app", app: "Figma", place: { kind: "none" }, pathOk: false, path: null }), null, "launch"],
    ["běží", pr(), pc("ready"), "open"],
    ["startuje", pr({ place: { kind: "choose", count: 2 } }), pc("starting"), "open"],
    ["selhal → znovu podle místa", pr(), pc("failed"), "launch"],
  ];
  for (const [name, p, proc, want] of cases) assert.equal(clickOf(p, proc), want, name);
});

test("co a odkud: tooltip ukáže příkaz, složku a port; běžící proces ví složku nejlíp", () => {
  assert.equal(whatOf(pr()), "npm run dev:staging");
  assert.equal(whatOf(pr({ run: "  " })), null);
  assert.equal(whatOf(pr({ kind: "app", app: "C:\\Apps\\x.exe", run: "npm run build" })), "C:\\Apps\\x.exe");
  assert.equal(tileTip(pr()), "Klik spustí: npm run dev:staging\nSložka: C:\\Work\\orbit-v2\nPort: 5210");
  assert.equal(
    tileTip(pr(), pc("ready", { cwd: "C:\\Work\\orbit", port: 5211 })),
    "Klik spustí: npm run dev:staging\nSložka: C:\\Work\\orbit\nPort: 5211\nBěží — ■ v akcích zástupce zastaví",
  );
  assert.match(tileTip(pr(), pc("failed", { message: "Server nenaběhl do 90 s." })), /Nenaběhl: Server nenaběhl do 90 s\.$/);
  assert.match(tileTip(pr({ run: null })), /^Chybí příkaz ke spuštění/);
  assert.equal(whereOf(pr({ place: { kind: "missing", path: "C:\\Old\\orbit" } })), "C:\\Old\\orbit — složka neexistuje");
  assert.equal(whereOf(pr({ place: { kind: "choose", count: 2 }, path: null })), "2 složky s tímhle repem — vyber jednu");
  assert.equal(whereOf(pr({ place: { kind: "choose", count: 5 }, path: null })), "5 složek s tímhle repem — vyber jednu");
  assert.equal(whereOf(pr({ place: { kind: "none" }, path: null })), "složka nenalezena");
});

test("chyba místa: srozumitelně, jen když je co opravit", () => {
  assert.equal(placeProblem(pr()), null);
  assert.equal(placeProblem(pr({ place: { kind: "missing", path: "C:\\Old\\orbit" } })), "Složka C:\\Old\\orbit neexistuje.");
  assert.equal(placeProblem(pr({ place: { kind: "choose", count: 2 } })), "Našel jsem 2 složky s tímto repem. Vyber, ze které se má spouštět.");
  assert.equal(placeProblem(pr({ place: { kind: "none" } })), "Složka projektu na tomhle PC není.");
  assert.equal(placeProblem(pr({ kind: "app", place: { kind: "none" } })), null);
});

test("výběr složky: předvybraná aktuální, jinak první (= nejčerstvější z jádra); označení nejčerstvější", () => {
  const items = [
    { path: "C:\\Work\\orbit-v2", lastCommit: 1_790_000_000, worktree: true },
    { path: "C:\\Work\\orbit", lastCommit: 1_787_000_000, worktree: false },
  ];
  assert.equal(preselect(items, null), "C:\\Work\\orbit-v2");
  assert.equal(preselect(items, "c:/work/orbit/"), "C:\\Work\\orbit");
  assert.equal(preselect(items, "C:\\Jinde"), "C:\\Work\\orbit-v2");
  assert.equal(preselect([], null), null);
  assert.equal(freshest(items), "C:\\Work\\orbit-v2");
  assert.equal(freshest(items.slice(0, 1)), null, "jedna složka = nic neoznačovat");
  assert.equal(freshest([{ path: "C:\\a", lastCommit: null, worktree: false }, { path: "C:\\b", lastCommit: null, worktree: false }]), null);
  assert.ok(samePath("C:\\Work\\orbit\\", "c:/work/orbit"));
  assert.equal(commitText(null, NOW), "datum commitu nejde zjistit");
  assert.match(commitText(Math.floor(NOW / 1000) - 3600, NOW), /^poslední commit /);
});

test("zápis do configu: jen path / run toho projektu, prázdné = smazat klíč, ostatní beze změny", () => {
  const cfg = { roots: ["C:\\Work"], projects: [{ id: "orbit", name: "Orbit", remote: "acme/orbit", run: "npm run dev" }, { id: "beacon", name: "Beacon", path: "C:\\Work\\beacon" }] };
  const a = withProjectFields(cfg, "orbit", { path: "  C:\\Work\\orbit-v2 " });
  assert.deepEqual(a.projects[0], { id: "orbit", name: "Orbit", remote: "acme/orbit", run: "npm run dev", path: "C:\\Work\\orbit-v2" });
  assert.equal(a.projects[1], cfg.projects[1], "jiný projekt se nemění");
  assert.deepEqual(cfg.projects[0].path, undefined, "vstup se nemění");
  const b = withProjectFields(a, "orbit", { path: null, run: " pnpm dev " });
  assert.deepEqual(b.projects[0], { id: "orbit", name: "Orbit", remote: "acme/orbit", run: "pnpm dev" });
  assert.deepEqual(withProjectFields(b, "orbit", { run: "" }).projects[0], { id: "orbit", name: "Orbit", remote: "acme/orbit" });
  assert.equal(withProjectFields(cfg, "neni", { path: "C:\\x" }), cfg);
  assert.deepEqual(withProjectFields({}, "orbit", { path: "C:\\x" }), {});
});

test("selhání: zpráva + poslední řádky; běžící a čistě skončený nejsou chyba", () => {
  assert.equal(failInfo(undefined), null);
  assert.equal(failInfo(pc("ready")), null);
  assert.equal(failInfo(pc("exited", { exitCode: 0 })), null);
  assert.deepEqual(failInfo(pc("failed", { message: "Server skončil dřív, než naběhl (kód 1).", tail: ["Error: Cannot find module 'x'"] })), { message: "Server skončil dřív, než naběhl (kód 1).", tail: ["Error: Cannot find module 'x'"] });
  assert.deepEqual(failInfo(pc("failed", { lastLine: "boom" })), { message: "Server se nespustil.", tail: ["boom"] });
  assert.deepEqual(failInfo(pc("exited", { exitCode: 1, message: "Server skončil (kód 1)." })), { message: "Server skončil (kód 1).", tail: [] });
  assert.ok(isLive("ready") && isLive("preparing") && !isLive("failed") && !isLive(undefined));
  assert.deepEqual(liveProcs([pc("ready"), pc("failed"), pc("starting")]).map((x) => x.phase), ["ready", "starting"]);
});

test("náhled po najetí: stav jako pilulka (běží s portem, startuje, nenaběhl, vyber složku, neběží)", () => {
  const cases = [
    ["běží", pr(), pc("ready", { port: 5210 }), { tone: "g", text: "běží :5210", detail: null }],
    ["aplikace běží", pr({ kind: "app" }), pc("running", { port: null }), { tone: "g", text: "běží", detail: null }],
    ["startuje", pr(), pc("starting"), { tone: "a", text: "startuje…", detail: null }],
    ["nenaběhl", pr(), pc("failed", { message: "Server nenaběhl do 90 s." }), { tone: "r", text: "nenaběhl", detail: "Server nenaběhl do 90 s." }],
    ["víc složek", pr({ place: { kind: "choose", count: 2 } }), null, { tone: "a", text: "vyber složku", detail: "Našel jsem 2 složky s tímto repem. Vyber, ze které se má spouštět." }],
    ["složka chybí", pr({ place: { kind: "missing", path: "C:\Old" } }), null, { tone: "r", text: "složka chybí", detail: "Složka C:\Old neexistuje." }],
    ["neběží", pr(), null, { tone: "n", text: "neběží", detail: null }],
  ];
  for (const [name, p, proc, want] of cases) assert.deepEqual(tipState(p, proc), want, name);
});

test("zdraví zástupce: dirty / ahead z gitu, Claude jen při pracující session téhož projektu", () => {
  const git = (dirty, ahead) => ({ branch: "main", dirty, ahead, behind: 0 });
  const sess = (projectId, status, over = {}) => ({ projectId, status, ended: false, ...over });
  const cases = [
    ["čistý bez session", pr({ git: git(0, 0) }), [], null],
    ["bez gitu", pr({ git: null }), [], null],
    ["neuložené", pr({ git: git(3, 0) }), [], { dirty: 3, ahead: 0, claude: false }],
    ["nepushnuté", pr({ git: git(0, 2) }), [], { dirty: 0, ahead: 2, claude: false }],
    ["oboje", pr({ git: git(1, 1) }), [], { dirty: 1, ahead: 1, claude: false }],
    ["Claude pracuje", pr({ git: git(0, 0) }), [sess("orbit", "working")], { dirty: 0, ahead: 0, claude: true }],
    ["cizí projekt", pr(), [sess("jiny", "working")], null],
    ["čeká / nečinná / ukončená", pr(), [sess("orbit", "waiting"), sess("orbit", "idle"), sess("orbit", "working", { ended: true })], null],
    ["bez projectId", pr(), [sess(null, "working")], null],
  ];
  for (const [name, p, sessions, want] of cases) assert.deepEqual(tileHealth(p, sessions), want, name);
  assert.deepEqual(["claude", "dirty", "ahead", "dirty"], [{ dirty: 1, ahead: 1, claude: true }, { dirty: 1, ahead: 1, claude: false }, { dirty: 0, ahead: 4, claude: false }, { dirty: 2, ahead: 0, claude: false }].map(healthTone));
  assert.equal(healthText({ dirty: 1, ahead: 0, claude: false }), "1 neuložená změna");
  assert.equal(healthText({ dirty: 3, ahead: 2, claude: true }), "Claude tu pracuje · 3 neuložené změny · 2 nepushnuté commity");
  assert.equal(healthText({ dirty: 5, ahead: 1, claude: false }), "5 neuložených změn · 1 nepushnutý commit");
});

test("poznámky projektu v panelu: dotaz s filtrem projektu a stropem 3, zbytek jako +N", () => {
  assert.equal(PROJECT_NOTES, 3);
  assert.deepEqual(projectNotesQuery("orbit"), { project: "orbit", limit: 3, includeDone: false });
  const cases = [[0, 0, 0], [2, 2, 0], [3, 3, 0], [7, 3, 4], [undefined, 0, 0], [null, 2, 0], [1, 3, 0]];
  for (const [total, shown, want] of cases) assert.equal(notesHidden(total, shown), want, `${total}/${shown}`);
});
