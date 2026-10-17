// Tabulkové testy viditelnosti karet a sekcí (`sections.ts`): výchozí stav, starý snapshot bez nových
// polí, změna jediného pole configu, co stránka ukáže, třída mřížky, úplnost tabulky pro Nastavení.
//   node --experimental-strip-types --test src/cards/sections.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  CLAUDE_PARTS, SECTION_GROUPS, SECTION_KEYS, allSectionsOn, gridClass, hiddenSections, isPart, sectionItem, sectionOn, visibleCards, withSection,
} from "./sections.ts";

test("výchozí: všechno zapnuté, chybějící pole = zapnuto", () => {
  const all = allSectionsOn();
  assert.deepEqual(Object.keys(all), [...SECTION_KEYS]);
  assert.ok(Object.values(all).every((v) => v === true));
  const cases = [
    // [sections, klíč, zapnuto]
    [undefined, "repos", true],
    [null, "weather", true],
    [{}, "claudeCosts", true],
    // starý snapshot / config jen se 4 poli
    [{ projects: true, running: false, claude: true, notes: true }, "running", false],
    [{ projects: true, running: false, claude: true, notes: true }, "shelf", true],
    [{ repos: false }, "repos", false],
    // jen výslovné false skrývá
    [{ clock: 0 }, "clock", true],
    [{ clock: null }, "clock", true],
  ];
  for (const [sec, k, want] of cases) assert.equal(sectionOn(sec, k), want, `${JSON.stringify(sec)} ${k}`);
  assert.deepEqual(hiddenSections({ repos: false, weather: false, links: true }), ["repos", "weather"]);
  assert.deepEqual(hiddenSections(undefined), []);
});

test("klíče se shodují s Rustem (`config::Sections` v config.rs, camelCase)", () => {
  const src = readFileSync(new URL("../../src-tauri/src/config.rs", import.meta.url), "utf8");
  const body = src.slice(src.indexOf("sections! {"), src.indexOf("}", src.indexOf("sections! {")));
  const rust = [...body.matchAll(/^\s*([a-z_]+),\s*$/gm)].map((m) => m[1].replace(/_([a-z])/g, (_, c) => c.toUpperCase()));
  assert.deepEqual(rust, [...SECTION_KEYS]);
});

test("withSection mění jen jedno pole a nesahá na vstup", () => {
  const cfg = { projects: [{ id: "a" }], drawer: { width: 900, sections: { projects: true, running: false } }, ui: { lang: "cs" } };
  const before = JSON.stringify(cfg);
  const out = withSection(cfg, "repos", false);
  assert.equal(JSON.stringify(cfg), before);
  assert.deepEqual(out, { projects: [{ id: "a" }], drawer: { width: 900, sections: { projects: true, running: false, repos: false } }, ui: { lang: "cs" } });
  assert.equal(out.projects, cfg.projects);
  // config bez drawer / sections
  assert.deepEqual(withSection({}, "clock", false), { drawer: { sections: { clock: false } } });
  assert.deepEqual(withSection({ drawer: { trigger: "edge" } }, "running", true), { drawer: { trigger: "edge", sections: { running: true } } });
});

const facts = (over = {}) => ({ mode: "prace", sec: {}, procs: 0, notesEnabled: true, actions: 0, shelf: false, ...over });

test("co stránka ukáže", () => {
  const off = (...ks) => Object.fromEntries(ks.map((k) => [k, false]));
  const cases = [
    // Práce: Běží bez procesů a akce/přihrádka bez obsahu se schovají samy
    [facts(), ["projects", "claude", "repos", "notes"]],
    [facts({ procs: 2, actions: 3, shelf: true }), ["projects", "actions", "shelf", "running", "claude", "repos", "notes"]],
    [facts({ sec: off("repos", "claude") }), ["projects", "notes"]],
    [facts({ notesEnabled: false }), ["projects", "claude", "repos"]],
    [facts({ sec: off("projects", "claude", "repos", "notes") }), []],
    [facts({ sec: off("projects", "claude", "repos", "notes"), procs: 1 }), ["running"]],
    // Osobní: prázdné akce zvou k přidání (karta je vidět i bez akcí)
    [facts({ mode: "osobni" }), ["clock", "timer", "weather", "links", "actions", "notes"]],
    [facts({ mode: "osobni", sec: off("clock", "weather"), shelf: true }), ["timer", "links", "actions", "shelf", "notes"]],
    [facts({ mode: "osobni", sec: off("clock", "timer", "weather", "links", "actions", "notes", "shelf"), shelf: true }), []],
    // Právě hraje: jen s daty (skladba nebo hlasitost); se skrytou skladbou je to karta „Zvuk“
    [facts({ mode: "osobni", media: true }), ["clock", "timer", "weather", "links", "nowPlaying", "actions", "notes"]],
    [facts({ mode: "osobni", media: true, sec: off("nowPlaying") }), ["clock", "timer", "weather", "links", "sound", "actions", "notes"]],
    [facts({ mode: "osobni", media: true, sec: off("nowPlaying", "sound") }), ["clock", "timer", "weather", "links", "actions", "notes"]],
    // sekce karty Claude nejsou karty stránky
    [facts({ sec: off("claudeLimits", "claudeCosts", "claudeGit", "claudeSubagents") }), ["projects", "claude", "repos", "notes"]],
  ];
  for (const [f, want] of cases) assert.deepEqual(visibleCards(f), want, JSON.stringify(f));
});

test("třída mřížky podle úzkých karet (Poznámky jsou přes celou šířku)", () => {
  const cases = [
    [[], ""],
    [["notes"], ""],
    [["repos"], "one"],
    [["projects", "repos", "notes"], "one"],
    [["claude", "repos"], "two"],
    [["running", "claude", "repos"], ""],
  ];
  for (const [vis, want] of cases) assert.equal(gridClass(vis), want, JSON.stringify(vis));
});

test("tabulka pro Nastavení: každý klíč právě jednou, sekce Claude jako části", () => {
  const keys = SECTION_GROUPS.flatMap((g) => g.items.map((i) => i.k));
  assert.deepEqual([...keys].sort(), [...SECTION_KEYS].sort());
  assert.equal(new Set(keys).size, keys.length);
  for (const it of SECTION_GROUPS.flatMap((g) => g.items)) {
    assert.ok(it.key.startsWith("settings.drawer.sec.") && it.cs && it.hint[0].startsWith(it.key) && it.hint[1], it.k);
    assert.equal(sectionItem(it.k), it);
  }
  assert.deepEqual(SECTION_GROUPS.find((g) => g.id === "claude").items.map((i) => i.k), [...CLAUDE_PARTS]);
  assert.ok(isPart("claudeGit") && !isPart("claude") && !isPart("repos"));
  assert.ok(isPart("sound") && isPart("battery") && !isPart("nowPlaying"));
});
