// QA před vydáním (L295, L296): I/O vrstva — pojistky, sandbox, start/ukončení izolované instance, API, CDP.
// Nikdy nesahá na skutečné %APPDATA%\Ledge, %USERPROFILE%\Ledge ani registr; ukončuje JEN vlastní strom procesů (PID).
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, openSync, closeSync, readFileSync, writeFileSync, statSync } from "node:fs";
import { createServer } from "node:net";
import { homedir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { seedOldProfile } from "./migrace.mjs";
import { fixtureConfig, fixtureNotes, fixtureDesk, fixtureLimits, isNoise, dataDirVerdict } from "./lib.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ---------- pojistky ----------

/** SHQueryUserNotificationState (3 = busy, 4 = fullscreen game) — `null`, když se nepodařilo zjistit. */
export function notificationState() {
  const r = spawnSync("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", join(HERE, "hraje.ps1")], { encoding: "utf8", windowsHide: true });
  const n = Number.parseInt((r.stdout ?? "").trim(), 10);
  return Number.isNaN(n) ? null : n;
}

export const portFree = (port) => new Promise((res) => {
  const s = createServer();
  s.once("error", () => res(false));
  s.listen(port, "127.0.0.1", () => s.close(() => res(true)));
});

/** Nejnovější z existujících kandidátů; výchozí release, jinak nejnovější debug. */
export function findExe(root, env = process.env) {
  const rel = join(root, "src-tauri", "target", "release", "ledge.exe");
  if (existsSync(rel)) return rel;
  const dbg = [env.CARGO_TARGET_DIR, join(root, "src-tauri", "target-agents"), join(root, "src-tauri", "target")]
    .filter(Boolean).map((d) => join(d, "debug", "ledge.exe")).filter(existsSync);
  return dbg.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0] ?? null;
}

/** Binárka musí znát izolovaný režim — jinak by druhý start zasignalizoval (a otevřel) šéfovu instanci. */
export function supportsIsolation(exe) {
  const b = readFileSync(exe);
  return b.includes("LEDGE_ISOLATED") && b.includes("LEDGE_CDP_PORT");
}

export function binaryInfo(exe) {
  const sha = createHash("sha256").update(readFileSync(exe)).digest("hex").slice(0, 12);
  const r = spawnSync("powershell", ["-NoProfile", "-Command", `(Get-Item -LiteralPath '${exe.replace(/'/g, "''")}').VersionInfo.ProductVersion`], { encoding: "utf8", windowsHide: true });
  return { path: exe, sha: `sha256:${sha}`, version: (r.stdout ?? "").trim() || null };
}

/** Klíč hooků k API sandboxové instance (sdílí šéfův Správce pověření) — jen ČTENÍ ~/.claude/settings.json, nikdy se nevypisuje. */
export function readHookAuth() {
  try {
    const j = JSON.parse(readFileSync(join(homedir(), ".claude", "settings.json"), "utf8"));
    for (const ev of Object.values(j.hooks ?? {})) {
      for (const g of ev ?? []) {
        for (const h of g.hooks ?? []) {
          if (String(h.url ?? "").startsWith("http://127.0.0.1:") && h.headers?.Authorization) return h.headers.Authorization;
        }
      }
    }
  } catch { /* chybí */ }
  return null;
}

// ---------- sandbox ----------

const put = (p, text) => { mkdirSync(dirname(p), { recursive: true }); writeFileSync(p, typeof text === "string" ? text : JSON.stringify(text, null, 2)); };

/** Složky a fixture data jedné instance; vrací env a cesty. */
export function makeSandbox(base, id, { apiPort, hotkeys, limits, desk, oldProfile }) {
  const dir = join(base, id);
  const p = { dir, home: join(dir, "home"), cfg: join(dir, "config"), appdata: join(dir, "appdata"), local: join(dir, "localappdata"), claude: join(dir, "claude"), notes: join(dir, "notes") };
  for (const d of Object.values(p)) mkdirSync(d, { recursive: true });
  put(join(p.cfg, "config.json"), fixtureConfig({ apiPort, hotkeys, notesRoot: p.notes }));
  for (const [rel, body] of Object.entries(fixtureNotes())) put(join(p.notes, rel), body);
  // s LEDGE_CONFIG_DIR jde „místní“ složka do <config>/local; pro jistotu i %LOCALAPPDATA%\Ledge
  for (const loc of [join(p.cfg, "local"), join(p.local, "Ledge")]) {
    if (desk) put(join(loc, "desk.json"), fixtureDesk());
    if (limits) put(join(loc, "ratelimits.json"), fixtureLimits(Date.now()));
  }
  put(join(p.claude, "settings.json"), {});
  if (oldProfile) seedOldProfile(p.local); // atrapa starého profilu WebView2 pro scénář migrace-id (jen uvnitř sandboxu)
  // USERPROFILE se NEpřepisuje: s cizím USERPROFILE WebView2 neotevře port CDP (změřeno, Edge 157); domov čte Ledge jen pro čtení
  // (config i poznámky míří do sandboxu přes LEDGE_CONFIG_DIR / notesRoot, migrace je s LEDGE_CONFIG_DIR vypnutá).
  const env = { ...process.env, LEDGE_ISOLATED: "1", LEDGE_CONFIG_DIR: p.cfg, APPDATA: p.appdata, LOCALAPPDATA: p.local, CLAUDE_CONFIG_DIR: p.claude };
  delete env.LEDGE_NO_GUI;
  return { ...p, env, notePath: (rel) => join(p.notes, ...rel.split("/")) };
}

/** Ledge `--ext` hosty a WebView2 děti jsou potomci ledge.exe; ukončí se celý strom podle PID. */
export function killTree(pid) {
  if (pid) spawnSync("taskkill", ["/PID", String(pid), "/T", "/F"], { windowsHide: true, stdio: "ignore" });
}
export const alive = (pid) => {
  const r = spawnSync("tasklist", ["/FI", `PID eq ${pid}`, "/NH"], { encoding: "utf8", windowsHide: true });
  return new RegExp(`\\b${pid}\\b`).test(r.stdout ?? "");
};

/** Procesy, jejichž příkazová řádka obsahuje cestu sandboxu (osiřelé WebView2 apod.) — [{pid, name}]. */
export function leftovers(sandboxDir) {
  const needle = sandboxDir.replace(/'/g, "''");
  const ps = `Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and $_.CommandLine.Contains('${needle}') -and $_.ProcessId -ne $PID } | ForEach-Object { "$($_.ProcessId)|$($_.Name)" }`;
  const r = spawnSync("powershell", ["-NoProfile", "-Command", ps], { encoding: "utf8", windowsHide: true });
  return (r.stdout ?? "").split(/\r?\n/).filter(Boolean).map((l) => { const [pid, name] = l.split("|"); return { pid: Number(pid), name }; });
}

/** Plochý seznam procesů {pid, ppid, name, cmd} (jen čtení; Win32_Process). */
export function listProcesses() {
  const ps = `Get-CimInstance Win32_Process | ForEach-Object { "$($_.ProcessId)|$($_.ParentProcessId)|$($_.Name)|$($_.CommandLine)" }`;
  const r = spawnSync("powershell", ["-NoProfile", "-Command", ps], { encoding: "utf8", windowsHide: true, maxBuffer: 64 << 20 });
  return (r.stdout ?? "").split(/\r?\n/).filter(Boolean).map((l) => { const [pid, ppid, name, ...cmd] = l.split("|"); return { pid: Number(pid), ppid: Number(ppid), name, cmd: cmd.join("|") }; });
}

/** Pojistka L299: WebView2 potomci instance musí mít `--user-data-dir` uvnitř sandboxu (jinak by sdílely data s nainstalovaným Ledge). */
export const checkDataDir = (rootPid, sandboxDir) => dataDirVerdict(listProcesses(), rootPid, sandboxDir);

// ---------- instance ----------

export function startInstance(exe, sb, cdpPort, logPath) {
  mkdirSync(dirname(logPath), { recursive: true });
  const fd = openSync(logPath, "a");
  const child = spawn(exe, [], { env: { ...sb.env, LEDGE_CDP_PORT: String(cdpPort) }, windowsHide: true, stdio: ["ignore", fd, fd] });
  closeSync(fd);
  let exited = null;
  child.on("exit", (code) => { exited = code ?? -1; });
  return { pid: child.pid, exited: () => exited, child };
}

export function tailLog(...paths) {
  return paths.filter((p) => existsSync(p)).map((p) => readFileSync(p, "utf8").split(/\r?\n/).slice(-15).join("\n")).join("\n").slice(-1500);
}

/** Klient lokálního API sandboxové instance. Klíč se nikam nevypisuje. */
export function apiClient(port, auth) {
  const base = `http://127.0.0.1:${port}`;
  const h = { Authorization: auth, "Content-Type": "application/json" };
  const get = async (path) => { const r = await fetch(base + path, { headers: h, signal: AbortSignal.timeout(5000) }); if (!r.ok) throw new Error(`GET ${path} → ${r.status}`); return r.json(); };
  const post = async (path, body = {}) => { const r = await fetch(base + path, { method: "POST", headers: h, body: JSON.stringify(body), signal: AbortSignal.timeout(5000) }); if (!r.ok) throw new Error(`POST ${path} → ${r.status}`); return r.json().catch(() => ({})); };
  const status = () => get("/status");
  /** Čeká, až `pred(status)` platí; vrací poslední status (platný) nebo null po limitu. */
  async function waitStatus(pred, ms = 8000) {
    const end = Date.now() + ms;
    let last = null;
    while (Date.now() < end) {
      try { last = await status(); if (pred(last)) return last; } catch { /* zatím nic */ }
      await sleep(250);
    }
    return null;
  }
  return { status, post, waitStatus };
}

// ---------- CDP ----------

const KIND_RE = /\/(index|pin|desk|settings)\.html/;
// šuplík se v produkčním buildu načítá z kořene (`http://tauri.localhost/`), ne z `/index.html`
const ROOT_RE = /^https?:\/\/(tauri\.localhost|localhost(:\d+)?)\/?([?#].*)?$/;
export const kindOf = (url) => KIND_RE.exec(url ?? "")?.[1] ?? (ROOT_RE.test(url ?? "") ? "index" : null);

const CSP_TAG = "__LEDGE_CSP__";
/** Init skript každého okna: porušení CSP → console.debug s JSON (prvek, zdroj, ukázka); chybová zpráva prohlížeče sama prvek neřekne. */
const CSP_PROBE = String.raw`(() => {
  // inline styl blokovaný CSP nese jen e.target = document: zápisy cssText / setAttribute("style") si proto pamatujeme (hodnota + zásobník)
  const writes = [];
  const keep = (kind, el, v) => { try { writes.push({ kind, el: el && el.outerHTML ? el.outerHTML.slice(0, 120) : "", v: String(v).slice(0, 160), at: String(new Error().stack).split("\n").slice(2, 5).join(" <- ").replace(/https?:\/\/[^/]+/g, "") }); if (writes.length > 8) writes.shift(); } catch (_) {} };
  const d = Object.getOwnPropertyDescriptor(CSSStyleDeclaration.prototype, "cssText");
  if (d && d.set) Object.defineProperty(CSSStyleDeclaration.prototype, "cssText", { ...d, set(v) { keep("cssText", null, v); return d.set.call(this, v); } });
  // Solid skládá šablony přes innerHTML (static style="…" v JSX) — i to CSP blokuje (style-src-attr)
  const ih = Object.getOwnPropertyDescriptor(Element.prototype, "innerHTML");
  if (ih && ih.set) Object.defineProperty(Element.prototype, "innerHTML", { ...ih, set(v) { if (/\sstyle\s*=/.test(String(v))) keep("innerHTML", null, String(v).replace(/\s+/g, " ")); return ih.set.call(this, v); } });
  const sa = Element.prototype.setAttribute;
  Element.prototype.setAttribute = function (n, v) { if (String(n).toLowerCase() === "style") keep("setAttribute", this, v); return sa.call(this, n, v); };
  document.addEventListener("securitypolicyviolation", (e) => {
    const t = e.target; let el = "";
    try { el = t && t.outerHTML ? t.outerHTML.slice(0, 200) : ""; } catch (_) {}
    console.debug("${CSP_TAG}" + JSON.stringify({ directive: e.violatedDirective, blockedURI: e.blockedURI, sourceFile: e.sourceFile, line: e.lineNumber, col: e.columnNumber, sample: e.sample, el, writes: writes.slice(-8) }));
  }, true);
})();`;

/** Okna WebView podle URL + sběr console.error a neodchycených chyb stránek. */
export class Hub {
  constructor(browser, outDir) {
    this.browser = browser;
    this.outDir = outDir;
    this.errors = [];
    this.kinds = new Set();
    this.seen = new WeakSet();
    this.csp = []; // porušení CSP (securitypolicyviolation) s prvkem a zdrojem — doplní „kde“ do chyby konzole
    this.lastSettle = null;
    for (const c of browser.contexts()) {
      c.on("page", (p) => this.attach(p));
      c.addInitScript(CSP_PROBE).catch(() => {});
    }
  }
  attach(p) {
    if (this.seen.has(p)) return;
    this.seen.add(p);
    const tag = () => kindOf(p.url()) ?? "?";
    p.on("console", (m) => {
      if (m.type() === "debug" && m.text().startsWith(CSP_TAG)) { try { this.csp.push({ page: tag(), ...JSON.parse(m.text().slice(CSP_TAG.length)) }); } catch { /* nečitelné */ } return; }
      if (m.type() === "error" && !isNoise(m.text())) { const l = m.location(); this.errors.push({ page: tag(), text: `${m.text().slice(0, 300)}${l?.url ? ` @ ${l.url.replace(/^https?:\/\/[^/]+/, "")}:${l.lineNumber ?? "?"}` : ""}` }); }
    });
    p.on("pageerror", (e) => this.errors.push({ page: tag(), text: `neodchycená chyba: ${String(e?.message ?? e).slice(0, 300)}` }));
  }
  pages() {
    const all = this.browser.contexts().flatMap((c) => c.pages());
    for (const p of all) { this.attach(p); const k = kindOf(p.url()); if (k) this.kinds.add(k); }
    return all;
  }
  async find(kind, ms = 10_000) {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      const p = this.pages().find((x) => kindOf(x.url()) === kind);
      if (p) return p;
      await sleep(250);
    }
    return null;
  }
  /**
   * Počká na doběhnutí konečných animací (vstupní blur/posun šuplíku, src/motion.ts), strop `cap` ms; co nedoběhne, dokončí násilně,
   * ať je snímek čitelný. Nekonečné (smyčky) se nečekají. Vrací {total, finite, running, forced, visibility, ms}.
   */
  async settle(page, cap = 3000) {
    const t = Date.now();
    const r = await page.evaluate(async (cap) => {
      const finite = () => document.getAnimations().filter((a) => { const e = a.effect?.getComputedTiming?.(); return e && Number.isFinite(e.endTime) && a.playState !== "finished" && a.playState !== "idle"; });
      const total = document.getAnimations().length;
      const first = finite().length;
      const done = await Promise.race([Promise.all(finite().map((a) => a.finished.catch(() => {}))).then(() => true), new Promise((res) => setTimeout(() => res(false), cap))]);
      const left = finite();
      left.forEach((a) => { try { a.finish(); } catch { /* zrušená */ } });
      return { total, finite: first, running: done ? 0 : left.length, forced: left.length, visibility: document.visibilityState };
    }, cap).catch((e) => ({ total: 0, finite: 0, running: 0, forced: 0, visibility: "?", err: String(e.message).slice(0, 60) }));
    r.ms = Date.now() - t;
    await sleep(120); // jeden snímek na vykreslení po dokončení
    this.lastSettle = r;
    return r;
  }
  /** Snímek JEN obsahu WebView (CDP Page.captureScreenshot); vrací název souboru, nebo null s důvodem. */
  async shot(page, name) {
    try {
      mkdirSync(this.outDir, { recursive: true });
      await this.settle(page);
      await page.screenshot({ path: join(this.outDir, `${name}.png`), timeout: 8000 });
      return `${name}.png`;
    } catch (e) {
      return `snímek nelze (${String(e.message).split("\n")[0].slice(0, 80)})`;
    }
  }
}

/** Ticho jako hrající relace (GSMTC): 8 kHz mono 8 bit, 300 s — vygeneruje node (PowerShell by to dělal dlouho). */
export function writeSilence(dir) {
  const n = 8000 * 300;
  const h = Buffer.alloc(44);
  h.write("RIFF", 0); h.writeInt32LE(36 + n, 4); h.write("WAVEfmt ", 8); h.writeInt32LE(16, 16); h.writeInt16LE(1, 20); h.writeInt16LE(1, 22);
  h.writeInt32LE(8000, 24); h.writeInt32LE(8000, 28); h.writeInt16LE(1, 32); h.writeInt16LE(8, 34); h.write("data", 36); h.writeInt32LE(n, 40);
  writeFileSync(join(dir, "ticho.wav"), Buffer.concat([h, Buffer.alloc(n, 0x80)]));
}

export function startPlayer(dir, seconds = 150) {
  writeSilence(dir);
  const child = spawn("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", join(HERE, "ticho.ps1"), "-Seconds", String(seconds), "-Dir", dir], { windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  let said = "";
  let wake = () => {};
  child.stdout.on("data", (d) => { said += d; wake(); });
  child.stderr.on("data", (d) => { said += d; wake(); });
  child.on("exit", () => wake());
  /** Počká na řádek "prehravac hraje" (přehrávač hlásí, že stav je Playing, nebo po 15 s co je); vrací {ok, said, ms}. Bez toho se ostrůvek otevíral dřív, než relace existovala. */
  child.ready = async (ms = 20_000) => {
    const t = Date.now();
    while (Date.now() - t < ms && !/prehravac hraje/.test(said) && child.exitCode === null) await new Promise((r) => { wake = r; setTimeout(r, 250); });
    return { ok: /prehravac hraje: Playing/.test(said), said: said.trim().replace(/\s+/g, " ").slice(0, 200), ms: Date.now() - t };
  };
  return child;
}
