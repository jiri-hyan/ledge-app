// Náhled barevných povrchů karet (kolo 28): kontaktní arch PNG z CSS povrchů (`src/layout/surfaces.css`) na maketě
// karty (nadpis, řádky, velké číslo dole) — ŽÁDNÁ data, žádné okno (headless Edge), žádný ledge.exe.
// Spuštění (fronta):  node "$USERPROFILE/.devtools/fronta.mjs" --auto -- node scripts/showcase/surfaces.mjs [--out složka]
// Vyžaduje playwright-core (PLAYWRIGHT_CORE=<složka s node_modules>) a Edge.
import { createRequire } from "node:module";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { PALETTES, PALETTE_COLORS, paletteStyle } from "../../src/layout/surfaces.ts";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const OUT = resolve(args.includes("--out") ? args[args.indexOf("--out") + 1] : join(ROOT, "shots-surfaces"));
mkdirSync(OUT, { recursive: true });
const require = createRequire(join(ROOT, "package.json"));
const loadPw = () => {
  for (const dir of [process.env.PLAYWRIGHT_CORE, ""].filter((x) => x !== undefined)) {
    try { return require(dir ? join(dir, "playwright-core") : "playwright-core"); } catch { /* další */ }
  }
  throw new Error("playwright-core nenalezen: PLAYWRIGHT_CORE=<složka s node_modules>");
};
const { chromium } = loadPw();
const css = ["src/styles.css", "src/layout/layout.css", "src/layout/surfaces.css"].map((f) => readFileSync(join(ROOT, f), "utf8")).join("\n");

const ACCENTS = { fialová: "#a78bfa", jantar: "#fbbf24", zelená: "#4ade80" };
const SURFACES = ["gradient", "flow", "edge", "stripes"];
const card = (surface, palette, accent) => {
  const st = Object.entries({ "--accent": accent, ...paletteStyle(surface, palette === "card" ? undefined : palette) }).map(([k, v]) => `${k}:${v}`).join(";");
  return `<div class="deck-item" data-card="demo" data-cl-surface="${surface}" style="${st};width:220px;height:150px"><div class="card" style="height:100%">
    <h4><span class="ic"></span>Ukázka</h4><div style="font-size:12px;color:var(--t2);margin-top:6px">Řádek jedna<br><span style="color:var(--t3)">Řádek dva</span></div>
    <div style="position:absolute;left:14px;bottom:10px"><div style="font-size:34px;font-weight:700;color:var(--t1);line-height:1">42 %</div><div style="font-size:11px;color:var(--t3)">popisek</div></div></div></div>`;
};
let browser;
try {
  browser = await chromium.launch({ channel: "msedge", headless: true });
  const page = await (await browser.newContext({ viewport: { width: 1500, height: 800 }, colorScheme: "dark", reducedMotion: "reduce" })).newPage();
  const rows = (accent) => SURFACES.map((s) => `<div style="display:flex;gap:10px;margin-bottom:10px;align-items:center"><b style="width:70px;color:#cfd3e0;font:12px sans-serif">${s}</b>${PALETTES.map((p) => card(s, p, accent)).join("")}</div>`).join("");
  for (const [name, accent] of Object.entries(ACCENTS)) {
    await page.setContent(`<style>${css}</style><body class="theme-scope" style="margin:0;padding:12px;background:#14161e;width:1500px"><div style="color:#cfd3e0;font:12px sans-serif;margin:0 0 6px 80px">${PALETTES.join("  ·  ")} — akcent ${name}</div>${rows(accent)}</body>`);
    await page.waitForTimeout(200);
    const file = join(OUT, `surfaces-${name}.png`);
    writeFileSync(file, await page.screenshot({ fullPage: true }));
    console.log("  →", file);
  }
} finally { await browser?.close(); }
