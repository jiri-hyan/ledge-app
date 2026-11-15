// QA před vydáním (L295, L296): scénáře. Každý dostane kontext `c` (api, hub, sb, add/ok/skip) a zapisuje kroky.
// Kde selektory nejsou stabilní, hledá se podle textu / role a řekne se to do `c.note` („po cestě“ ve zprávě).
// Řízení jde přes CDP do obsahu WebView a přes lokální API — žádná myš ani klávesnice systému, žádný fokus.
import { PASS, FAIL, SKIP, checkField, settleStep, field, hasHotkey, rawKeys, viewIs, PASSTHROUGH_KEY, PIN_NOTE } from "./lib.mjs";
import { startPlayer, killTree, sleep } from "./run.mjs";
import { propojeni } from "./propojeni.mjs";
import { migraceId } from "./migrace.mjs";

const count = (v) => (Array.isArray(v) ? v.length : Number(v) || 0);
const invoke = (page, cmd, args) => page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args ?? {}]);
const bodyText = (page) => page.evaluate(() => document.body.innerText);

/** Kontrola syrových klíčů v těle stránky → krok. */
async function noRawKeys(c, scen, name, page) {
  const raw = rawKeys(await bodyText(page));
  c.add(scen, name, raw.length ? FAIL : PASS, raw.length ? `syrové klíče: ${raw.slice(0, 5).join(", ")}` : "žádný syrový překladový klíč");
}

export async function supik(c) {
  const S = "supik";
  await c.api.post("/ctl/open", { view: "full" });
  const st = await c.api.waitStatus((s) => viewIs(s, "full"));
  c.add(S, "/ctl/open full → drawer.view", st ? PASS : FAIL, st ? `drawer.view = ${field(st, "drawer.view")}` : "pohled se neotevřel do 8 s");
  const page = await c.hub.find("index");
  if (!page) return c.add(S, "okno šuplíku (index.html)", FAIL, "WebView šuplíku se přes CDP nenašlo");
  await page.waitForSelector("[data-card]", { timeout: 8000 }).catch(() => {});
  const cards = await page.$$eval("[data-card]", (els) => els.map((e) => e.getAttribute("data-card")));
  c.add(S, "karty vykreslené", cards.length ? PASS : FAIL, `${cards.length} karet: ${cards.slice(0, 8).join(", ")}`);
  c.add(S, "snímek full", PASS, await c.hub.shot(page, "supik-full"));
  c.addRaw(settleStep(S, c.hub.lastSettle));
  await noRawKeys(c, S, "žádný syrový překladový klíč (full)", page);
  const banner = await page.$(".banner");
  c.add(S, "žádný banner chyb", banner ? FAIL : PASS, banner ? `.banner: ${(await banner.innerText()).slice(0, 120)}` : "bez .banner");
  const status = await c.api.status();
  c.addRaw(checkField(S, "qa.cfgError == false", status, "qa.cfgError", (v) => v === false || v === null || v === "", "false"));
  c.addRaw(checkField(S, "qa.down prázdné", status, "qa.down", (v) => count(v) === 0, "[]"));
  // přechod full → peek se v ~1/3 běhů neprovedl (pohled zůstal hidden/full): zkouška pohledů jde vždy ze zavřeného stavu
  await c.api.post("/ctl/close");
  await c.api.waitStatus((x) => viewIs(x, "hidden"));
  for (const view of ["peek", "mid", "island"]) {
    await c.api.post("/ctl/open", { view });
    const s = await c.api.waitStatus((x) => viewIs(x, view));
    c.add(S, `/ctl/open ${view}`, s ? PASS : FAIL, s ? `drawer.view = ${view}` : `pohled ${view} se neotevřel do 8 s (drawer.view = ${field(await c.api.status().catch(() => null), "drawer.view")})`);
    if (s) {
      await sleep(400);
      c.add(S, `snímek ${view}`, PASS, await c.hub.shot(page, `supik-${view}`));
      await noRawKeys(c, S, `žádný syrový klíč (${view})`, page);
    }
    await c.api.post("/ctl/close");
    const h = await c.api.waitStatus((x) => viewIs(x, "hidden"));
    c.add(S, `/ctl/close po ${view}`, h ? PASS : FAIL, `drawer.view = ${field(h ?? await c.api.status().catch(() => null), "drawer.view")}`);
  }
}

/** Lístky (L260): průchozí režim a zkratka. `hotkeys` = config `hotkeys.enabled` této instance. */
export async function listky(c, hotkeys) {
  const S = hotkeys ? "listky-l260-zkratky" : "listky-l260";
  const key = PASSTHROUGH_KEY;
  const reg = (st) => hasHotkey(field(st, "qa.hotkeys"), "notePassthrough", key);
  const hk = (name, st, want) => {
    const v = reg(st);
    c.add(S, name, v === undefined ? SKIP : v === want ? PASS : FAIL, v === undefined ? "pole qa.hotkeys chybí v /status" : `notePassthrough registrována = ${v} (čekáno ${want})`);
  };
  await c.api.post("/ctl/open", { view: "full" });
  await c.api.waitStatus((s) => viewIs(s, "full"));
  const drawer = await c.hub.find("index");
  if (!drawer) return c.add(S, "okno šuplíku", FAIL, "WebView šuplíku se přes CDP nenašlo");
  await c.api.post("/ctl/close");
  const path = c.sb.notePath(PIN_NOTE);
  c.note("lístek se připíná příkazem pin_toggle z okna šuplíku (stejné volání jako klik na špendlík u poznámky), ne klikem v seznamu");
  try { await invoke(drawer, "pin_toggle", { path }); } catch (e) { return c.add(S, "připnutí lístku", FAIL, String(e).slice(0, 160)); }
  let st = await c.api.waitStatus((s) => count(field(s, "qa.pins.open")) === 1 || field(s, "qa.pins.open") === undefined, 10_000);
  if (st && field(st, "qa.pins.open") === undefined) c.add(S, "qa.pins.open == 1", SKIP, "pole qa.pins.open chybí v /status");
  else c.add(S, "qa.pins.open == 1", st ? PASS : FAIL, `qa.pins.open = ${JSON.stringify(field(await c.api.status().catch(() => null), "qa.pins.open"))}`);
  st = await c.api.status();
  hk(hotkeys ? "zkratka registrovaná už s neprůchozím lístkem" : "zkratka NEregistrovaná (zkratky vypnuté, nic průchozího)", st, hotkeys);

  const pin = await c.hub.find("pin");
  if (!pin) return c.add(S, "okno lístku (pin.html)", FAIL, "WebView lístku se přes CDP nenašlo");
  await pin.waitForSelector(".pin", { timeout: 8000 }).catch(() => {});
  c.add(S, "snímek lístku", PASS, await c.hub.shot(pin, `${S}-listek`));
  // menu ⋯ → Průchozí (hledá se podle role a textu, třídy lístku nejsou stabilní API)
  await pin.locator('button[aria-haspopup="menu"]').first().click({ timeout: 5000 }).catch(() => {});
  const item = pin.getByText(/^(Průchozí|Click-through)/).first();
  if (!(await item.isVisible().catch(() => false))) return c.add(S, "položka Průchozí v menu ⋯", FAIL, "položka v menu lístku nenalezena");
  await item.click();
  await pin.waitForSelector(".pin-hint", { timeout: 5000 }).catch(() => {});
  const hint = await pin.$(".pin-hint");
  if (!hint) c.add(S, "nápověda před prvním zapnutím", FAIL, "dialog .pin-hint se neukázal");
  else {
    const txt = await hint.innerText();
    const warn = await hint.$(".warn");
    c.add(S, "nápověda NEvaruje (hintNoKey)", warn ? FAIL : PASS, warn ? `varování: ${txt.slice(0, 140)}` : "bez varování o nefunkční zkratce");
    c.add(S, "nápověda ukazuje zkratku", /F11/.test(txt) ? PASS : FAIL, `text nápovědy: ${txt.replace(/\s+/g, " ").slice(0, 140)}`);
    c.add(S, "snímek nápovědy", PASS, await c.hub.shot(pin, `${S}-napoveda`));
    await pin.locator(".pin-hint .pin-btn.pri").first().click();
  }
  st = await c.api.waitStatus((s) => count(field(s, "qa.pins.through")) === 1, 8000);
  c.add(S, "qa.pins.through == 1", st ? PASS : field(await c.api.status().catch(() => null), "qa.pins.through") === undefined ? SKIP : FAIL, `qa.pins.through = ${JSON.stringify(field(await c.api.status().catch(() => null), "qa.pins.through"))}`);
  hk("zkratka registrovaná v průchozím režimu", await c.api.status(), true);
  c.add(S, "snímek průchozího", PASS, await c.hub.shot(pin, `${S}-pruchozi`));

  c.note("průchozí režim se vypíná z okna šuplíku příkazem pin_through_all (to volá položka palety); zkratku OS vyvolat nelze (žádný SendInput)");
  await invoke(drawer, "pin_through_all").catch((e) => c.add(S, "pin_through_all", FAIL, String(e).slice(0, 160)));
  st = await c.api.waitStatus((s) => count(field(s, "qa.pins.through")) === 0, 8000);
  c.add(S, "qa.pins.through == 0 po vypnutí", st ? PASS : FAIL, `qa.pins.through = ${JSON.stringify(field(await c.api.status().catch(() => null), "qa.pins.through"))}`);
  hk(hotkeys ? "zkratka zůstává (zkratky zapnuté, lístek je na ploše)" : "zkratka zmizela po vypnutí průchozího", await c.api.status(), hotkeys);

  await invoke(drawer, "pin_toggle", { path }).catch(() => {});
  st = await c.api.waitStatus((s) => count(field(s, "qa.pins.open")) === 0, 8000);
  c.add(S, "qa.pins.open == 0 po zavření lístku", st ? PASS : FAIL, `qa.pins.open = ${JSON.stringify(field(await c.api.status().catch(() => null), "qa.pins.open"))}`);
  if (hotkeys) hk("zkratka uvolněna bez lístku na ploše", await c.api.status(), false);
}

/** Kit na ploše: bez dat „—“, s fixture daty čísla. */
export async function kit(c, withData) {
  const S = withData ? "kit-data" : "kit-bez-dat";
  const st = await c.api.waitStatus((s) => count(field(s, "qa.desk.open")) > 0 || field(s, "qa.desk") === undefined, 12_000);
  if (!st) c.add(S, "qa.desk.open > 0", FAIL, "widget na ploše se neotevřel do 12 s");
  else if (field(st, "qa.desk") === undefined) c.add(S, "qa.desk.open > 0", SKIP, "pole qa.desk chybí v /status");
  else c.add(S, "qa.desk.open > 0", PASS, `qa.desk.open = ${JSON.stringify(field(st, "qa.desk.open"))}`);
  const page = await c.hub.find("desk", 8000);
  if (!page) return c.add(S, "okno widgetu (desk.html)", FAIL, "WebView widgetu se přes CDP nenašlo");
  await sleep(1500);
  const text = (await bodyText(page)).replace(/\s+/g, " ").trim();
  c.add(S, "snímek widgetu", PASS, await c.hub.shot(page, S));
  c.add(S, "bez syrových klíčů", rawKeys(text).length ? FAIL : PASS, rawKeys(text).join(", ") || "čisté");
  if (withData) {
    const ok = /\b37\b/.test(text) && /\b12\b/.test(text);
    c.add(S, "čísla z fixture (37 % / 12 %)", ok ? PASS : FAIL, `text widgetu: ${text.slice(0, 160)}`);
  } else {
    // bez dat widget nesmí vymýšlet nuly: ukáže „—“, nebo výzvu k zapojení zdroje (prázdný stav s volbami)
    const dash = text.includes("—") || /Zapojit|Connect/.test(text);
    const zero = /(^|\s)0\s?%/.test(text);
    c.add(S, "bez dat ukazuje „—“ / výzvu, ne nuly", dash && !zero ? PASS : FAIL, `text widgetu: ${text.slice(0, 160)}`);
  }
}

/** Hudba v ostrůvku: tichý přehrávač (GSMTC) → titulek v DOM; po ukončení zmizí. */
export async function hudba(c) {
  const S = "hudba";
  const TITLE = "Ledge QA (ticho)";
  const t0 = Date.now();
  const player = startPlayer(c.sb.dir);
  try {
    // přehrávač musí HRÁT, než se otevře ostrůvek (jinak by test měřil rozjezd přehrávače, ne aplikaci)
    const pl = await player.ready();
    c.add(S, "tichý přehrávač hraje (před otevřením ostrůvku)", pl.ok ? PASS : FAIL, `po ${pl.ms} ms: ${pl.said || "bez výstupu"}`);
    await c.api.post("/ctl/open", { view: "island" });
    await c.api.waitStatus((s) => viewIs(s, "island"));
    // média vlákno běží jen s otevřeným pohledem, který hudbu ukazuje (live == false v klidu je záměr) — proto až po otevření ostrůvku
    const tl = []; // časová osa qa.media (ms od startu přehrávače) — důkaz při nestabilitě
    const st = await c.api.waitStatus((s) => { const m = JSON.stringify(field(s, "qa.media")); if (tl[tl.length - 1]?.m !== m) tl.push({ t: Date.now() - t0, m }); return field(s, "qa.media.playing") === true || field(s, "qa.media") === undefined; }, 20_000);
    c.note(`hudba: časová osa qa.media (ms od startu přehrávače): ${tl.map((x) => `${x.t}=${x.m}`).join(" ")}`);
    if (st && field(st, "qa.media") === undefined) c.add(S, "qa.media.playing == true", SKIP, "pole qa.media chybí v /status");
    else c.add(S, "qa.media.playing == true", st ? PASS : FAIL, `qa.media = ${JSON.stringify(field(await c.api.status().catch(() => null), "qa.media"))}`);
    const page = await c.hub.find("index");
    if (!page) return c.add(S, "okno šuplíku", FAIL, "WebView šuplíku se přes CDP nenašlo");
    const seen = await page.waitForFunction((t) => document.body.innerText.includes(t), TITLE, { timeout: 15_000 }).then(() => true, () => false);
    c.add(S, "titulek přehrávače v ostrůvku", seen ? PASS : FAIL, seen ? `DOM obsahuje „${TITLE}“` : `titulek v DOM není; text: ${(await bodyText(page)).replace(/\s+/g, " ").slice(0, 120)}`);
    c.add(S, "snímek ostrůvku", PASS, await c.hub.shot(page, "hudba-ostruvek"));
    killTree(player.pid);
    const gone = await page.waitForFunction((t) => !document.body.innerText.includes(t), TITLE, { timeout: 20_000 }).then(() => true, () => false);
    c.add(S, "po ukončení přehrávače titulek zmizí", gone ? PASS : FAIL, gone ? "titulek z DOM zmizel" : "titulek v DOM zůstal 20 s po ukončení přehrávače");
    const st2 = await c.api.status().catch(() => null);
    if (field(st2, "qa.media.playing") !== undefined) c.add(S, "qa.media.playing == false po ukončení", field(st2, "qa.media.playing") === false ? PASS : FAIL, `qa.media = ${JSON.stringify(field(st2, "qa.media"))}`);
    await c.api.post("/ctl/close");
  } finally {
    killTree(player.pid);
  }
}

/** Nastavení: projde části (stanice vlevo), bez syrových klíčů; chyby konzole sbírá `konzole`. */
export async function nastaveni(c) {
  const S = "nastaveni";
  await c.api.post("/ctl/settings");
  const page = await c.hub.find("settings", 15_000);
  if (!page) return c.add(S, "okno Nastavení (settings.html)", FAIL, "WebView Nastavení se přes CDP nenašlo");
  await page.waitForSelector(".stu-spine .stu-station", { timeout: 10_000 }).catch(() => {});
  c.note("Nastavení se prochází klikem na stanice `.stu-spine .stu-station` (části Studia), ne podle registru panelů — registr (settings/registry.ts) má jen plochý seznam, okno je Studio s částmi");
  const n = await page.locator(".stu-spine .stu-station").count();
  if (!n) return c.add(S, "části Nastavení", FAIL, "v okně nejsou žádné `.stu-station`");
  const names = [];
  const bad = [];
  for (let i = 0; i < n; i++) {
    const b = page.locator(".stu-spine .stu-station").nth(i);
    const name = (await b.innerText().catch(() => `#${i}`)).replace(/\s+/g, " ").trim().slice(0, 30);
    await b.click({ timeout: 4000 }).catch(() => {});
    await sleep(350);
    names.push(name);
    const raw = rawKeys(await bodyText(page));
    if (raw.length) bad.push(`${name}: ${raw.slice(0, 3).join(", ")}`);
    if (i === 1) c.add(S, "snímek části", PASS, await c.hub.shot(page, "nastaveni-cast"));
  }
  c.add(S, `${n} částí bez syrových klíčů`, bad.length ? FAIL : PASS, bad.length ? bad.join("; ") : names.join(" · "));
}

/** Chyby konzole a neodchycené výjimky ze všech oken instance. */
export function konzole(c, scen) {
  const errs = c.hub.errors;
  const csp = c.hub.csp.map((v) => `CSP ${v.page}: ${v.directive} blokoval ${v.blockedURI || "inline"} @ ${String(v.sourceFile || "").replace(/^https?:\/\/[^/]+/, "")}:${v.line}${v.sample ? ` vzorek „${String(v.sample).slice(0, 80)}“` : ""} ${v.el ? ` prvek ${v.el}` : ""} zápisy stylu: ${(v.writes ?? []).map((w) => `${w.kind}(${w.v}) z ${w.at}`).join(" ; ")}`);
  for (const v of csp.slice(0, 1)) c.note(v);
  const kinds = [...c.hub.kinds].join(", ") || "žádné okno";
  c.add(scen, "console.error a pageerror", errs.length ? FAIL : PASS, errs.length ? errs.slice(0, 3).map((e) => `${e.page}: ${e.text}`).join(" | ") + (errs.length > 3 ? ` (+${errs.length - 3})` : "") + (csp.length ? ` || ${csp.slice(0, 3).join(" | ")}` : "") : `0 chyb v oknech: ${kinds}`);
}

export const RUNNERS = {
  supik,
  "listky-l260": (c) => listky(c, false),
  "listky-l260-zkratky": (c) => listky(c, true),
  "kit-bez-dat": (c) => kit(c, false),
  "kit-data": (c) => kit(c, true),
  hudba,
  nastaveni,
  propojeni,
  "migrace-id": migraceId,
};
