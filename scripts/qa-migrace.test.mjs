// QA scénář migrace-id: čisté funkce nad dočasnou složkou (žádné skutečné %LOCALAPPDATA%, žádná instance).
// Spuštění: node --experimental-strip-types --test scripts/qa-migrace.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, cpSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { seedOldProfile, migraceKroky, OLD_PROFILE, NEW_ID, OLD_FAKE_ID, MARKER, STAGING, DEFERRED } from "./qa/migrace.mjs";
import { keepRel, listFiles, tally, webdataLines, parseCopied } from "./qa/upgrade.mjs";
import { installedProfileKind, dataDirVerdict, selectGroups, ALL_SCENARIOS, PASS, FAIL, SKIP } from "./qa/lib.mjs";

const tmp = () => mkdtempSync(join(tmpdir(), "ledge-migrace-test-"));
const status = (steps) => steps.map((s) => s.status);

/** Napodobí správně provedenou migraci: kopie jen souborů stavu do `<nový>\EBWebView` + značka. */
function fakeMigrate(local) {
  const oldRoot = join(local, OLD_FAKE_ID, "EBWebView");
  const newRoot = join(local, NEW_ID, "EBWebView");
  for (const [rel, f] of Object.entries(OLD_PROFILE)) {
    if (!f.state) continue;
    const to = join(newRoot, ...rel.split("/"));
    mkdirSync(join(to, ".."), { recursive: true });
    cpSync(join(oldRoot, ...rel.split("/")), to);
  }
  writeFileSync(join(local, NEW_ID, MARKER), JSON.stringify({ from: OLD_FAKE_ID, ver: "0.16.0", at: "2026-11-14T00:00:00Z" }));
}

test("migraceKroky: bez nové složky = stará nedotčená PASS, kopie SKIP (migrace se v izolaci nespustila)", () => {
  const local = tmp();
  try {
    seedOldProfile(local);
    assert.deepEqual(status(migraceKroky(local)), [PASS, SKIP]);
  } finally { rmSync(local, { recursive: true, force: true }); }
});

test("migraceKroky: správná migrace = vše PASS; stará složka zůstala", () => {
  const local = tmp();
  try {
    seedOldProfile(local);
    fakeMigrate(local);
    const steps = migraceKroky(local);
    assert.deepEqual(status(steps), [PASS, PASS, PASS, PASS, PASS, PASS, PASS], JSON.stringify(steps));
  } finally { rmSync(local, { recursive: true, force: true }); }
});

test("migraceKroky: zkopírovaná cache, zbytek .migrating, chybějící značka, změněná stará složka = FAIL", () => {
  const local = tmp();
  try {
    seedOldProfile(local);
    fakeMigrate(local);
    const cache = join(local, NEW_ID, "EBWebView", "Default", "Cache", "Cache_Data");
    mkdirSync(cache, { recursive: true });
    writeFileSync(join(cache, "data_0"), "cache");
    mkdirSync(join(local, NEW_ID, STAGING));
    writeFileSync(join(local, NEW_ID, DEFERRED), "");
    rmSync(join(local, NEW_ID, MARKER));
    writeFileSync(join(local, OLD_FAKE_ID, "EBWebView", "Local State"), "zmeneno");
    const steps = migraceKroky(local);
    assert.deepEqual(status(steps), [FAIL, FAIL, FAIL, FAIL, FAIL, PASS, FAIL], JSON.stringify(steps));
    assert.match(steps[0].evidence, /Local State/);
    assert.match(steps[2].evidence, /Cache_Data/);
  } finally { rmSync(local, { recursive: true, force: true }); }
});

test("migraceKroky: chybějící soubor stavu v nové složce = FAIL", () => {
  const local = tmp();
  try {
    seedOldProfile(local);
    fakeMigrate(local);
    rmSync(join(local, NEW_ID, "EBWebView", "Default", "Network", "Cookies"));
    const s = migraceKroky(local)[1];
    assert.equal(s.status, FAIL);
    assert.match(s.evidence, /Cookies/);
  } finally { rmSync(local, { recursive: true, force: true }); }
});

test("seedOldProfile: jen uvnitř zadané složky a s maskou starého identifikátoru", () => {
  const local = tmp();
  try {
    const root = seedOldProfile(local);
    assert.ok(root.startsWith(local));
    assert.ok(existsSync(join(root, "Local State")));
    assert.match(OLD_FAKE_ID, /^cz\.[^.]+\.ledge$/);
  } finally { rmSync(local, { recursive: true, force: true }); }
});

test("installedProfileKind: nový tvar a starý tvar (maska), cizí a sandbox ne", () => {
  const cases = [
    [String.raw`C:\Users\x\AppData\Local\app.ledge.desktop\EBWebView`, "new"],
    ['"c:/users/x/appdata/local/APP.LEDGE.DESKTOP/EBWebView/Default"', "new"],
    [String.raw`C:\Users\x\AppData\Local\cz.acme.ledge\EBWebView`, "old"],
    [String.raw`C:\Users\x\AppData\Local\cz.acme.ledge-wt\EBWebView`, null],
    [String.raw`C:\Users\x\AppData\Local\Ledge\EBWebView`, null],
    [String.raw`C:\T\qa\zaklad\config\webview2`, null],
    [null, null],
  ];
  for (const [p, want] of cases) assert.equal(installedProfileKind(p), want, String(p));
});

test("dataDirVerdict: profil nainstalované kopie je v hlášení pojmenovaný", () => {
  const sb = String.raw`C:\T\qa\zaklad`;
  const root = { pid: 10, ppid: 1, name: "ledge.exe", cmd: "ledge.exe" };
  const wv = { pid: 11, ppid: 10, name: "msedgewebview2.exe", cmd: String.raw`msedgewebview2.exe --user-data-dir=C:\Users\x\AppData\Local\app.ledge.desktop\EBWebView` };
  const v = dataDirVerdict([root, wv], 10, sb);
  assert.equal(v.ok, false);
  assert.match(v.bad[0], /SKUTEČNÝ PROFIL nainstalované kopie \(nový identifikátor\)/);
});

test("scénář migrace-id je ve výchozí skupině a jde zvolit přes --only", () => {
  assert.ok(ALL_SCENARIOS.includes("migrace-id"));
  const g = selectGroups(["migrace-id"]);
  assert.equal(g.length, 1);
  assert.equal(g[0].opts.oldProfile, true, "skupina musí založit atrapu starého profilu");
});

test("upgrade: keepRel zrcadlí webdata::keep (cache a provozní soubory ne, stav ano)", () => {
  for (const [p, want] of [["Local State", true], ["Default/Local Storage/leveldb/000003.log", true], ["Default/Network/Cookies", true], ["Default/Cache/Cache_Data/f", false],
    ["Default/Code Cache/js/x", false], ["GrShaderCache/a", false], ["Crashpad/x", false], ["lockfile", false], ["Default/Local Storage/leveldb/LOCK", false], ["BrowserMetrics-spare.pma", false], ["Default/Service Worker/CacheStorage/a", true]]) {
    assert.equal(keepRel(p), want, p);
  }
});

test("upgrade: listFiles/tally počítají jen vybrané soubory a podpis se změní se změnou velikosti", () => {
  const local = tmp();
  try {
    const root = seedOldProfile(local);
    const kept = tally(listFiles(root, true));
    const all = tally(listFiles(root, false));
    assert.equal(kept.files, 4);
    assert.equal(all.files, 7);
    assert.ok(all.bytes > kept.bytes);
    writeFileSync(join(root, "Local State"), "jiný obsah");
    assert.notEqual(tally(listFiles(root, true)).sig, kept.sig);
  } finally { rmSync(local, { recursive: true, force: true }); }
});

test("upgrade: webdataLines a parseCopied čtou log jádra (čas, doba mezi řádky, počty)", () => {
  const log = ["12:00:01.000 start", "12:00:01.200 jiný řádek", "12:00:01.950 webdata: profil WebView zkopírován (59 souborů, 123456 B), starý ponechán", "12:00:02.000 dál"].join("\n");
  const r = webdataLines(log);
  assert.equal(r.lines.length, 1);
  assert.equal(r.gapMs, 750);
  assert.deepEqual(parseCopied(r.lines[0]), { files: 59, bytes: 123456 });
  assert.equal(parseCopied("webdata: přenos profilu selhal (x)"), null);
  assert.deepEqual(webdataLines("").lines, []);
});
