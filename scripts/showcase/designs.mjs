// Přehlídka vzhledů karet a pozadí šuplíku (kolo 25): kontaktní archy PNG ze SKUTEČNÉHO UI s ukázkovými daty
// (`index.html?demo=1`, parametry v `src/showcase/demo.ts`). Žádné okno (headless Edge), žádný ledge.exe, čas pevně 14:30.
// Spuštění (fronta):  node "$USERPROFILE/.devtools/fronta.mjs" --auto -- node scripts/showcase/designs.mjs [--only cards|backdrops|empty] [--out složka]
// Vyžaduje playwright-core (PLAYWRIGHT_CORE=<složka s node_modules>) a Edge. Výstup: shots-k25/*.png (necommitovat).
import { createRequire } from "node:module";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { CARD_DESIGNS } from "../../src/layout/designs.ts";
import { PRESETS, LIVE_PRESETS } from "../../src/theme/backdropPresets.ts";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const opt = (k) => (args.includes(k) ? args[args.indexOf(k) + 1] : null);
const only = opt("--only");
const OUT = resolve(opt("--out") ?? join(ROOT, "shots-k25"));
mkdirSync(OUT, { recursive: true });

const require = createRequire(join(ROOT, "package.json"));
const loadPw = () => {
  for (const dir of [process.env.PLAYWRIGHT_CORE, ""].filter((x) => x !== undefined)) {
    try { return require(dir ? join(dir, "playwright-core") : "playwright-core"); } catch { /* další */ }
  }
  throw new Error("playwright-core nenalezen: PLAYWRIGHT_CORE=<složka s node_modules>");
};
const { chromium } = loadPw();

const WIDTHS = [300, 460, 680];
const DAY = new Date(); DAY.setHours(14, 30, 0, 0);
const NIGHT = new Date(); NIGHT.setHours(23, 20, 0, 0);
const CARDS = { claude: "Claude", system: "Systém", weather: "Počasí", week: "Týden" };

const { createServer } = await import("vite");
const server = await createServer({ root: ROOT, logLevel: "error", clearScreen: false, server: { port: 14260, strictPort: false, host: "127.0.0.1", hmr: false, watch: null } });
await server.listen();
const BASE = `http://127.0.0.1:${server.config.server.port}`;

let browser;
try {
  browser = await chromium.launch({ channel: "msedge", headless: true });
  const ctx = await browser.newContext({ viewport: { width: 800, height: 900 }, deviceScaleFactor: 1, colorScheme: "dark", reducedMotion: "reduce" });
  const url = (p) => `${BASE}/index.html?demo=1&open=1&set=studio&page=prace&nopending=1&${new URLSearchParams(p)}`;

  /** Otevře ukázkový šuplík a počká, až je karta vykreslená. */
  async function open(page, params, w, h, time = DAY) {
    await page.setViewportSize({ width: w, height: h });
    await page.clock.setFixedTime(time);
    await page.goto(url(params));
    await page.waitForSelector(".card h4", { timeout: 20_000 });
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(1300);
    // grafy se kreslí až s IntersectionObserverem: počkej, ať je vidět kreslení, nebo to nahlas
    await page.waitForFunction(() => [...document.querySelectorAll(".chart-gauge, .chart-rings")].every((g) => g.querySelector("svg")), null, { timeout: 5000 }).catch(() => console.log("  ! graf se nevykreslil:", params.cards));
  }
  const page = await ctx.newPage();
  page.on("pageerror", (e) => console.log("  ! pageerror:", String(e).slice(0, 200)));

  // rozdíl šířky okna a karty (odsazení šuplíku) — jednou, z první karty
  await open(page, { cards: "claude:xl", full: 1 }, 800, 900);
  const delta = 800 - (await page.evaluate(() => document.querySelector(".card").getBoundingClientRect().width));
  console.log(`odsazení okna → karta: ${delta}px`);

  const shotCard = async (params, w, time) => {
    await open(page, { ...params }, w + Math.round(delta), 1400, time);
    const el = page.locator(".card").first();
    const box = await el.boundingBox();
    const png = await el.screenshot({ animations: "disabled" });
    const over = await page.evaluate(() => {
      const c = document.querySelector(".card");
      return [...c.querySelectorAll("*")].filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).overflow === "visible" && e.clientWidth > 0).slice(0, 3).map((e) => `${e.tagName}.${String(e.className).slice(0, 24)}`);
    });
    return { png, w: Math.round(box.width), h: Math.round(box.height), over };
  };

  /** Archy: HTML s <img> v mřížce → jeden PNG; dělí se na části do `maxH` px výšky. */
  const sheetPage = await ctx.newPage();
  async function sheet(name, title, rows, cols, { width = 2000, maxH = 2000, cell = null } = {}) {
    // rows: [{label, cells:[{png?, note?}]}]
    const parts = [];
    let cur = [], curH = 0;
    for (const r of rows) {
      const h = Math.max(...r.cells.map((c) => (c.h ?? 120) * (c.scale ?? 1))) + 34;
      if (cur.length && curH + h > maxH - 40) { parts.push(cur); cur = []; curH = 0; }
      cur.push(r); curH += h;
    }
    if (cur.length) parts.push(cur);
    let i = 0;
    for (const part of parts) {
      i++;
      const html = `<body style="margin:0;background:#12141c;color:#cfd3e0;font:12px Segoe UI,sans-serif;width:${width}px">
        <div style="padding:6px 10px;font-weight:700">${title}${parts.length > 1 ? ` (${i}/${parts.length})` : ""}</div>
        <div style="display:grid;grid-template-columns:96px ${cols.map((c) => `${c}fr`).join(" ")};gap:6px 8px;padding:0 10px 10px;align-items:start">
        <div></div>${cols.map((c, k) => `<div style="opacity:.6">${cell?.[k] ?? ""}</div>`).join("")}
        ${part.map((r) => `<div style="padding-top:4px;font-weight:600;word-break:break-word">${r.label}</div>${r.cells.map((c) => `<div>${c.png ? `<img src="data:image/png;base64,${c.png.toString("base64")}" style="display:block;max-width:100%;${c.scale ? `width:${Math.round(c.w * c.scale)}px;` : ""}outline:1px solid #2c3040"/><div style="opacity:.55;margin-top:2px">${c.w}×${c.h}${c.over?.length ? ` · přetéká: ${c.over.join(", ")}` : ""}</div>` : `<i style="opacity:.5">${c.note ?? ""}</i>`}</div>`).join("")}`).join("")}
        </div></body>`;
      await sheetPage.setViewportSize({ width, height: 400 });
      await sheetPage.setContent(html);
      await sheetPage.waitForTimeout(150);
      const file = join(OUT, `${name}${parts.length > 1 ? `-${i}` : ""}.png`);
      writeFileSync(file, await sheetPage.screenshot({ fullPage: true }));
      console.log("  →", file);
    }
  }

  // ---------- karty: vzhledy × 3 šířky (a prázdné stavy) ----------
  const runCards = async (empty) => {
    for (const [cardId, label] of Object.entries(CARDS)) {
      const designs = CARD_DESIGNS[cardId];
      const variants = designs.map((d) => ({ d: d.id, wx: "day", tag: d.id }));
      if (cardId === "weather" && !empty) variants.push({ d: "sky", wx: "night", tag: "sky · noc", time: NIGHT }, { d: "sky", wx: "rain", tag: "sky · déšť" });
      const rows = [];
      for (const v of variants) {
        const cells = [];
        for (const w of WIDTHS) {
          const params = { cards: `${cardId}:xl:${v.d}`, full: 1, wx: v.wx, ...(empty ? { empty: 1 } : {}) };
          cells.push(await shotCard(params, w, v.time));
        }
        console.log(`  ${cardId}/${v.tag}${empty ? " (prázdný)" : ""}: ${cells.map((c) => `${c.w}×${c.h}`).join(" ")}${cells.some((c) => c.over.length) ? " PŘETÉKÁ" : ""}`);
        rows.push({ label: `${label}<br>${v.tag}`, cells });
      }
      await sheet(`${empty ? "empty" : "cards"}-${cardId}`, `${label}${empty ? " — prázdné stavy" : ""}: vzhledy × šířka karty`, rows, WIDTHS, { cell: WIDTHS.map((w) => `~${w} px`) });
    }
  };
  if (!only || only === "cards") await runCards(false);
  if (!only || only === "empty") await runCards(true);

  // ---------- pozadí šuplíku s kartami ----------
  if (!only || only === "backdrops") {
    const shotDrawer = async (params, w = 1080) => {
      await open(page, { cards: "claude:l,projects:l,weather:m,notes:m", full: 1, theme: "oled", material: "solid", ...params }, w, 760);
      const png = await page.screenshot({ animations: "disabled" });
      return { png, w, h: 760, scale: 0.9 };
    };
    const items = [...PRESETS.map((p) => ({ tag: p.id, p: { bd: p.id } })), ...LIVE_PRESETS.map((l) => ({ tag: `live ${l.id}`, p: { live: l.id, bd: l.base } }))];
    const rows = [];
    for (let i = 0; i < items.length; i += 2) {
      const pair = items.slice(i, i + 2);
      const cells = [];
      for (const it of pair) cells.push(await shotDrawer(it.p));
      rows.push({ label: pair.map((x) => x.tag).join("<br>"), cells });
      console.log(`  pozadí ${pair.map((x) => x.tag).join(", ")}`);
    }
    await sheet("backdrops-oled", "Pozadí šuplíku, motiv oled, materiál plné, karty auto", rows, [1, 1], { cell: ["", ""] });
    // rozložení float a notch, režimy karet
    const extra = [
      { tag: "float · sunset · auto", p: { layout: "float", bd: "sunset" } },
      { tag: "notch · sunset · auto", p: { layout: "notch", bd: "sunset" } },
      { tag: "sunset · solid", p: { bd: "sunset", bdcards: "solid" } },
      { tag: "sunset · tint", p: { bd: "sunset", bdcards: "tint" } },
      { tag: "sunset · glass", p: { bd: "sunset", bdcards: "glass" } },
      { tag: "bez pozadí (beze změny)", p: {} },
      { tag: "světlý motiv · aurora", p: { bd: "aurora", theme: "light" } },
      { tag: "živé rain · float", p: { live: "rain", bd: "midnight", layout: "float" } },
    ];
    const rows2 = [];
    for (let i = 0; i < extra.length; i += 2) {
      const pair = extra.slice(i, i + 2);
      const cells = [];
      for (const it of pair) cells.push(await shotDrawer(it.p, it.p.layout === "notch" ? 1200 : 1080));
      rows2.push({ label: pair.map((x) => x.tag).join("<br>"), cells });
      console.log(`  varianty ${pair.map((x) => x.tag).join(", ")}`);
    }
    await sheet("backdrops-modes", "Karty nad pozadím: režimy, rozložení float / notch", rows2, [1, 1], { cell: ["", ""] });
  }
} finally {
  await browser?.close();
  await server.close();
}
console.log("Hotovo.");
