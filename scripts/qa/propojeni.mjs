// QA scénář `propojeni` (kolo 35): přejmenované napojení poznámek na cizí MCP server (`connector`, „Propojená aplikace“).
// Stav „nenastaveno“ (prázdný config) a „nastaveno“ (název + adresa lokálního falešného serveru, bez klíče). Klíč se NIKDY
// nezapisuje: izolovaná instance nezapisuje do Správce pověření (`accounts/vault.rs`), čtení ale míří do skutečného → `hasKey`
// se jen čte a stav „s klíčem“ se neověřuje. Falešný server je jen 127.0.0.1 (port přidělí systém) a počítá požadavky:
// klik na „Poslat do …“ bez klíče nesmí poslat nic.
import { createServer } from "node:http";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { PASS, FAIL, SKIP, rawKeys } from "./lib.mjs";
import { sleep } from "./run.mjs";

const invoke = (page, cmd, args) => page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args ?? {}]);
const bodyText = (page) => page.evaluate(() => document.body.innerText);

/** Falešný MCP server: jen loopback, port přidělí systém; odpovídá minimální platnou odpovědí JSON-RPC a počítá požadavky. */
export function fakeServer() {
  const hits = [];
  const srv = createServer((req, res) => {
    let body = "";
    req.on("data", (d) => { body += d; });
    req.on("end", () => {
      let id = null;
      try { id = JSON.parse(body).id ?? null; } catch { /* ne JSON */ }
      hits.push(req.method);
      res.setHeader("Content-Type", "application/json");
      res.end(JSON.stringify({ jsonrpc: "2.0", id, result: { content: [{ type: "text", text: "[]" }] } }));
    });
  });
  return new Promise((ok) => srv.listen(0, "127.0.0.1", () => ok({ hits, port: srv.address().port, close: () => new Promise((r) => srv.close(() => r())) })));
}

/** Přepíše `connector` v configu sandboxu a nechá instanci config znovu načíst. */
async function setConnector(c, conn) {
  const f = join(c.sb.cfg, "config.json");
  const cfg = JSON.parse(readFileSync(f, "utf8"));
  if (conn) cfg.connector = conn; else delete cfg.connector;
  writeFileSync(f, JSON.stringify(cfg, null, 2));
  await c.api.post("/ctl/reload").catch(() => {});
  await sleep(600);
}

/** Otevře první poznámku v šuplíku a vrátí stránku s otevřeným editorem (nebo null). */
async function openNote(c) {
  await c.api.post("/ctl/open", { view: "full" });
  const page = await c.hub.find("index");
  if (!page) return null;
  await page.waitForSelector(".note-open", { timeout: 10_000 }).catch(() => {});
  await page.locator(".note-open").first().click({ timeout: 5000 }).catch(() => {});
  await page.waitForSelector(".note-top", { timeout: 8000 }).catch(() => {});
  return page;
}

/** Klik na ⋯ v editoru poznámky; vrátí text menu. */
async function openMenu(page) {
  await page.locator('.note-top button[aria-haspopup="menu"]').first().click({ timeout: 5000 }).catch(() => {});
  await page.waitForSelector('[role="menu"]', { timeout: 4000 }).catch(() => {});
  return page.locator('[role="menu"]').first().innerText().catch(() => "");
}

async function menuStep(c, S, state, want, capWant) {
  const page = await openNote(c);
  if (!page) return c.add(S, `${state}: okno šuplíku`, FAIL, "WebView šuplíku se přes CDP nenašlo");
  const txt = await openMenu(page);
  const flat = txt.replace(/\s+/g, " ");
  c.add(S, `${state}: položka „${want}“ v menu ⋯ poznámky`, flat.includes(want) ? PASS : FAIL, `menu: ${flat.slice(0, 200)}`);
  c.add(S, `${state}: záhlaví skupiny „${capWant}“`, flat.includes(capWant) ? PASS : FAIL, `menu: ${flat.slice(0, 120)}`);
  c.add(S, `${state}: snímek menu poznámky`, PASS, await c.hub.shot(page, `propojeni-menu-${state}`));
  return page;
}

export async function propojeni(c) {
  const S = "propojeni";
  const srv = await fakeServer();
  try {
    // ---------- nenastaveno ----------
    await setConnector(c, null);
    await c.api.post("/ctl/settings");
    const set = await c.hub.find("settings", 15_000);
    if (!set) return c.add(S, "okno Nastavení (settings.html)", FAIL, "WebView Nastavení se přes CDP nenašlo");
    await set.waitForSelector(".stu-spine .stu-station", { timeout: 10_000 }).catch(() => {});
    const ids = ["connector.key", "connector.url", "connector.name"];
    const found = async () => Object.fromEntries(await Promise.all(ids.map(async (id) => [id, (await set.locator(`[id="f-${id}"]`).count()) > 0])));
    // část Claude: stanice podle textu (názvy částí nejsou stabilní API), jinak průchod všemi
    const n = await set.locator(".stu-spine .stu-station").count();
    let f = await found();
    for (let i = 0; i < n && !Object.values(f).every(Boolean); i++) {
      await set.locator(".stu-spine .stu-station").nth(i).click({ timeout: 4000 }).catch(() => {});
      await sleep(400);
      f = await found();
      const adv = set.locator("summary, button").filter({ hasText: /Adresa a název propojené aplikace|address and name/i }).first();
      if (!f["connector.url"] && (await adv.count())) { await adv.click({ timeout: 2000 }).catch(() => {}); await sleep(300); f = await found(); }
    }
    await set.locator('[id="f-connector.key"]').scrollIntoViewIfNeeded({ timeout: 3000 }).catch(() => {});
    await sleep(300);
    const txt = await bodyText(set);
    const inDom = await set.evaluate(() => /Propojená aplikace|Linked app/.test(document.body.textContent ?? ""));
    const seen = /Propojená aplikace|Linked app/i.test(txt);
    const keyVis = await set.locator('[id="f-connector.key"]').isVisible().catch(() => false);
    c.add(S, "nenastaveno: sekce „Propojená aplikace“ v Nastavení (viditelná)", seen && keyVis ? PASS : FAIL, `text v DOM: ${inDom}, viditelný text: ${seen}, pole klíče viditelné: ${keyVis}`);
    for (const id of ids) c.add(S, `nenastaveno: pole #f-${id}`, f[id] ? PASS : FAIL, f[id] ? "v DOM" : "kotva není v DOM (sbalené / jiná část?)");
    c.add(S, "nenastaveno: snímek Nastavení se sekcí", PASS, await c.hub.shot(set, "propojeni-nastaveni"));
    const raw = rawKeys(txt);
    c.add(S, "nenastaveno: Nastavení bez syrových klíčů", raw.length ? FAIL : PASS, raw.slice(0, 4).join(", ") || "čisté");

    const drawer = await c.hub.find("index");
    const st0 = drawer ? await invoke(drawer, "connector_state").catch((e) => ({ err: String(e) })) : null;
    if (!st0 || st0.err) c.add(S, "nenastaveno: connector_state", FAIL, st0?.err ?? "okno šuplíku nenalezeno");
    else {
      c.add(S, "nenastaveno: connector_state configured=false, name='', host=''", st0.configured === false && st0.name === "" && st0.host === "" ? PASS : FAIL, JSON.stringify(st0));
      c.add(S, "nenastaveno: connector_state hasKey=false", st0.hasKey === false ? PASS : SKIP, st0.hasKey === false ? "hasKey = false" : "ve skutečném Správci pověření (čtení) je klíč Ledge/connector — nezávisí na sandboxu");
    }
    const page0 = await menuStep(c, S, "nenastaveno", "Poslat do propojené aplikace", "Propojená aplikace");
    if (page0) {
      await page0.getByText("Poslat do propojené aplikace").first().click({ timeout: 4000 }).catch(() => {});
      await sleep(900);
      const t = (await bodyText(page0)).replace(/\s+/g, " ");
      c.add(S, "nenastaveno: klik ukáže hlášku s cestou do Nastavení", /Nastavení → Claude/.test(t) ? PASS : FAIL, `hláška v DOM: ${/Nastavení → Claude/.test(t)}`);
      c.add(S, "nenastaveno: klik neotevřel dialog odeslání", (await page0.locator(".send-dialog, [role='dialog']").count()) === 0 ? PASS : FAIL, "bez dialogu");
      await sleep(600);
      const hit = await set.evaluate(() => [...document.querySelectorAll(".hit")].map((e) => e.id).join(",")).catch(() => "?");
      c.add(S, "nenastaveno: klik vede v Nastavení k poli adresy (zvýrazněno .hit)", hit.includes("connector.url") ? PASS : FAIL, `zvýrazněné kotvy v Nastavení: ${hit || "žádné"}`);
      c.add(S, "nenastaveno: snímek Nastavení po kliku", PASS, await c.hub.shot(set, "propojeni-klik-nenastaveno"));
    }
    c.add(S, "nenastaveno: na falešný server nic nedošlo", srv.hits.length === 0 ? PASS : FAIL, `požadavků: ${srv.hits.length}`);

    // ---------- nastaveno ----------
    await c.api.post("/ctl/close").catch(() => {});
    await setConnector(c, { url: `http://127.0.0.1:${srv.port}/mcp`, appUrl: "", name: "Demo" });
    const d2 = await c.hub.find("index");
    const st1 = d2 ? await invoke(d2, "connector_state").catch((e) => ({ err: String(e) })) : null;
    if (!st1 || st1.err) c.add(S, "nastaveno: connector_state", FAIL, st1?.err ?? "okno šuplíku nenalezeno");
    else c.add(S, "nastaveno: connector_state configured=true, name=Demo, host=127.0.0.1:port", st1.configured === true && st1.name === "Demo" && st1.host === `127.0.0.1:${srv.port}` ? PASS : FAIL, JSON.stringify(st1));
    const page1 = await menuStep(c, S, "nastaveno", "Poslat do Demo", "Demo");
    if (page1 && st1 && !st1.err) {
      await page1.getByText("Poslat do Demo").first().click({ timeout: 4000 }).catch(() => {});
      await sleep(900);
      const t = (await bodyText(page1)).replace(/\s+/g, " ");
      if (st1.hasKey) c.add(S, "nastaveno: klik s klíčem otevře náhled", SKIP, "ve Správci pověření je skutečný klíč; odeslání na falešný server by ho poslalo — neověřeno");
      else c.add(S, "nastaveno: bez klíče hláška vede k poli klíče", /klíč propojené aplikace/.test(t) ? PASS : FAIL, `hláška v DOM: ${/klíč propojené aplikace/.test(t)}`);
      await sleep(600);
      const hit = await set.evaluate(() => [...document.querySelectorAll(".hit")].map((e) => e.id).join(",")).catch(() => "?");
      if (!st1.hasKey) c.add(S, "nastaveno: klik vede v Nastavení k poli klíče (zvýrazněno .hit)", hit.includes("connector.key") ? PASS : FAIL, `zvýrazněné kotvy v Nastavení: ${hit || "žádné"}`);
      c.add(S, "nastaveno: snímek Nastavení po kliku", PASS, await c.hub.shot(set, "propojeni-klik-nastaveno"));
    }
    c.add(S, "nastaveno: bez klíče na falešný server nic nedošlo", st1?.hasKey ? SKIP : srv.hits.length === 0 ? PASS : FAIL, `požadavků: ${srv.hits.length}`);
    c.note("propojeni: klíč se nezapisuje (izolovaná instance nezapisuje do Správce pověření), proto se neověřuje náhled ani odeslání na falešný server");
  } finally {
    await setConnector(c, null).catch(() => {});
    await c.api.post("/ctl/close").catch(() => {});
    await srv.close();
  }
}
