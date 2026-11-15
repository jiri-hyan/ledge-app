// QA scénář `migrace-id` (kolo 35): přechod složky dat WebView2 ze starého identifikátoru na `app.ledge.desktop`.
// Jde jen o to, co se dá ověřit bez čistého stroje: sandbox má falešné %LOCALAPPDATA% (`sb.local`, env LOCALAPPDATA) a v něm
// atrapu starého profilu (`cz.acme.ledge\EBWebView`, maska starého identifikátoru bez jména). Po startu instance se ověří:
// stará složka je nedotčená (bajt po bajtu), v nové jsou jen soubory „stavu“ (ne cache), po rozdělané kopii nic nezbylo a značka
// dokončení je tam. Zda WebView2 nad zkopírovaným profilem skutečně přečte localStorage a cookies, tady NEJDE (izolace parkuje
// data WebView2 do `<LEDGE_CONFIG_DIR>/webview2`) — viz docs/agents/install.md, ruční zkouška na čistém stroji.
// Názvy odpovídají jádru (`webdata.rs`: `MARKER`, `DEFERRED`, `STAGING`) a jsou VŠECHNY uvnitř nové složky: značka `.migrated-from`,
// odklad `.migrate-deferred`, rozdělaná kopie `EBWebView.migrating`, odsunutý čistý profil `EBWebView.clean-<čas>` (informativní).
// Skutečný přenos a čtení profilu WebView2 nad ním zkouší scénář `upgrade` (`upgrade.mjs`, dvě binárky).
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { PASS, FAIL, SKIP } from "./lib.mjs";

export const NEW_ID = "app.ledge.desktop";
export const OLD_FAKE_ID = "cz.acme.ledge";
export const MARKER = ".migrated-from";
export const DEFERRED = ".migrate-deferred";
export const STAGING = "EBWebView.migrating";
export const CLEAN_PREFIX = "EBWebView.clean-";

/** Atrapa profilu: relativní cesta v `EBWebView` → { text, state } (state = musí se přenést, jinak je to cache a přenést se nesmí). */
export const OLD_PROFILE = {
  "Local State": { text: '{"os_crypt":{"encrypted_key":"atrapa"}}', state: true },
  "Default/Local Storage/leveldb/000003.log": { text: "atrapa localStorage", state: true },
  "Default/Network/Cookies": { text: "atrapa cookies", state: true },
  "Default/Preferences": { text: "{}", state: true },
  "Default/Cache/Cache_Data/data_0": { text: "cache", state: false },
  "Default/Code Cache/js/index": { text: "cache", state: false },
  "GrShaderCache/data_0": { text: "cache", state: false },
};

const at = (root, rel) => join(root, ...rel.split("/"));

/** Založí atrapu starého profilu pod `<local>\cz.acme.ledge\EBWebView`; vrací kořen profilu. */
export function seedOldProfile(local, id = OLD_FAKE_ID) {
  const root = join(local, id, "EBWebView");
  for (const [rel, f] of Object.entries(OLD_PROFILE)) {
    mkdirSync(dirname(at(root, rel)), { recursive: true });
    writeFileSync(at(root, rel), f.text);
  }
  return root;
}

const walk = (dir, base = dir, out = []) => {
  if (!existsSync(dir)) return out;
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) walk(p, base, out); else out.push(relative(base, p).replace(/\\/g, "/"));
  }
  return out;
};
const sha = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");

/** Kroky ověření nad složkou `local`; čistá funkce nad souborovým systémem (bez instance). */
export function migraceKroky(local, S = "migrace-id", oldId = OLD_FAKE_ID) {
  const steps = [];
  const add = (name, status, evidence = "") => steps.push({ scenario: S, name, status, evidence: String(evidence) });
  const oldRoot = join(local, oldId, "EBWebView");
  const newDir = join(local, NEW_ID);
  const newRoot = join(newDir, "EBWebView");
  const changed = Object.entries(OLD_PROFILE).filter(([rel, f]) => !existsSync(at(oldRoot, rel)) || readFileSync(at(oldRoot, rel), "utf8") !== f.text).map(([rel]) => rel);
  add("stará složka dat zůstala nedotčená", changed.length ? FAIL : PASS, changed.length ? `změněno/chybí: ${changed.join(", ")}` : `${Object.keys(OLD_PROFILE).length} souborů shodných s atrapou`);
  if (!existsSync(newRoot) && !existsSync(join(newDir, MARKER))) {
    add("kopie profilu do nové složky", SKIP, `${NEW_ID}\\EBWebView nevznikla — migrace se v izolované instanci nespouští (LEDGE_CONFIG_DIR) nebo tato binárka nemá modul migrace; ověř ručně na čistém stroji (docs/agents/install.md)`);
    return steps;
  }
  const got = new Set(walk(newRoot));
  const state = Object.entries(OLD_PROFILE).filter(([, f]) => f.state).map(([rel]) => rel);
  const missing = state.filter((rel) => !got.has(rel));
  const differ = state.filter((rel) => got.has(rel) && sha(at(newRoot, rel)) !== sha(at(oldRoot, rel)));
  add("stav (Local State, localStorage, cookies) se přenesl beze změny", missing.length || differ.length ? FAIL : PASS, missing.length || differ.length ? `chybí: ${missing.join(", ") || "-"}; jiný obsah: ${differ.join(", ") || "-"}` : "všechny soubory stavu shodné");
  const cache = Object.entries(OLD_PROFILE).filter(([rel, f]) => !f.state && got.has(rel)).map(([rel]) => rel);
  add("cache se nepřenesla", cache.length ? FAIL : PASS, cache.length ? `přenesené cache: ${cache.join(", ")}` : "cache v nové složce není");
  const leftover = existsSync(join(newDir, STAGING));
  add("po dokončení nezbyla rozdělaná kopie", leftover ? FAIL : PASS, leftover ? `${NEW_ID}\${STAGING} existuje` : "žádný zbytek");
  const deferred = existsSync(join(newDir, DEFERRED));
  add("po dokončení není značka odkladu", deferred ? FAIL : PASS, deferred ? `${DEFERRED} zůstala vedle ${MARKER}` : "bez odkladu");
  const clean = readdirSync(newDir).filter((n) => n.startsWith(CLEAN_PREFIX));
  add("odsunutý čistý profil (informativně)", PASS, clean.length ? `${clean.join(", ")} (WebView založil profil dřív než migrace; nic se nemazalo)` : "žádný — migrace běžela před prvním WebView");
  const marker = join(newDir, MARKER);
  add("značka dokončení je zapsaná", existsSync(marker) ? PASS : FAIL, existsSync(marker) ? `${MARKER}: ${readFileSync(marker, "utf8").slice(0, 120)}` : `chybí ${MARKER}`);
  return steps;
}

/** Scénář: instance už běží (migrace proběhla při startu před prvním oknem); jen se přečte souborový systém sandboxu. */
export async function migraceId(c) {
  for (const s of migraceKroky(c.sb.local)) c.addRaw(s);
}
