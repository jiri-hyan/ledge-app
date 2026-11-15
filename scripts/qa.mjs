// QA před vydáním (L295, L296): spustí druhou, IZOLOVANOU instanci Ledge s testovacími daty (LEDGE_ISOLATED=1, vlastní
// config / appdata / localappdata / claude / poznámky v dočasné složce), řídí její WebView přes CDP (playwright-core),
// čte GET /status a vydá zprávu PASS/FAIL/SKIP s důkazy. Nesahá na nainstalovanou instanci (port 47800), na skutečná data,
// ani na plochu: žádný snímek obrazovky, žádná myš / klávesnice systému, žádný fokus.
//   node scripts/qa.mjs [--exe <ledge.exe>] [--old-exe <starý build>] [--out <složka>] [--only <scénář,…>] [--keep] [--list]
// Kódy: 0 = bez FAIL, 1 = FAIL, 2 = nespustitelné (hraje se, obsazený port, chybí binárka / klíč / podpora izolace).
import { mkdirSync, mkdtempSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadPlaywright } from "./showcase/pw.mjs";
import { ALL_SCENARIOS, wantsUpgrade, exitCode, parseArgs, renderReport, selectGroups, step, stdoutSummary, FAIL, SKIP, PASS } from "./qa/lib.mjs";
import { Hub, alive, apiClient, binaryInfo, findExe, killTree, leftovers, makeSandbox, notificationState, portFree, readHookAuth, sleep, checkDataDir, startInstance, supportsIsolation, tailLog } from "./qa/run.mjs";
import { RUNNERS, konzole } from "./qa/scenarios.mjs";
import { upgrade } from "./qa/upgrade.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const stamp = () => new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
const die = (msg) => { console.error(`QA nespustitelné: ${msg}`); process.exit(2); };

let opts;
try { opts = parseArgs(process.argv.slice(2)); } catch (e) { die(e.message); }
if (opts.list) { console.log(ALL_SCENARIOS.join("\n")); process.exit(0); }
let groups;
try { groups = selectGroups(opts.only); } catch (e) { die(e.message); }

// ---------- pojistky před startem ----------
const exe = opts.exe ? resolve(opts.exe) : findExe(ROOT);
if (!exe || !existsSync(exe)) die("ledge.exe nenalezen (--exe <cesta>, nebo cargo build / publish)");
if (!supportsIsolation(exe)) die("binárka nezná LEDGE_ISOLATED / LEDGE_CDP_PORT (starší než qa-jádro) — druhý start by zasignalizoval nainstalovaný Ledge");
const play = notificationState();
if (play === 3 || play === 4) die(`uživatel hraje / je na celé obrazovce (SHQueryUserNotificationState = ${play})`);
if (play === null) die("stav herního režimu uživatele se nepodařilo zjistit — bez něj aplikaci nespouštím");
for (const p of [opts.apiPort, opts.cdpPort]) if (!(await portFree(p))) die(`port ${p} je obsazený`);
const auth = readHookAuth();
if (!auth) die("v ~/.claude/settings.json nejsou hooky Ledge s klíčem (Nastavení → Claude → Zapojit hooky) — bez klíče není k API sandboxu přístup");

const out = resolve(opts.out ?? join(ROOT, ".qa", stamp()));
mkdirSync(out, { recursive: true });
const base = mkdtempSync(join(tmpdir(), "ledge-qa-"));
const { chromium } = loadPlaywright(ROOT);
const started = new Date().toISOString();
const t0 = Date.now();
const steps = [];
const notes = [];
const spawned = [];
const binary = binaryInfo(exe);
let fatal = null;

async function runGroup(g, attempt = 1) {
  const sb = makeSandbox(base, g.id, { apiPort: opts.apiPort, ...g.opts });
  const log = join(out, `instance-${g.id}.log`);
  const inst = startInstance(exe, sb, opts.cdpPort, log);
  spawned.push(inst.pid);
  const api = apiClient(opts.apiPort, auth);
  let browser = null;
  let retry = null;
  const add = (scenario, name, status, evidence) => steps.push(step(scenario, name, status, evidence));
  // start instance selhal poprvé: jeden pokus znovu (zapsáno do „Po cestě“, ne tiše); podruhé je to FAIL
  const fail = (why) => { if (attempt === 1) { retry = why.split(String.fromCharCode(10))[0]; return; } for (const s of g.scenarios) add(s, "start instance", FAIL, `${why}\n${tailLog(log, join(sb.cfg, "ledge.log"))}`); };
  try {
    const up = await api.waitStatus(() => true, 30_000);
    if (!up) return fail(inst.exited() !== null ? `instance skončila kódem ${inst.exited()}` : "GET /status neodpovídá do 30 s");
    // WebView2 vzniká líně (první okno): otevři šuplík a zkoušej CDP, než se port rozběhne (strop 25 s)
    await api.post("/ctl/open", { view: "full" }).catch(() => {});
    let lastErr = "";
    for (let i = 0; i < 50 && !browser; i++) {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${opts.cdpPort}`).catch((e) => { lastErr = String(e.message).split(String.fromCharCode(10))[0]; return null; });
      if (!browser) await sleep(500);
    }
    if (!browser) return fail(`CDP ${opts.cdpPort} nedostupné ani po 25 s: ${lastErr}`);
    // pojistka L299: WebView2 potomci instance MUSÍ mít data uvnitř sandboxu, jinak okamžitě pryč (kód 2)
    const dd = checkDataDir(inst.pid, sb.dir);
    if (!dd.ok) { killTree(inst.pid); throw Object.assign(new Error(`WebView2 instance nemá --user-data-dir uvnitř sandboxu ${sb.dir}: ${dd.bad.join("; ")}`), { fatal: true }); }
    notes.push(`pojistka dat: ${dd.found} procesů msedgewebview2.exe skupiny ${g.id}, všechny s --user-data-dir uvnitř sandboxu`);
    await api.post("/ctl/close").catch(() => {});
    const hub = new Hub(browser, out);
    const c = { api, hub, sb, add, addRaw: (s) => steps.push(s), note: (n) => { if (!notes.includes(n)) notes.push(n); } };
    for (const name of g.scenarios) {
      if (inst.exited() !== null) { add(name, "instance běží", FAIL, `spadla kódem ${inst.exited()}\n${tailLog(log, join(sb.cfg, "ledge.log"))}`); continue; }
      try { await RUNNERS[name](c); } catch (e) { add(name, "výjimka scénáře", FAIL, String(e?.stack ?? e).split("\n").slice(0, 3).join(" ")); }
    }
    konzole(c, `konzole(${g.id})`);
  } finally {
    await browser?.close().catch(() => {});
    killTree(inst.pid);
  }
  if (retry) {
    notes.push(`start skupiny ${g.id} selhal (${retry}) — instance spuštěna znovu, 2. pokus rozhoduje`);
    await freePorts();
    return runGroup(g, 2);
  }
}

/** Port API i CDP musí být volné, než se spustí další instance (obsazený port CDP WebView2 tiše vynechá a pak nejde připojit). */
async function freePorts() {
  for (let i = 0; i < 40 && !((await portFree(opts.apiPort)) && (await portFree(opts.cdpPort))); i++) await sleep(500);
}

/** Scénář `upgrade`: dvě binárky po sobě nad jedním sandboxem; bez `--old-exe` (nebo bez podpory izolace) SKIP, ne FAIL. */
async function runUpgrade() {
  const add = (name, status, evidence) => steps.push(step("upgrade", name, status, evidence));
  if (!opts.oldExe) return add("starý build", SKIP, "chybí --old-exe <build se starým identifikátorem>");
  const oldExe = resolve(opts.oldExe);
  if (!existsSync(oldExe)) return add("starý build", SKIP, `${oldExe} neexistuje`);
  if (!supportsIsolation(oldExe)) return add("starý build", SKIP, "starý build nezná LEDGE_ISOLATED / LEDGE_CDP_PORT — nešel by spustit izolovaně");
  notes.push(`upgrade: starý build ${binaryInfo(oldExe).sha}, nový ${binary.sha}`);
  await upgrade({ exe, oldExe, base, out, opts, auth, chromium, add, note: (n) => notes.push(n), spawned });
}

try {
  for (const g of groups) {
    await runGroup(g);
    await freePorts(); // porty se uvolní, než jede další skupina
  }
  if (wantsUpgrade(opts.only)) await runUpgrade();
} catch (e) {
  if (e?.fatal) fatal = e.message;
  steps.push(step("qa", "běh skriptu", FAIL, String(e?.stack ?? e).split("\n").slice(0, 3).join(" ")));
} finally {
  for (const pid of spawned) killTree(pid);
  await sleep(1000);
  // kontrola osiřelých: žijící vlastní PID a procesy s cestou sandboxu v příkazové řádce (ověřeno podle řádky, ne podle jména)
  const orphans = [...spawned.filter(alive).map((pid) => ({ pid, name: "ledge.exe" })), ...leftovers(base)];
  for (const o of orphans) killTree(o.pid);
  steps.push(step("qa", "úklid procesů", orphans.length ? SKIP : PASS, orphans.length ? `po ukončení zbylo ${orphans.length} procesů sandboxu (${orphans.map((o) => `${o.name}:${o.pid}`).join(", ")}) — ukončeny podle PID` : "žádné osiřelé procesy"));
  if (!opts.keep) { try { rmSync(base, { recursive: true, force: true }); } catch { /* soubory drží dojíždějící proces */ } }
  else notes.push(`sandbox ponechán: ${base}`);
}

const report = renderReport({ steps, binary, started, durationMs: Date.now() - t0, notes, out });
writeFileSync(join(out, "report.md"), report);
writeFileSync(join(out, "report.json"), JSON.stringify({ binary, started, durationMs: Date.now() - t0, steps, notes }, null, 2));
console.log(stdoutSummary(steps, join(out, "report.md")));
if (fatal) { console.error(`QA nespustitelné: ${fatal}`); process.exit(2); }
process.exit(exitCode(steps));
