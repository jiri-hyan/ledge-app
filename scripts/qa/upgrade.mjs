// QA scénář `upgrade` (kolo 35): skutečný přenos profilu WebView2 při přechodu starý identifikátor → `app.ledge.desktop`.
// Dvě binárky PO SOBĚ (nikdy naráz) nad STEJNÝM sandboxem (stejné falešné %LOCALAPPDATA%): STARÁ (`--old-exe`, build se starým
// identifikátorem) zapíše localStorage + cookie, NOVÁ pak při startu zkopíruje profil (`webdata.rs`) a nad kopií ho musí PŘEČÍST.
// Izolace s CDP parkuje data WebView2 do `<LEDGE_CONFIG_DIR>/webview2` (`qa::data_dir`), takže by migrovaný profil nikdy nečetla.
// Řešení bez zásahu do jádra: `<cfg>/webview2` je JUNCTION na složku dat podle identifikátoru dané binárky (`<local>\cz.acme.ledge`
// pro starou, `<local>\app.ledge.desktop` pro novou) — WebView2 pak čte přesně tvar, který by měl v instalaci (`<složka>\EBWebView`).
// Kroky: A (stará zapíše), B (nová migruje a čte), C (druhý start, bez opakování), D (odklad: profil starého drží zámek).
// Nesahá na nic mimo sandbox; ukončuje JEN vlastní PID (a vlastní držák zámku).
import { cpSync, existsSync, lstatSync, mkdirSync, openSync, closeSync, readFileSync, readdirSync, rmdirSync, statSync, writeFileSync, copyFileSync } from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import { join, relative } from "node:path";
import { PASS, FAIL, SKIP, step } from "./lib.mjs";
import { Hub, apiClient, checkDataDir, killTree, makeSandbox, portFree, sleep, startInstance, tailLog } from "./run.mjs";
import { NEW_ID, OLD_FAKE_ID, MARKER, DEFERRED, STAGING, CLEAN_PREFIX } from "./migrace.mjs";

export const S = "upgrade";
export const LS_KEY = "ledge-qa-upgrade";
export const COOKIE = "ledge_qa_upgrade";

// ---------- čisté pomocné funkce (test: scripts/qa-migrace.test.mjs) ----------

/** Zrcadlo `webdata::keep`: patří cesta (relativní uvnitř `EBWebView`) do kopie? Cache a provozní soubory ne. */
export function keepRel(rel) {
  const deny = ["cache", "code cache", "gpucache", "grshadercache", "shadercache", "dawncache", "graphitedawncache", "crashpad", "lockfile", "lock"];
  return !String(rel).split(/[\\/]/).some((c) => { const l = c.toLowerCase(); return deny.includes(l) || l.startsWith("browsermetrics"); });
}

/** Rekurzivní výpis souborů `[rel, size]` (rel s `/`), seřazený; volitelně jen to, co projde `keepRel`. */
export function listFiles(dir, onlyKept = false, base = dir, out = []) {
  if (!existsSync(dir)) return out;
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    const rel = relative(base, p).replace(/\\/g, "/");
    if (onlyKept && !keepRel(rel)) continue;
    if (e.isDirectory()) listFiles(p, onlyKept, base, out); else out.push([rel, statSync(p).size]);
  }
  return out.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
}

/** { files, bytes, sig } výpisu; `sig` = text pro porovnání „před = po“. */
export function tally(list) {
  return { files: list.length, bytes: list.reduce((n, [, s]) => n + s, 0), sig: list.map(([r, s]) => `${r}:${s}`).join("\n") };
}

/** Řádky logu `webdata:` s časem (ms od půlnoci) a řádek těsně před prvním z nich (horní odhad doby migrace). */
export function webdataLines(text) {
  const lines = String(text ?? "").split(/\r?\n/).filter(Boolean);
  const ms = (l) => { const m = /^(\d\d):(\d\d):(\d\d)\.(\d{3}) /.exec(l); return m ? ((+m[1] * 60 + +m[2]) * 60 + +m[3]) * 1000 + +m[4] : null; };
  const idx = lines.findIndex((l) => / webdata: /.test(l));
  const wd = lines.filter((l) => / webdata: /.test(l));
  const prev = idx > 0 ? ms(lines[idx - 1]) : null;
  const at = idx >= 0 ? ms(lines[idx]) : null;
  return { lines: wd, gapMs: prev !== null && at !== null && at >= prev ? at - prev : null };
}

/** „zkopírován (N souborů, M B)“ → { files, bytes }; `null`, když řádek jinak. */
export function parseCopied(line) {
  const m = /zkopírován \((\d+) souborů, (\d+) B/.exec(String(line ?? ""));
  return m ? { files: Number(m[1]), bytes: Number(m[2]) } : null;
}

// ---------- I/O ----------

/** Junction `link` → `target` (bez práv správce); `target` musí existovat. */
export function mkJunction(link, target) {
  mkdirSync(target, { recursive: true });
  const r = spawnSync("cmd.exe", ["/d", "/c", "mklink", "/J", link, target], { windowsHide: true, encoding: "utf8" });
  if (!existsSync(link)) throw new Error(`junction ${link} se nezaložila: ${(r.stdout ?? "") + (r.stderr ?? "")}`.trim());
}
/** Odpojí junction (rmdir na linku nemaže obsah cíle); jen pokud to opravdu je odkaz. */
export function rmJunction(link) {
  try { if (lstatSync(link).isSymbolicLink()) rmdirSync(link); } catch { /* neexistuje */ }
}
/** Přesměruje `<cfg>/webview2` na složku dat dané binárky v `local`. */
function pointData(sb, id) {
  const link = join(sb.cfg, "webview2");
  rmJunction(link);
  mkJunction(link, join(sb.local, id));
}

/** Drží soubor někdo (otevření ke čtení a zápisu selže sdílením)? Chybějící soubor NENÍ zámek. */
export const lockedNow = (p) => { if (!existsSync(p)) return false; try { closeSync(openSync(p, "r+")); return false; } catch (e) { return ["EBUSY", "EPERM", "EACCES"].includes(e.code); } };

/**
 * Držák zámku (PowerShell, ukončí se podle vlastního PID). `exclusive` = `FileShare.None` (nejostřejší zámek);
 * `chromium` = jako `lockfile` živého Chromia/WebView2: zápis, sdílení jen pro ČTENÍ, smazat při zavření (změřeno naživo, viz webdata.rs).
 */
export function holdLock(file, seconds = 180, mode = "exclusive") {
  writeFileSync(file, ""); // soubor musí existovat dřív, než držák naběhne (jinak je okno bez zámku)
  const q = file.replace(/'/g, "''");
  const open = mode === "chromium" ? `New-Object IO.FileStream('${q}','Create','Write','Read',4096,'DeleteOnClose')` : `[IO.File]::Open('${q}','Open','ReadWrite','None')`;
  const ps = `$f=${open}; Start-Sleep -Seconds ${seconds}; $f.Close()`;
  return spawn("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps], { windowsHide: true, stdio: "ignore" });
}

/**
 * Jedno spuštění binárky nad sandboxem: start → /status → okno šuplíku → CDP → pojistka dat → `fn(h)` → korektní ukončení
 * (Browser.close přes CDP vyprázdní leveldb i cookies, pak se zabije vlastní strom podle PID). Vrací { ms: doba do /status }.
 */
async function cycle(x, label, exe, sb, fn, { statusMs = 30_000 } = {}) {
  const log = join(x.out, `instance-upgrade-${label}.log`);
  const t0 = Date.now();
  const inst = startInstance(exe, sb, x.opts.cdpPort, log);
  x.spawned?.push(inst.pid);
  const api = apiClient(x.opts.apiPort, x.auth);
  let browser = null;
  let up = null;
  try {
    up = await api.waitStatus(() => true, statusMs);
    const ms = Date.now() - t0;
    if (!up) { x.add(`${label}: start instance`, FAIL, `${inst.exited() !== null ? `skončila kódem ${inst.exited()}` : "GET /status neodpovídá"}\n${tailLog(log, join(sb.cfg, "ledge.log"))}`); return { ms: null, ok: false }; }
    await api.post("/ctl/open", { view: "full" }).catch(() => {});
    let err = "";
    for (let i = 0; i < 50 && !browser; i++) {
      browser = await x.chromium.connectOverCDP(`http://127.0.0.1:${x.opts.cdpPort}`).catch((e) => { err = String(e.message).split("\n")[0]; return null; });
      if (!browser) await sleep(500);
    }
    if (!browser) { x.add(`${label}: CDP`, FAIL, `CDP ${x.opts.cdpPort} nedostupné: ${err}`); return { ms, ok: false }; }
    const dd = checkDataDir(inst.pid, sb.dir);
    if (!dd.ok) { x.add(`${label}: pojistka dat`, FAIL, dd.bad.join("; ")); return { ms, ok: false }; }
    const hub = new Hub(browser, x.out);
    const page = await hub.find("index", 15_000);
    await fn({ api, browser, hub, page, ms, inst, log });
    return { ms, ok: true };
  } finally {
    if (browser) {
      try { const s = await browser.newBrowserCDPSession(); await s.send("Browser.close").catch(() => {}); } catch { /* spojení už není */ }
      await browser.close().catch(() => {});
      for (let i = 0; i < 40 && !(await portFree(x.opts.cdpPort)); i++) await sleep(250);
      await sleep(500);
    }
    killTree(inst.pid);
    for (let i = 0; i < 40 && !((await portFree(x.opts.apiPort)) && (await portFree(x.opts.cdpPort))); i++) await sleep(500);
    try { copyFileSync(join(sb.cfg, "ledge.log"), join(x.out, `upgrade-ledge-${label}.log`)); } catch { /* bez logu */ }
  }
}

const readLog = (sb) => { try { return readFileSync(join(sb.cfg, "ledge.log"), "utf8"); } catch { return ""; } };
const readJson = (p) => { try { return JSON.parse(readFileSync(p, "utf8")); } catch { return null; } };
const ls = (page) => page.evaluate(([k]) => { const v = localStorage.getItem(k); return v === null ? null : { len: v.length, head: v.slice(0, 40) }; }, [LS_KEY]);
const cookiePresent = async (browser, name) => { const all = await browser.contexts()[0].cookies("http://tauri.localhost/").catch(() => []); return all.find((c) => c.name === name)?.value ?? null; };

/** Scénář `upgrade`; `x` = { exe, oldExe, base, out, opts:{apiPort,cdpPort}, auth, chromium, add(name,status,evidence), note(text), spawned }. */
export async function upgrade(x) {
  const add = (name, status, evidence = "") => x.add(name, status, evidence);
  const sbs = [];
  const value = `qa-${Date.now().toString(36)}-${"x".repeat(48_000)}`;
  const head = value.slice(0, 40);
  const cookieVal = `ck-${Date.now().toString(36)}`;
  let holder = null;
  try {
    // ---------- A: stará binárka zapíše stav ----------
    const sb = makeSandbox(x.base, "upgrade", { apiPort: x.opts.apiPort, hotkeys: false, limits: false, desk: false });
    sbs.push(sb);
    const oldDir = join(sb.local, OLD_FAKE_ID);
    const newDir = join(sb.local, NEW_ID);
    const oldProf = join(oldDir, "EBWebView");
    pointData(sb, OLD_FAKE_ID);
    let wroteLs = false;
    let wroteCookie = false;
    const a = await cycle(x, "A", x.oldExe, sb, async ({ page, browser }) => {
      if (!page) return add("A: okno šuplíku", FAIL, "WebView šuplíku (index) se přes CDP nenašlo");
      const back = await page.evaluate(([k, v]) => { localStorage.setItem(k, v); const r = localStorage.getItem(k); return r === null ? null : { len: r.length, head: r.slice(0, 40) }; }, [LS_KEY, value]);
      wroteLs = !!back && back.len === value.length && back.head === head;
      add("A: stará zapsala localStorage a přečetla ho zpět", wroteLs ? PASS : FAIL, back ? `${back.len} znaků` : "getItem vrátil null");
      try {
        await browser.contexts()[0].addCookies([{ name: COOKIE, value: cookieVal, url: "http://tauri.localhost/", expires: Math.floor(Date.now() / 1000) + 86400 }]);
        wroteCookie = (await cookiePresent(browser, COOKIE)) === cookieVal;
        add("A: stará zapsala cookie a přečetla ji zpět", wroteCookie ? PASS : FAIL, wroteCookie ? `cookie ${COOKIE} (trvalá, 1 den)` : "cookie se nezapsala");
      } catch (e) { add("A: stará zapsala cookie", SKIP, `addCookies selhalo: ${String(e.message).split("\n")[0].slice(0, 100)}`); }
      await sleep(4000); // commit leveldb (localStorage) před ukončením
    });
    if (!a.ok || !wroteLs) { add("A: stará binárka", FAIL, "bez zapsané hodnoty nemá smysl pokračovat"); return; }
    const hasShape = existsSync(join(oldProf, "Default", "Local Storage")) && existsSync(join(oldProf, "Local State"));
    add("A: ve falešném LOCALAPPDATA vznikla složka starého tvaru s EBWebView", hasShape ? PASS : FAIL, hasShape ? `${OLD_FAKE_ID}\\EBWebView (Local State + Default\\Local Storage)` : `chybí ${oldProf}\\Local State nebo Default\\Local Storage`);
    if (!hasShape) return;
    // sentinely v cache: nová instance si cache vytvoří sama, proto se kopie cache ověřuje značkami, ne pouhou existencí složky
    for (const rel of ["Default/Cache/Cache_Data/qa-sentinel", "Default/Code Cache/js/qa-sentinel", "GrShaderCache/qa-sentinel"]) {
      const p = join(oldProf, ...rel.split("/"));
      mkdirSync(join(p, ".."), { recursive: true });
      writeFileSync(p, "cache");
    }
    const stateOld = tally(listFiles(oldProf, true));
    const allOld = tally(listFiles(oldProf, false));
    add("A: stav starého profilu (bez cache)", PASS, `${stateOld.files} souborů, ${stateOld.bytes} B (celkem s cache a provozními soubory ${allOld.files} souborů, ${allOld.bytes} B); start staré ${a.ms} ms`);

    // ---------- B: nová migruje a čte ----------
    pointData(sb, NEW_ID);
    let markerSeen = null;
    const b = await cycle(x, "B", x.exe, sb, async ({ page, browser, hub }) => {
      if (!page) return add("B: okno šuplíku", FAIL, "WebView šuplíku se přes CDP nenašlo");
      const got = await ls(page);
      add("B: localStorage v nové instanci vrací hodnotu zapsanou starou (HLAVNÍ)", got && got.len === value.length && got.head === head ? PASS : FAIL, got ? `${got.len} znaků (čekáno ${value.length})` : "getItem vrátil null — WebView2 zkopírovaný profil nepřečetlo");
      const ck = await cookiePresent(browser, COOKIE);
      add("B: cookie zapsaná starou je čitelná", !wroteCookie ? SKIP : ck === cookieVal ? PASS : FAIL, !wroteCookie ? "stará cookie nezapsala" : ck === null ? "cookie chybí" : ck === cookieVal ? `${COOKIE} shodná` : "jiná hodnota");
      add("B: snímek obsahu okna po migraci", PASS, await hub.shot(page, "upgrade-B"));
      markerSeen = readJson(join(newDir, MARKER));
      await sleep(500);
      x.hubB = hub;
    });
    if (!b.ok) return;
    const wd = webdataLines(readLog(sb));
    const copied = parseCopied(wd.lines.find((l) => /zkopírován/.test(l)));
    add("B: v ledge.log jsou řádky webdata:", wd.lines.length ? PASS : FAIL, wd.lines.map((l) => l.slice(0, 160)).join(" | ") || "žádný řádek");
    add("B: počet souborů a bajtů z logu = stav staré složky (bez cache)", copied && copied.files === stateOld.files && copied.bytes === stateOld.bytes ? PASS : FAIL, copied ? `log ${copied.files} souborů / ${copied.bytes} B, stará ${stateOld.files} / ${stateOld.bytes}` : "řádek „zkopírován (N souborů, M B)“ chybí");
    add("B: doba migrace (horní mez: celý start nové včetně migrace do /status)", b.ms < 5000 ? PASS : FAIL, `≤ ${b.ms} ms pro ${copied?.files ?? "?"} souborů / ${copied?.bytes ?? "?"} B`);
    add("B: značka .migrated-from jmenuje zdroj", markerSeen && markerSeen.from === OLD_FAKE_ID ? PASS : FAIL, markerSeen ? JSON.stringify(markerSeen) : `chybí ${MARKER}`);
    add("B: žádný zbytek EBWebView.migrating ani značka odkladu", !existsSync(join(newDir, STAGING)) && !existsSync(join(newDir, DEFERRED)) ? PASS : FAIL, readdirSync(newDir).join(", "));
    const newProf = join(newDir, "EBWebView");
    const leaked = ["Default/Cache/Cache_Data/qa-sentinel", "Default/Code Cache/js/qa-sentinel", "GrShaderCache/qa-sentinel"].filter((r) => existsSync(join(newProf, ...r.split("/"))));
    add("B: cache se nekopírovala (sentinely z cache staré složky v nové nejsou)", leaked.length ? FAIL : PASS, leaked.length ? `zkopírováno: ${leaked.join(", ")}` : "3 sentinely nepřenesené");
    const afterB = tally(listFiles(oldProf, false));
    add("B: stará složka je nedotčená (počet souborů a bajtů před = po)", afterB.sig === allOld.sig ? PASS : FAIL, `před ${allOld.files} souborů / ${allOld.bytes} B, po ${afterB.files} / ${afterB.bytes} B`);
    const errs = x.hubB?.errors ?? [];
    add("B: konzole bez chyb", errs.length ? FAIL : PASS, errs.length ? errs.slice(0, 3).map((e) => `${e.page}: ${e.text}`).join(" | ") : "0 chyb");
    add("B: start nové (do /status)", PASS, `${b.ms} ms`);

    // ---------- C: druhý start nové ----------
    await cycle(x, "C", x.exe, sb, async ({ page, browser }) => {
      const wd2 = webdataLines(readLog(sb));
      add("C: migrace se neopakuje (žádný nový řádek webdata:)", wd2.lines.length === wd.lines.length ? PASS : FAIL, `řádků ${wd.lines.length} → ${wd2.lines.length}`);
      const m2 = readJson(join(newDir, MARKER));
      add("C: značka beze změny", JSON.stringify(m2) === JSON.stringify(markerSeen) ? PASS : FAIL, JSON.stringify(m2));
      const got = page ? await ls(page) : null;
      add("C: localStorage pořád čitelný", got && got.len === value.length && got.head === head ? PASS : FAIL, got ? `${got.len} znaků` : "null");
      if (wroteCookie) { const ck = await cookiePresent(browser, COOKIE); add("C: cookie pořád čitelná", ck === cookieVal ? PASS : FAIL, ck === null ? "chybí" : "shoduje se"); }
    });
    const afterC = tally(listFiles(oldProf, false));
    add("C: stará složka pořád nedotčená", afterC.sig === allOld.sig ? PASS : FAIL, `${afterC.files} souborů / ${afterC.bytes} B`);

    // ---------- D: odklad (profil starého drží zámek) ----------
    const sb2 = makeSandbox(x.base, "upgrade-d", { apiPort: x.opts.apiPort, hotkeys: false, limits: false, desk: false });
    sbs.push(sb2);
    const oldDir2 = join(sb2.local, OLD_FAKE_ID);
    const newDir2 = join(sb2.local, NEW_ID);
    cpSync(oldDir, oldDir2, { recursive: true });
    const lock = join(oldDir2, "EBWebView", "lockfile");
    holder = holdLock(lock);
    for (let i = 0; i < 40 && !lockedNow(lock); i++) await sleep(250);
    if (!lockedNow(lock)) return add("D: zámek profilu staré složky", SKIP, "držák zámku se nespustil — odklad nelze spolehlivě nasimulovat");
    pointData(sb2, NEW_ID);
    const d1 = await cycle(x, "D1", x.exe, sb2, async ({ page, hub }) => {
      const wdD = webdataLines(readLog(sb2));
      add("D1: nová počkala na zámek a odložila (log)", wdD.lines.some((l) => /odkládá/.test(l)) ? PASS : FAIL, `${wdD.lines.map((l) => l.slice(0, 160)).join(" | ") || "žádný řádek webdata:"} (zámek během běhu: ${lockedNow(lock) ? "drží" : "NEDRŽÍ"})`);
      add("D1: vznikl .migrate-deferred a není značka dokončení", existsSync(join(newDir2, DEFERRED)) && !existsSync(join(newDir2, MARKER)) ? PASS : FAIL, readdirSync(newDir2).join(", "));
      const got = page ? await ls(page) : null;
      add("D1: nová jede s čistým profilem (localStorage prázdný)", page && got === null ? PASS : FAIL, got ? "hodnota je čitelná — profil nebyl čistý" : "null");
      if (page) add("D1: snímek obsahu okna", PASS, await hub.shot(page, "upgrade-D1"));
    }, { statusMs: 60_000 });
    add("D1: start nové s čekáním na zámek (do /status)", d1.ms !== null && d1.ms >= 14_000 ? PASS : FAIL, `${d1.ms} ms (čekání ≤ 15 s + start)`);
    holder.kill();
    holder = null;
    for (let i = 0; i < 40 && lockedNow(lock); i++) await sleep(250);
    await cycle(x, "D2", x.exe, sb2, async ({ page, browser }) => {
      const wdD = webdataLines(readLog(sb2));
      const done = wdD.lines.some((l) => /zkopírován/.test(l));
      add("D2: další start bez zámku dokončil migraci (log)", done ? PASS : FAIL, wdD.lines.slice(-1).map((l) => l.slice(0, 160)).join(""));
      const clean = existsSync(newDir2) ? readdirSync(newDir2).filter((n) => n.startsWith(CLEAN_PREFIX)) : [];
      add("D2: značka dokončení, bez odkladu, čistý profil odsunutý (ne smazaný)", existsSync(join(newDir2, MARKER)) && !existsSync(join(newDir2, DEFERRED)) && clean.length === 1 ? PASS : FAIL, readdirSync(newDir2).join(", "));
      const got = page ? await ls(page) : null;
      add("D2: localStorage ze staré složky je čitelný", got && got.len === value.length && got.head === head ? PASS : FAIL, got ? `${got.len} znaků` : "null");
      if (wroteCookie) { const ck = await cookiePresent(browser, COOKIE); add("D2: cookie čitelná", ck === cookieVal ? PASS : FAIL, ck === null ? "chybí" : "shoduje se"); }
    });

    // ---------- D3: zámek jako skutečný Chromium (sdílení jen pro čtení) ----------
    // Změřeno naživo: živý WebView2 drží `lockfile` se sdílením pro čtení, takže otevření ke čtení projde a `webdata.rs` ho nepozná.
    const sb3 = makeSandbox(x.base, "upgrade-d3", { apiPort: x.opts.apiPort, hotkeys: false, limits: false, desk: false });
    sbs.push(sb3);
    const oldDir3 = join(sb3.local, OLD_FAKE_ID);
    cpSync(oldDir, oldDir3, { recursive: true });
    const lock3 = join(oldDir3, "EBWebView", "lockfile");
    holder = holdLock(lock3, 180, "chromium");
    for (let i = 0; i < 40 && !lockedNow(lock3); i++) await sleep(250);
    if (!lockedNow(lock3)) return add("D3: zámek jako Chromium", SKIP, "držák zámku se nespustil");
    let readOk = false;
    try { closeSync(openSync(lock3, "r")); readOk = true; } catch { /* čtení zamčeno */ }
    pointData(sb3, NEW_ID);
    const d3 = await cycle(x, "D3", x.exe, sb3, async () => {
      const wd3 = webdataLines(readLog(sb3));
      add("D3: nová pozná zámek stylu Chromium a odloží (log)", wd3.lines.some((l) => /odkládá/.test(l)) ? PASS : FAIL, `${wd3.lines.map((l) => l.slice(0, 120)).join(" | ") || "žádný řádek"}; čtení zámku ${readOk ? "PROŠLO (webdata.rs:148 otevírá jen ke čtení, a proto zámek nevidí)" : "bylo zamčené"}`);
    }, { statusMs: 60_000 });
    add("D3: start nové (do /status)", PASS, `${d3.ms} ms`);
  } finally {
    holder?.kill();
    for (const sb of sbs) rmJunction(join(sb.cfg, "webview2")); // dřív, než se sandbox smaže — ať rmSync nemůže zasáhnout cíl
  }
}
