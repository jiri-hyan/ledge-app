// Showcase Ledge (kolo 24): snímky a GIFy ze SKUTEČNÉHO UI vyrenderovaného headless s ukázkovými daty
// (`index.html?demo=1`, `settings.html?demo=1`) na vlastní tapetě (`stage.html`). Žádná fotka obrazovky, žádný ledge.exe.
// Jedním příkazem a idempotentně:  node scripts/showcase/shoot.mjs [--only id,id] [--list] [--no-pack]
// Vyžaduje playwright-core (devDependency; jinak PLAYWRIGHT_CORE=<složka>) a nainstalovaný Edge (`channel: "msedge"`).
// Fronta:  node %USERPROFILE%\.devtools\fronta.mjs --auto -- node scripts/showcase/shoot.mjs
import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, readdirSync, copyFileSync, statSync } from "node:fs";
import { tmpdir, homedir } from "node:os";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { SHOTS } from "./shots.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const only = args.includes("--only") ? args[args.indexOf("--only") + 1].split(",") : null;
if (args.includes("--list")) { console.log(SHOTS.map((s) => `${s.id}\t${s.kind}`).join("\n")); process.exit(0); }

const require = createRequire(join(ROOT, "package.json"));
function loadPlaywright() {
  const tries = [process.env.PLAYWRIGHT_CORE, ""].filter((x) => x !== undefined);
  for (const dir of tries) {
    try { return require(dir ? join(dir, "playwright-core") : "playwright-core"); } catch { /* další */ }
  }
  throw new Error("playwright-core nenalezen: pnpm install, nebo PLAYWRIGHT_CORE=<složka s node_modules>");
}
const { chromium } = loadPlaywright();

const TMP = process.env.SHOWCASE_TMP || join(tmpdir(), "ledge-showcase");
rmSync(TMP, { recursive: true, force: true });
mkdirSync(TMP, { recursive: true });
const FFMPEG = join(process.env.LOCALAPPDATA || join(homedir(), "AppData", "Local"), "ms-playwright", "ffmpeg-1013", "ffmpeg-win64.exe");
const PY = process.env.PYTHON || "python";
const run = (cmd, a, opt = {}) => {
  const r = spawnSync(cmd, a, { encoding: "utf8", windowsHide: true, ...opt });
  if (r.status !== 0) throw new Error(`${cmd} ${a.slice(0, 3).join(" ")} → ${r.status}\n${(r.stderr || "").slice(-600)}`);
  return r;
};
const pack = (...a) => run(PY, ["-I", join(ROOT, "scripts", "showcase", "pack.py"), ...a.map(String)]);
const outPath = (rel) => { const p = join(ROOT, rel); mkdirSync(dirname(p), { recursive: true }); return p; };
const kb = (p) => `${Math.round(statSync(p).size / 1024)} kB`;

// --- vite dev server na volném portu (jen pro tuhle běh) ---
const { createServer } = await import("vite");
const server = await createServer({ root: ROOT, logLevel: "error", clearScreen: false, server: { port: 14240, strictPort: false, host: "127.0.0.1", hmr: false, watch: null } });
await server.listen();
const BASE = `http://127.0.0.1:${server.config.server.port}`;
const stage = (qs) => `${BASE}/scripts/showcase/stage.html?${qs}`;

const waitReady = async (page, ms = 1100) => {
  await page.waitForFunction(() => {
    const fr = [...document.querySelectorAll("iframe")];
    return fr.length > 0 && fr.every((f) => f.dataset.ready === "1" || f.classList.contains("sx-studio"));
  }, null, { timeout: 30_000 }).catch(() => {});
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(ms);
};
const fit = async (page, vw) => {
  const h = await page.evaluate(() => {
    const els = [...document.querySelectorAll(".sx-drawer, .sx-studio, .sx-cell, .sx-half")];
    return Math.ceil(Math.max(0, ...els.map((e) => e.getBoundingClientRect().bottom)));
  });
  await page.setViewportSize({ width: vw, height: h + 40 });
  await page.waitForTimeout(150);
};

let browser, ctxStill;
const done = [];
try {
  browser = await chromium.launch({ channel: "msedge", headless: true });
  for (const s of SHOTS) {
    if (only && !only.includes(s.id)) continue;
    console.log(`> ${s.id}`);
    if (s.kind === "still") {
      const ctx = await browser.newContext({ viewport: { width: s.vw, height: s.vh ?? 800 }, deviceScaleFactor: 2, colorScheme: "dark" });
      const page = await ctx.newPage();
      await page.goto(stage(s.url));
      await waitReady(page);
      if (s.prep) { await s.prep(page); await page.waitForTimeout(500); }
      if (s.fit !== false && !s.vh) await fit(page, s.vw);
      const raw = join(TMP, `${s.id}.png`);
      await page.screenshot({ path: raw });
      await ctx.close();
      const out = outPath(s.out);
      pack("png", raw, out, s.width ?? 1600, s.colors ?? 192);
      done.push(`${s.out}  ${kb(out)}`);
      if (s.web) { const w = outPath(s.web); pack("webp", raw, w, s.webWidth ?? 1400, 82); done.push(`${s.web}  ${kb(w)}`); }
    } else if (s.kind === "page") {
      // hotová statická stránka webu (og.png): žádná scéna, jen snímek
      const ctx = await browser.newContext({ viewport: { width: s.vw, height: s.vh }, deviceScaleFactor: 1, colorScheme: "dark" });
      const page = await ctx.newPage();
      await page.goto(`${BASE}${s.url}`);
      await page.evaluate(() => Promise.all([document.fonts.ready, ...[...document.images].map((i) => i.decode().catch(() => {}))]));
      await page.waitForTimeout(400);
      const raw = join(TMP, `${s.id}.png`);
      await page.screenshot({ path: raw });
      await ctx.close();
      const out = outPath(s.out);
      pack("png", raw, out, s.vw, 256);
      done.push(`${s.out}  ${kb(out)}`);
    } else {
      const dir = join(TMP, s.id);
      mkdirSync(dir, { recursive: true });
      const ctx = await browser.newContext({ viewport: { width: s.vw, height: s.vh }, deviceScaleFactor: 1, colorScheme: "dark", recordVideo: { dir, size: { width: s.vw, height: s.vh } } });
      const page = await ctx.newPage();
      await page.goto(stage(s.url));
      if (s.closed) await page.waitForTimeout(1600); else await waitReady(page, 600);
      await s.script(page);
      const video = page.video();
      await ctx.close();
      const webm = await video.path();
      const frames = join(dir, "frames");
      mkdirSync(frames, { recursive: true });
      run(FFMPEG, ["-y", "-loglevel", "error", "-ss", String(s.trim ?? 0.4), "-i", webm, "-r", String(s.fps ?? 14), join(frames, "f-%04d.png")]);
      if (s.poster) { // poslední snímek = poster videa na webu
        const last = readdirSync(frames).sort().pop();
        const w = outPath(s.poster);
        pack("webp", join(frames, last), w, s.width ?? 900, 80);
        done.push(`${s.poster}  ${kb(w)}`);
      }
      if (s.gif) { const g = outPath(s.gif); pack("gif", frames, g, s.fps ?? 14, s.width ?? 900, s.colors ?? 128); done.push(`${s.gif}  ${kb(g)}`); }
      if (s.webm) {
        const w = outPath(s.webm);
        run(FFMPEG, ["-y", "-loglevel", "error", "-ss", String(s.trim ?? 0.4), "-i", webm, "-c:v", "libvpx-vp9", "-b:v", "0", "-crf", "36", "-pix_fmt", "yuv420p", w]);
        done.push(`${s.webm}  ${kb(w)}`);
      }
    }
  }
} finally {
  await browser?.close();
  await server.close();
}
console.log("\nHotovo:\n" + done.join("\n"));
