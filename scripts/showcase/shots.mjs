// Seznam snímků a GIFů showcase (kolo 24). `url` = parametry scény `src/showcase/stage.tsx` + ukázkového šuplíku
// (`src/showcase/demo.ts`: set, style, theme, page, cards, ask, nopending, w …). `out` = výstup v repu.
// Pravidlo srovnání: galerie (closed, open, GIF C/E/F) drží JEDNU sadu (studio), aby se měnil jen vzhled.
const S = (id, url, extra = {}) => ({ id, kind: "still", vw: 1440, url, out: `docs/screenshots/${id}.png`, ...extra });
const frame = async (page) => (await page.locator("#main").elementHandle()).contentFrame();
const demo = async (page, fn, ...a) => (await frame(page)).evaluate(([f, args]) => window.__demo[f](...args), [fn, a]);
const wait = (page, ms) => page.waitForTimeout(ms);

const STYLES = ["classic", "glass", "terminal", "paper", "neon", "oled"];
const THEMES = ["dark", "midnight", "nord", "aurora", "warm", "light"];
const V = (id, url, extra = {}) => ({ id, kind: "video", vw: 1280, vh: 640, fps: 12, width: 900, ...extra, url, gif: `docs/media/${id}.gif`, webm: `site/media/${id}.webm` });

export const SHOTS = [
  // --- běžné snímky (střídají sady) ---
  S("drawer", "scene=drawer&set=studio", { web: "site/img/drawer.webp" }),
  S("approve", "scene=drawer&set=studio&cards=claude:xl&w=820&style=glass&theme=midnight", { web: "site/img/approve.webp" }),
  S("question", "scene=drawer&set=studio&cards=claude:xl&w=820&ask=1", { web: "site/img/question.webp" }),
  S("limits", "scene=drawer&set=studio&cards=claude:xl&w=820&nopending=1&theme=graphite"),
  S("personal", "scene=drawer&set=studio&page=osobni", { web: "site/img/personal.webp" }),
  S("notes", "scene=drawer&set=creator&page=prace"),
  S("creator", "scene=drawer&set=creator&page=hudba"),
  S("minimal", "scene=drawer&set=minimal"),
  S("palette", "scene=drawer&set=minimal&style=glass", {
    web: "site/img/palette.webp",
    prep: async (page) => { await (await frame(page)).evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true, bubbles: true }))); await wait(page, 700); },
  }),
  // --- Studio ---
  S("studio-home", "scene=studio", { vw: 1480 }),
  S("studio-strip", "scene=studio&hash=pruzek", { vw: 1480, web: "site/img/studio.webp" }),
  // --- galerie vzhledů ---
  S("looks-closed", "scene=closed&looks=classic,island,dots,line,clock,corner,rings,custom&state=work", { vw: 1500, web: "site/img/looks-closed.webp" }),
  S("looks-open", `scene=open&items=${STYLES.join(",")}&scale=0.46&h=760&cols=3&set=studio&cards=claude,projects`, { vw: 1640, web: "site/img/looks-open.webp" }),
  S("styles-4", "scene=open&items=classic,glass,terminal,paper&scale=0.62&h=640&cols=2&set=studio&cards=claude,projects", { vw: 1480 }),
  S("pair-1", "scene=pair&set=creator&style=glass&theme=aurora&look=island&state=work", { vw: 1500 }),
  S("pair-2", "scene=pair&set=studio&style=terminal&look=bar&state=wait", { vw: 1500 }),
  S("pair-3", "scene=pair&set=minimal&style=paper&look=clock&state=idle", { vw: 1500 }),
  { id: "og", kind: "page", url: "/site/og.html", vw: 1280, vh: 640, out: "site/og.png" },
  // --- GIFy ---
  V("open", "scene=drawer&set=studio&cards=claude,projects&look=island&state=work&overlap=1", {
    closed: true, poster: "site/img/open-poster.webp",
    script: async (page) => {
      await wait(page, 900);
      await page.evaluate(() => { const s = document.querySelector(".sx-strip"); s.style.opacity = "0"; });
      await demo(page, "open", true);
      await wait(page, 3600);
    },
  }),
  V("approve-click", "scene=drawer&set=studio&cards=claude:xl&w=820&open=1", {
    vw: 1000, vh: 540,
    script: async (page) => {
      await wait(page, 1800);
      await (await frame(page)).locator("button", { hasText: "Povolit" }).first().click();
      await wait(page, 2200);
    },
  }),
  V("styles", "scene=drawer&set=studio&cards=claude,projects&w=1080&open=1", {
    script: async (page) => {
      await wait(page, 900);
      for (const id of STYLES.slice(1)) { await demo(page, "style", id); await wait(page, 1250); }
    },
  }),
  V("themes", "scene=drawer&set=studio&cards=claude,projects&w=1080&open=1", {
    script: async (page) => {
      await wait(page, 900);
      for (const id of THEMES.slice(1)) { await demo(page, "appearance", { theme: id }); await wait(page, 1100); }
    },
  }),
  V("strip", "scene=strip&look=classic&state=idle", {
    vw: 1100, vh: 330, width: 800, fps: 10,
    script: async (page) => {
      const set = (v) => page.evaluate((x) => window.__stage.set(x), v);
      for (const look of ["classic", "island", "dots", "line", "clock"]) {
        for (const state of ["idle", "work", "wait"]) { await set({ look, state }); await wait(page, 430); }
      }
    },
  }),
  V("studio-live", "scene=studio&hash=pruzek", {
    vw: 1480, vh: 940, width: 960, fps: 10,
    script: async (page) => {
      const f = await frame(page);
      await wait(page, 700);
      for (const name of ["Jen tečky", "Hodiny", "Rohový odznak", "Ostrůvek"]) {
        await f.locator("button", { hasText: name }).first().click();
        await wait(page, 1100);
      }
    },
  }),
];
