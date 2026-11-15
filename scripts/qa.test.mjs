// QA skript (scripts/qa.mjs, scripts/qa/lib.mjs): čisté funkce bez GUI. Spuštění: node --experimental-strip-types --test scripts/qa.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import {
  rawKeys, field, hasHotkey, viewIs, checkField, summarize, exitCode, hollowScenarios, renderReport, stdoutSummary, parseArgs,
  settleStep, insideDir, descendants, userDataDir, dataDirVerdict,
  fixtureConfig, fixtureNotes, fixtureDesk, fixtureLimits, selectGroups, ALL_SCENARIOS, GROUP_SCENARIOS, wantsUpgrade, step, isNoise, PASS, FAIL, SKIP, PIN_NOTE, PASSTHROUGH_KEY,
} from "./qa/lib.mjs";

test("rawKeys: syrové překladové klíče ano, hostitelé, soubory, verze a běžný text ne", () => {
  const cases = [
    ["Zkratka pro vypnutí teď nefunguje (notes.pin.hintNoKey)", ["notes.pin.hintNoKey"]],
    ["cards.desk.kind.waiting.hint a settings.nav.vzhled", ["cards.desk.kind.waiting.hint", "settings.nav.vzhled"]],
    ["api.anthropic.com, github.com, config.v2.json, ledge.exe", []],
    ["Verze 0.16.0 z 14. 11. 2026, 5.3.1", []],
    ["Konec věty.Další věta.A další", []],
    ["https://example.org/a.b.c", []],
    ["mail jan@firma.example.cz", []],
    ["Ledge QA (ticho) — 37 % · 12 %", []],
    ["", []],
  ];
  for (const [text, want] of cases) assert.deepEqual(rawKeys(text), want, text);
  assert.deepEqual(rawKeys(null), []);
  assert.deepEqual(rawKeys("ab.cd.ef cards.xx.yy cards.xx.yy"), ["ab.cd.ef", "cards.xx.yy"], "duplicity se sloučí");
});

test("field: chybějící článek = undefined, hodnota false/0/null se nezaměňuje s chybějící", () => {
  const o = { qa: { pins: { open: 0, through: false }, n: null } };
  assert.equal(field(o, "qa.pins.open"), 0);
  assert.equal(field(o, "qa.pins.through"), false);
  assert.equal(field(o, "qa.n"), null);
  assert.equal(field(o, "qa.desk.open"), undefined);
  assert.equal(field(undefined, "a"), undefined);
});

test("hasHotkey: podle akce nebo kombinace, chybějící seznam = undefined", () => {
  const l = [{ action: "toggle", key: "Ctrl+Alt+Space" }, { action: "notePassthrough", key: PASSTHROUGH_KEY }];
  assert.equal(hasHotkey(l, "notePassthrough"), true);
  assert.equal(hasHotkey(l, "x", "ctrl+alt+shift+f11"), true, "kombinace bez ohledu na velikost písmen");
  assert.equal(hasHotkey(l, "note"), false);
  assert.equal(hasHotkey([], "notePassthrough", PASSTHROUGH_KEY), false);
  assert.equal(hasHotkey(undefined, "notePassthrough"), undefined);
});

test("viewIs: nezávislé na velikosti písmen", () => {
  assert.equal(viewIs({ drawer: { view: "Full" } }, "full"), true);
  assert.equal(viewIs({ drawer: { view: "hidden" } }, "full"), false);
  assert.equal(viewIs({}, "full"), false);
});

test("checkField: chybí pole → SKIP s důvodem, jinak PASS/FAIL podle predikátu", () => {
  const st = { qa: { pins: { open: 1 } } };
  const r = (path, ok) => checkField("s", "n", st, path, ok, "x");
  assert.equal(r("qa.pins.open", (v) => v === 1).status, PASS);
  assert.equal(r("qa.pins.open", (v) => v === 0).status, FAIL);
  assert.match(r("qa.pins.open", (v) => v === 0).evidence, /čekáno x/);
  const miss = r("qa.pins.through", () => true);
  assert.equal(miss.status, SKIP);
  assert.match(miss.evidence, /qa\.pins\.through chybí/);
});

test("verdikt: exitCode, souhrn, scénář jen se SKIP je prázdný", () => {
  const ok = [step("a", "1", PASS), step("a", "2", SKIP, "x"), step("b", "1", SKIP, "y")];
  assert.equal(exitCode(ok), 0);
  assert.deepEqual(summarize(ok), { PASS: 1, FAIL: 0, SKIP: 2 });
  assert.deepEqual(hollowScenarios(ok), ["b"]);
  assert.equal(exitCode([...ok, step("c", "1", FAIL, "z")]), 1);
  assert.equal(exitCode([]), 0);
});

test("renderReport: tabulka, souhrn, pipe v důkazu se escapuje, prázdný scénář a poznámky", () => {
  const steps = [step("supik", "karty", PASS, "5 karet"), step("kit", "data", FAIL, "a|b\nc"), step("hudba", "media", SKIP, "pole chybí")];
  const md = renderReport({ steps, binary: { path: "C:/x/ledge.exe", version: "0.16.0", sha: "sha256:abc" }, started: "2026-11-14T10:00:00Z", durationMs: 12_300, notes: ["pozn"], out: "C:/out" });
  assert.match(md, /^# Ledge QA — FAIL/);
  assert.match(md, /1 PASS · 1 FAIL · 1 SKIP/);
  assert.match(md, /\| supik \| karty \| PASS \| 5 karet \|/);
  assert.match(md, /a\\\|b c/);
  assert.match(md, /Bez jediného ověření \(jen SKIP\): hudba/);
  assert.match(md, /- pozn/);
  assert.match(md, /12\.3 s/);
  assert.match(renderReport({ steps: [step("a", "b", PASS)], started: "t", durationMs: 1 }), /^# Ledge QA — PASS/);
});

test("stdoutSummary: nejvýš 25 řádků, FAIL před SKIP", () => {
  const many = Array.from({ length: 60 }, (_, i) => step("s", `k${i}`, i === 50 ? FAIL : SKIP, "důvod"));
  const lines = stdoutSummary(many, "r.md").split("\n");
  assert.ok(lines.length <= 25);
  assert.match(lines[0], /^QA FAIL/);
  assert.match(lines[1], /^FAIL s\/k50/);
  assert.match(lines.at(-1), /^zpráva: r\.md/);
  assert.match(lines.at(-2), /dalších \d+ řádků/);
});

test("parseArgs: výchozí hodnoty, přepínače, zákaz portu 47800 a neznámého argumentu", () => {
  assert.deepEqual(parseArgs([]), { exe: null, oldExe: null, out: null, only: null, keep: false, list: false, apiPort: 47811, cdpPort: 47812 });
  const o = parseArgs(["--exe", "x.exe", "--old-exe", "old.exe", "--only", "supik, kit", "--keep", "--out", "o", "--api-port", "47820"]);
  assert.deepEqual([o.exe, o.oldExe, o.only, o.keep, o.out, o.apiPort], ["x.exe", "old.exe", ["supik", "kit"], true, "o", 47820]);
  assert.throws(() => parseArgs(["--api-port", "47800"]), /47800/);
  assert.throws(() => parseArgs(["--cdp-port", "47811"]), /různá/);
  assert.throws(() => parseArgs(["--foo"]), /neznámý/);
});

test("fixtureConfig: port ≠ 47800, bez autostartu, hover a proužek vypnuté, zkratka nekoliduje s psaním", () => {
  const c = fixtureConfig({ apiPort: 47811, hotkeys: true, notesRoot: "C:/t/notes" });
  assert.equal(c.autostart, false);
  assert.deepEqual([c.claude.port, c.notes.root, c.hotkeys.enabled, c.drawer.trigger], [47811, "C:/t/notes", true, undefined]);
  // trigger "off" je platný jen se zapnutou zkratkou; hover vlákno a proužek jsou v izolované instanci vypnuté jinak
  assert.equal(c.strip.enabled, false);
  assert.notEqual(fixtureConfig({ apiPort: 1, notesRoot: "x" }).drawer.trigger, "off");
  assert.deepEqual(c.strip.items, []);
  assert.match(c.hotkeys.notePassthrough, /^Ctrl\+Alt\+Shift\+F\d+$/);
  assert.equal(fixtureConfig({ apiPort: 1, notesRoot: "x" }).hotkeys.enabled, false);
});

// klíče fixture musí existovat v dokumentaci agentů (hlídá drift schématu, ne jen překlep)
test("fixtureConfig: každý klíč je v docs/agents/*.md", () => {
  const dir = new URL("../docs/agents/", import.meta.url);
  const docs = readdirSync(dir).filter((f) => f.endsWith(".md")).map((f) => readFileSync(new URL(f, dir), "utf8")).join("\n");
  const c = fixtureConfig({ apiPort: 47811, hotkeys: true, notesRoot: "x" });
  for (const [sec, v] of Object.entries(c)) {
    const keys = typeof v === "object" && !Array.isArray(v) ? Object.keys(v).map((k) => [sec, k]) : [[sec]];
    for (const [a, b] of keys) assert.ok(new RegExp(String.raw`\b${b ?? a}\b`).test(docs) && (!b || docs.includes(a)), `klíč ${a}${b ? "." + b : ""} není v docs/agents/*.md`);
  }
});

test("fixtureNotes: frontmatter s ISO časem s offsetem, úkol s termínem, lístek existuje", () => {
  const n = fixtureNotes(new Date("2026-11-14T10:00:00Z"));
  assert.ok(n[PIN_NOTE]);
  for (const body of Object.values(n)) assert.match(body, /^---\ncreated: \d{4}-\d\d-\d\dT\d\d:\d\d:\d\d[+-]\d\d:\d\d\n/);
  assert.ok(Object.values(n).some((b) => /kind: ukol/.test(b) && /remind: \d{4}-/.test(b) && /- \[ \]/.test(b)));
});

test("fixtureDesk / fixtureLimits: tvar podle docs a limits.rs", () => {
  const d = fixtureDesk();
  assert.equal(d.widgets.length, 1);
  assert.deepEqual(Object.keys(d.widgets[0]).sort(), ["compact", "h", "hideInGame", "id", "kind", "layer", "locked", "opacity", "through", "w", "x", "y"]);
  const l = fixtureLimits(1_000_000);
  assert.deepEqual([l.at, l.fiveHour.pct, l.sevenDay.pct], [1_000_000, 37, 12]);
  assert.ok(l.fiveHour.resetsAt > l.at);
});

test("selectGroups: výchozí = vše, --only zúží skupiny, kit a listky-l260 jsou aliasy, neznámé selže", () => {
  assert.deepEqual(selectGroups(null).flatMap((g) => g.scenarios), GROUP_SCENARIOS);
  assert.deepEqual(ALL_SCENARIOS, [...GROUP_SCENARIOS, "upgrade"]);
  assert.deepEqual(selectGroups(["upgrade"]), [], "upgrade není skupina instancí");
  assert.equal(wantsUpgrade(null), true);
  assert.equal(wantsUpgrade(["hudba"]), false);
  assert.equal(wantsUpgrade(["hudba", "upgrade"]), true);
  assert.deepEqual(selectGroups(["hudba"]).map((g) => [g.id, g.scenarios]), [["zaklad", ["hudba"]]]);
  assert.deepEqual(selectGroups(["kit"]).flatMap((g) => g.scenarios), ["kit-bez-dat", "kit-data"]);
  assert.deepEqual(selectGroups(["listky-l260"]).flatMap((g) => g.scenarios), ["listky-l260", "listky-l260-zkratky"]);
  assert.deepEqual(selectGroups(["propojeni"]).map((g) => [g.id, g.scenarios]), [["propojeni", ["propojeni"]]]);
  assert.throws(() => selectGroups(["nic"]), /neznámý scénář: nic/);
});

test("isNoise: favicon ano, skutečná chyba ne", () => {
  assert.equal(isNoise("Failed to load resource: favicon.ico"), true);
  assert.equal(isNoise("TypeError: x is undefined"), false);
});

test("settleStep: nedoběhlé animace = FAIL s důvodem, doběhlé PASS, bez snímku SKIP", () => {
  const ok = { total: 4, finite: 2, running: 0, forced: 0, visibility: "visible", ms: 300 };
  assert.equal(settleStep("s", ok).status, PASS);
  const bad = settleStep("s", { ...ok, running: 2, forced: 2, visibility: "hidden", ms: 3000 });
  assert.equal(bad.status, FAIL);
  assert.match(bad.evidence, /násilně/);
  assert.equal(settleStep("s", null).status, SKIP);
});

test("insideDir: uvnitř sandboxu bez ohledu na lomítka, uvozovky a velikost; sourozenec s podobným názvem ne", () => {
  const sb = String.raw`C:\Temp\ledge-qa-1\zaklad`;
  assert.equal(insideDir("C:/Temp/ledge-qa-1/zaklad/config/webview2", sb), true);
  assert.equal(insideDir(String.raw`"c:\temp\LEDGE-QA-1\zaklad\config\webview2"`, sb), true);
  assert.equal(insideDir(sb + "\\", sb), true, "koncové lomítko nevadí");
  assert.equal(insideDir(String.raw`C:\Temp\ledge-qa-1\zaklad2\webview2`, sb), false);
  assert.equal(insideDir(String.raw`C:\Users\x\AppData\Local\Ledge\EBWebView`, sb), false);
  assert.equal(insideDir("", sb), false);
});

test("userDataDir: z příkazové řádky s uvozovkami i bez", () => {
  assert.equal(userDataDir(String.raw`msedgewebview2.exe --user-data-dir="C:\a b\wv" --x=1`), String.raw`C:\a b\wv`);
  assert.equal(userDataDir(String.raw`msedgewebview2.exe --user-data-dir=C:\a\wv --x=1`), String.raw`C:\a\wv`);
  assert.equal(userDataDir("msedgewebview2.exe --type=gpu"), null);
});

test("dataDirVerdict: jen potomci instance; mimo sandbox / bez příznaku v hlavním procesu / bez procesů = nejde", () => {
  const sb = String.raw`C:\T\qa\zaklad`;
  const alien = String.raw`C:\Users\x\Ledge\EBWebView`;
  const wv = (pid, ppid, cmd) => ({ pid, ppid, name: "msedgewebview2.exe", cmd });
  const good = `msedgewebview2.exe --user-data-dir="${sb}\\config\\webview2" --remote-debugging-port=1`;
  const root = { pid: 10, ppid: 1, name: "ledge.exe", cmd: "ledge.exe" };
  const procs = [root, wv(11, 10, good), wv(12, 11, `${good} --type=renderer`), wv(13, 99, `msedgewebview2.exe --user-data-dir=${alien}`)]; // 13 = cizí strom
  assert.deepEqual(dataDirVerdict(procs, 10, sb), { ok: true, found: 2, bad: [] });
  const out = dataDirVerdict([root, wv(11, 10, good), wv(14, 10, `msedgewebview2.exe --user-data-dir=${alien}`)], 10, sb);
  assert.equal(out.ok, false);
  assert.match(out.bad[0], /^14: C:/);
  const nodir = dataDirVerdict([root, wv(15, 10, "msedgewebview2.exe --no-startup-window")], 10, sb);
  assert.equal(nodir.ok, false);
  assert.match(nodir.bad[0], /výchozí složka/);
  assert.equal(dataDirVerdict([root, wv(16, 10, "msedgewebview2.exe --type=gpu")], 10, sb).ok, false, "jen děti bez hlavního procesu nejde ověřit");
  assert.equal(dataDirVerdict([root], 10, sb).ok, false);
  assert.deepEqual(descendants(procs, 10).map((p) => p.pid).sort(), [11, 12]);
});

test("fakeServer (propojeni): jen loopback, odpoví JSON-RPC a spočítá požadavky", async () => {
  const { fakeServer } = await import("./qa/propojeni.mjs");
  const srv = await fakeServer();
  try {
    const r = await fetch(`http://127.0.0.1:${srv.port}/mcp`, { method: "POST", body: JSON.stringify({ jsonrpc: "2.0", id: 7, method: "tools/call" }) });
    assert.deepEqual(await r.json(), { jsonrpc: "2.0", id: 7, result: { content: [{ type: "text", text: "[]" }] } });
    assert.equal(srv.hits.length, 1);
    assert.ok(srv.port > 1023);
  } finally { await srv.close(); }
});
