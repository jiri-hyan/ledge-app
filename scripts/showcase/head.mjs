// Přehlídka hlavičky šuplíku (kolo 27): styly hodin, štítků a záložek ve SKUTEČNÉM UI s ukázkovými daty
// (`index.html?demo=1`, parametry `head=<json>` a `tabs=<styl>` v `src/showcase/demo.ts`). Žádné okno (headless Edge),
// žádný ledge.exe. Spuštění (fronta): PLAYWRIGHT_CORE=<složka s node_modules> node scripts/showcase/head.mjs [--out složka]
// Výstup: PNG hlavičky (oříznutá horní část šuplíku) v šířce notche 720 px a v široké 1100 px.
import { createRequire } from "node:module";
import { mkdirSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const OUT = resolve(args.includes("--out") ? args[args.indexOf("--out") + 1] : join(ROOT, "shots-hlavicka"));
mkdirSync(OUT, { recursive: true });
const require = createRequire(join(ROOT, "package.json"));
const { chromium } = (() => {
  for (const dir of [process.env.PLAYWRIGHT_CORE, ""].filter((x) => x !== undefined)) {
    try { return require(dir ? join(dir, "playwright-core") : "playwright-core"); } catch { /* další */ }
  }
  throw new Error("playwright-core nenalezen: PLAYWRIGHT_CORE=<složka s node_modules>");
})();

const { createServer } = await import("vite");
const server = await createServer({ root: ROOT, logLevel: "error", clearScreen: false, server: { port: 14270, strictPort: false, host: "127.0.0.1", hmr: false, watch: null } });
await server.listen();
const BASE = `http://127.0.0.1:${server.config.server.port}`;
const DAY = new Date(); DAY.setHours(20, 2, 0, 0);
let browser;
try {
  browser = await chromium.launch({ channel: "msedge", headless: true });
  const ctx = await browser.newContext({ viewport: { width: 800, height: 700 }, deviceScaleFactor: 1, colorScheme: "dark", reducedMotion: "reduce" });
  const page = await ctx.newPage();
  page.on("pageerror", (e) => console.log("  ! pageerror:", String(e).slice(0, 200)));
  const shot = async (name, w, head, tabs, extra = {}) => {
    const q = new URLSearchParams({ demo: 1, open: 1, set: "studio", full: 1, head: JSON.stringify(head), ...(tabs ? { tabs } : {}), ...extra });
    await page.setViewportSize({ width: w, height: 420 });
    await page.clock.setFixedTime(DAY);
    await page.goto(`${BASE}/index.html?${q}`);
    await page.waitForSelector(".head .clock", { timeout: 20_000 }).catch(() => {});
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(900);
    const box = await page.locator(".head").boundingBox();
    const clip = { x: 0, y: Math.max(0, box.y - 12), width: w, height: Math.round(box.height + 40) };
    await page.screenshot({ path: join(OUT, `${name}-${w}.png`), clip, animations: "disabled" });
    const over = await page.evaluate(() => {
      const r = document.querySelector(".hl-chips"), t = document.querySelector(".lay-tabs");
      return r && t && r.getBoundingClientRect().right > t.getBoundingClientRect().left + 0.5 ? "PŘEKRÝVÁ ZÁLOŽKY" : "";
    });
    if (over) console.log("  !", name, w, over);
    const wraps = await page.evaluate(() => {
      const r = document.querySelector(".hl-chips");
      if (!r) return "bez štítků";
      const ys = new Set([...r.children].filter((c) => c.offsetParent).map((c) => Math.round(c.getBoundingClientRect().top)));
      return `${ys.size} řada, styl ${r.dataset.style}/${r.dataset.shrink}, ${r.querySelectorAll(".hl-hide").length} schováno`;
    });
    console.log(`  ${name}-${w}: ${wraps}`);
  };
  for (const w of [720, 1100]) {
    for (const clock of ["big", "line", "seconds", "analog", "time"]) await shot(`clock-${clock}`, w, { clock, chips: "pills" });
    await shot("chips-auto", w, { clock: "big", chips: "auto", showZero: true });
    await shot("chips-auto6", w, { clock: "big", chips: "auto", showZero: true }, undefined, { morechips: 1 });
    for (const chips of ["pills", "merged", "dots", "icons"]) await shot(`chips-${chips}`, w, { clock: "line", chips, showZero: true });
    await shot("chips-zero-hidden", w, { clock: "line", chips: "pills" });
    for (const tabs of ["segment", "underline", "pills", "icons", "dots"]) await shot(`tabs-${tabs}`, w, { clock: "line", chips: "pills" }, tabs);
    await shot("custom-order", w, { clock: "time", chips: "icons", order: ["tabs", "clock", "chips"], hidden: ["note"] }, "underline");
  }
} finally {
  await browser?.close();
  await server.close();
}
