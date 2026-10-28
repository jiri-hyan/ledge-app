// Guard Studia (kolo 24): 8 částí v pořadí anatomie, každý blok (dnešní panel / krok Vzhledu) právě v jedné
// otázce, KAŽDÁ dnešní volba (`labels.ts` FIELDS, prefix `placement.ts` PLACEMENT, ADVANCED_ANCHORS) má
// právě jednu část a blok, který ji opravdu vykresluje; skoky odjinud (staré stránky, aliasy, kotvy);
// počty „upraveno“ podle části.
//   node --experimental-strip-types --test src/studio/parts.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { PARTS, PART_IDS, PAGE_PART, blockOf, modifiedByPart, partOfBlock, resolveStudioTarget, stepPart } from "./parts.ts";
import { ADVANCED_ANCHORS, PLACEMENT } from "../settings/placement.ts";
import { ANCHOR_STEP } from "../settings/appearanceModel.ts";
import { PANEL_ALIASES, PANEL_META } from "../settings/registry.ts";

const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");
const FIELDS = Object.fromEntries([...read("../settings/labels.ts").matchAll(/^\s+"([\w.]+)": \{ panel: "(\w+)"/gm)].map((m) => [m[1], m[2]]));
const fp = (id) => FIELDS[id] ?? null;

test("8 částí v pořadí, jak se s Ledge potkáš", () => {
  assert.deepEqual(PART_IDS, ["pruzek", "otevirani", "vzhled", "obsah", "claude", "projekty", "poznamky", "aplikace"]);
  assert.deepEqual(PARTS.map((p) => p.n), [1, 2, 3, 4, 5, 6, 7, 8]);
  for (const p of PARTS) {
    assert.ok(p.questions.length >= 1 && p.questions.length <= 4, p.id);
    assert.ok(p.questions.some((q) => q.primary), `${p.id}: průvodce potřebuje aspoň jednu hlavní otázku`);
  }
});

test("každý blok v právě jedné otázce; bloky pokrývají všechny stránky kromě Doporučených", () => {
  const blocks = PARTS.flatMap((p) => p.questions.map((q) => q.block));
  assert.equal(new Set(blocks).size, blocks.length, "blok dvakrát");
  const pages = PANEL_META.map((p) => p.id).filter((id) => id !== "doporucene" && id !== "vzhled");
  for (const pg of pages) assert.ok(blocks.includes(pg), `stránka ${pg} nemá blok`);
  for (const b of ["strip", "hover", "look", "motion"]) assert.ok(blocks.includes(b), b);
});

test("každá volba z FIELDS: právě jedna část, blok = panel volby (u Vzhledu její krok)", () => {
  const ids = Object.keys(FIELDS);
  assert.ok(ids.length > 120, `${ids.length}`);
  const step = { closed: "strip", hover: "hover", open: "look", motion: "motion" };
  for (const id of ids) {
    const b = blockOf(id, fp);
    const want = FIELDS[id] === "vzhled" ? step[ANCHOR_STEP[id]] : FIELDS[id];
    assert.equal(b, want, id);
    assert.ok(partOfBlock(b), `${id} bez části`);
    assert.equal(resolveStudioTarget(`f-${id}`, PANEL_ALIASES, fp)?.part, partOfBlock(b), id);
  }
});

test("každý prefix PLACEMENT a ADVANCED_ANCHORS má část", () => {
  for (const [k, page] of PLACEMENT) {
    const b = blockOf(k, fp);
    assert.ok(b && partOfBlock(b), `${k} (${page})`);
  }
  for (const a of ADVANCED_ANCHORS) assert.ok(resolveStudioTarget(`f-${a}`, PANEL_ALIASES, fp)?.part, a);
});

test("rozdělení dnešního Vzhledu: proužek, otevírání, vzhled šuplíku", () => {
  const part = (k) => partOfBlock(blockOf(k, fp));
  const cases = [
    ["strip.look", "pruzek"], ["strip.opacity", "pruzek"], ["timer.strip", "pruzek"], ["appearance.strip", "pruzek"], ["appearance.strip.color", "pruzek"],
    ["drawer.hover", "otevirani"], ["drawer.hoverExpandMs", "otevirani"], ["drawer.trigger", "otevirani"], ["hotkeys", "otevirani"], ["palette", "otevirani"],
    ["appearance.style", "vzhled"], ["appearance.font", "vzhled"], ["appearance.motion", "vzhled"], ["drawer.width", "vzhled"], ["drawer.glass", "vzhled"],
    ["layout", "obsah"], ["modes", "obsah"], ["personal.weather.lat", "obsah"], ["accounts", "obsah"],
    ["claude.port", "claude"], ["mcp", "claude"], ["smart.approvalSound", "claude"],
    ["projects.2.port", "projekty"], ["scripts", "projekty"], ["personal.links", "projekty"],
    ["notes.sync.remote", "poznamky"], ["desk", "poznamky"],
    ["ui.lang", "aplikace"], ["autostart", "aplikace"], ["drawer.keepWarmS", "aplikace"], ["update.repo", "aplikace"],
    ["neznamy.klic", null], [null, null],
  ];
  for (const [k, want] of cases) assert.equal(part(k), want, String(k));
});

test("skoky odjinud: část, stará stránka, alias, kotva, domov, průvodce", () => {
  const r = (x) => resolveStudioTarget(x, PANEL_ALIASES, fp);
  const cases = [
    ["claude", { part: "claude", anchor: null, guide: false }],
    ["#obsah", { part: "obsah", anchor: null, guide: false }],
    ["osobni", { part: "obsah", anchor: null, guide: false }],
    ["chovani", { part: "otevirani", anchor: null, guide: false }],
    ["zkratky", { part: "otevirani", anchor: null, guide: false }],
    ["widgety", { part: "poznamky", anchor: null, guide: false }],
    ["doporucene", { part: "home", anchor: null, guide: false }],
    ["home", { part: "home", anchor: null, guide: false }],
    ["guide", { part: "pruzek", anchor: null, guide: true }],
    ["f-strip.look", { part: "pruzek", anchor: "strip.look", guide: false }],
    ["f-accounts.microsoft", { part: "obsah", anchor: "accounts.microsoft", guide: false }],
    ["f-projects.add", { part: "projekty", anchor: "projects.add", guide: false }],
    ["f-autostart", { part: "aplikace", anchor: "autostart", guide: false }],
    ["nesmysl", null], ["", null], [null, null], ["f-nic.takoveho", null],
  ];
  for (const [x, want] of cases) assert.deepEqual(r(x), want, String(x));
  for (const page of Object.keys(PAGE_PART)) assert.ok(PANEL_META.some((m) => m.id === page), page);
  for (const m of PANEL_META) assert.ok(PAGE_PART[m.id], `stránka ${m.id} nevede nikam`);
});

test("upraveno proti výchozímu podle části a pořadí částí", () => {
  const m = modifiedByPart(["strip.look", "appearance.strip.size", "appearance.theme", "projects", "neznamy", "personal.weather"], fp);
  assert.equal(m.pruzek, 2);
  assert.equal(m.vzhled, 1);
  assert.equal(m.projekty, 1);
  assert.equal(m.obsah, 1);
  assert.equal(m.claude, 0);
  assert.equal(stepPart("pruzek", 1), "otevirani");
  assert.equal(stepPart("pruzek", -1), null);
  assert.equal(stepPart("aplikace", 1), null);
});

test("průvodce: postup z úložiště a hotové části", async () => {
  const { parseGuide, withDone } = await import("./parts.ts");
  const cases = [[null, []], ["", []], ["{", []], ["{}", []], ['["claude","pruzek","nic","claude"]', ["pruzek", "claude"]], ["[1,2]", []]];
  for (const [raw, want] of cases) assert.deepEqual(parseGuide(raw), want, String(raw));
  assert.deepEqual(withDone([], "obsah"), ["obsah"]);
  assert.deepEqual(withDone(["obsah"], "pruzek"), ["pruzek", "obsah"]);
  assert.deepEqual(withDone(["pruzek", "obsah"], "obsah", false), ["pruzek"]);
});

test("viditelnost a kroky průvodce jako data (role a moduly příštího kola)", async () => {
  const { PARTS, visibleParts, visibleQuestions, guideSteps, guideStep } = await import("./parts.ts");
  assert.equal(visibleParts({}).length, 8, "dnes vidět vše");
  assert.deepEqual(guideSteps({}).slice(0, 2), ["pruzek", "otevirani"]);
  const steps = guideSteps({}, ["role"]);
  assert.deepEqual(steps.slice(0, 2), ["role", "pruzek"]);
  assert.equal(guideStep(steps, "role", 1), "pruzek");
  assert.equal(guideStep(steps, "aplikace", 1), null);
  assert.equal(guideStep(steps, "nic", 1), null);
  const hidden = { ...PARTS[4], visible: (c) => !!c.claudeOn };
  assert.equal([hidden].filter((p) => !p.visible || p.visible({})).length, 0);
  assert.equal(visibleQuestions({ ...PARTS[1], questions: [{ ...PARTS[1].questions[0], visible: () => false }, PARTS[1].questions[1]] }, {}).length, 1);
});

test("hlavní vrstva bez žargonu (port, hook, repo, MCP patří do Doladit)", () => {
  for (const p of PARTS) {
    for (const x of [p.name, p.line, ...p.questions.flatMap((q) => [q.q, q.sub])]) {
      assert.doesNotMatch(x[1], /(port|hook|hooky|repo|MCP|JSON)/i, x[0]);
    }
  }
});
