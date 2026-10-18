// Testy Nastavení bez DOM: chyba validace → panel (`panelForField`), katalog hledání (`labels.ts`) proti
// panelům — každá volba z hledání má kotvu `f-<id>` právě v JEDNOM panelu, a to ve svém (žádný mrtvý
// odkaz, žádná volba nastavitelná na dvou místech) —, složení panelů z importů, skoky odjinud
// (`gotoSettingsPanel`, i staré id panelů) a hledání starých názvů. Panely se čtou jako text (JSX bez
// transpilace).
//   node --experimental-strip-types --test src/settings/settings.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { panelForField, resolveTarget, search } from "./model.ts";
import { PANEL_ALIASES, PANEL_META } from "./registry.ts";

const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");

/** panel id → soubory, které ho vykreslují (že je panel opravdu skládá, hlídá test „složení panelů“) */
const PANEL_FILES = {
  zaciname: ["panels/Start.tsx"],
  suplik: ["panels/Drawer.tsx", "../layout/LayoutEditor.tsx"],
  vzhled: ["panels/Appearance.tsx", "panels/AppearanceStyles.tsx", "panels/AppearanceLayout.tsx", "panels/AppearanceMotion.tsx", "panels/StripSettings.tsx"],
  akce: ["panels/ProjectsActions.tsx", "panels/Projects.tsx", "panels/ProjectEditor.tsx", "panels/Actions.tsx", "../actions/ScriptsSettings.tsx", "panels/LegacyLinks.tsx"],
  claude: ["panels/Claude.tsx", "panels/Questions.tsx"],
  osobni: ["panels/Personal.tsx", "../media/MediaSettings.tsx", "../cliphist/ClipSettings.tsx", "../calendar/CalendarSettings.tsx"],
  poznamky: ["panels/Notes.tsx"],
  chovani: ["panels/Behavior.tsx", "panels/Hotkeys.tsx", "../pkgctx/BubbleNotice.tsx"],
  aplikace: ["panels/AppPanel.tsx", "panels/Backup.tsx", "panels/About.tsx"],
};
const ALL_FILES = [...new Set(Object.values(PANEL_FILES).flat())];
const src = Object.fromEntries(ALL_FILES.map((f) => [f, read(f)]));

/** `FIELDS` z labels.ts: id → { panel, anchor } (řádky `"id": { panel: "…", … anchor: "…" }`). */
function fields() {
  const out = {};
  for (const m of read("labels.ts").matchAll(/^\s*"?([\w.]+)"?: \{ panel: "(\w+)"(.*)$/gm)) {
    out[m[1]] = { panel: m[2], anchor: m[3].match(/anchor: "([\w.]+)"/)?.[1] ?? m[1] };
  }
  return out;
}

const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
/** Deklaruje soubor kotvu `id`? (`Row/Section/PanelHead id="…"`, `id="f-…"`, `anchor="…"`, šablona `id={`pre.${…}`}`) */
function declares(text, id) {
  if (new RegExp(`\\b(id|anchor)="(f-)?${esc(id)}"`).test(text)) return true;
  for (const m of text.matchAll(/\bid=\{`([\w.-]+)\$\{/g)) if (id.startsWith(m[1].replace(/^f-/, ""))) return true;
  return false;
}

test("katalog: každá volba má kotvu ve svém panelu a v žádném jiném", () => {
  const f = fields();
  assert.ok(Object.keys(f).length > 40, "labels.ts se načetl");
  const bad = [];
  for (const [id, { panel, anchor }] of Object.entries(f)) {
    const own = PANEL_FILES[panel];
    if (!own) {
      bad.push(`${id}: neznámý panel ${panel}`);
      continue;
    }
    const where = ALL_FILES.filter((file) => declares(src[file], anchor));
    if (!where.some((file) => own.includes(file))) bad.push(`${id}: kotva f-${anchor} chybí v ${own.join(" / ")}`);
    const foreign = where.filter((file) => !own.includes(file));
    if (foreign.length) bad.push(`${id}: kotva f-${anchor} i v cizím panelu ${foreign.join(", ")}`);
  }
  assert.deepEqual(bad, []);
});

test("sjednocené volby: vzhled i celý proužek jen ve Vzhledu, chování jen v Chování", () => {
  const f = fields();
  for (const id of ["drawer.glass", "drawer.width", "appearance.motion", "appearance.strip", "strip.enabled", "strip.items", "strip.custom", "strip.monitor", "timer.strip"]) assert.equal(f[id].panel, "vzhled", id);
  for (const id of ["drawer.trigger", "drawer.keepWarmS", "drawer.barWindows", "autostart", "hotkeys.enabled", "palette.compact", "gameMode"]) assert.equal(f[id].panel, "chovani", id);
  for (const id of ["clipboard.enabled", "timer.presets", "media.variant", "personal.weather"]) assert.equal(f[id].panel, "osobni", id);
  assert.equal(f["claude.questionHoldS"].panel, "claude");
  assert.equal(f["drawer.animationMs"], undefined);
  assert.equal(f["personal.links"], undefined, "odkazy jsou akce — jedna volba „Akce a odkazy“");
  assert.equal(f.actions.panel, "akce");
  for (const file of ALL_FILES) assert.ok(!src[file].includes("animationMs"), `${file} nepoužívá drawer.animationMs`);
  const drawer = src["panels/Drawer.tsx"];
  for (const k of ["drawer.", "strip.", "appearance.", "autostart", "clipboard"]) assert.ok(!drawer.includes(`"${k}`), `Šuplík a karty needituje ${k}`);
});

test("složení panelů: každý soubor z mapy panel opravdu vykresluje (importy), jiný panel ne", () => {
  const cat = read("catalog.ts");
  const imp = Object.fromEntries([...cat.matchAll(/^import (\w+) from "\.\/(panels\/\w+)";$/gm)].map((m) => [m[1], `${m[2]}.tsx`]));
  const roots = Object.fromEntries([...cat.matchAll(/^\s+(\w+): (\w+Panel),$/gm)].map((m) => [m[1], imp[m[2]]]));
  assert.deepEqual(Object.keys(roots).sort(), PANEL_META.map((m) => m.id).sort(), "každý panel z registru má komponentu");
  const href = (f) => new URL(f, import.meta.url).href;
  const byHref = new Map(ALL_FILES.map((f) => [href(f), f]));
  /** soubory z mapy dosažitelné importy z kořene panelu */
  const reach = (root) => {
    const seen = new Set([root]);
    const todo = [root];
    while (todo.length) {
      const f = todo.pop();
      for (const m of src[f].matchAll(/from "(\.{1,2}\/[^"]+)"/g)) {
        const k = byHref.get(new URL(`${m[1]}.tsx`, href(f)).href);
        if (k && !seen.has(k)) {
          seen.add(k);
          todo.push(k);
        }
      }
    }
    return seen;
  };
  const bad = [];
  for (const [id, files] of Object.entries(PANEL_FILES)) {
    assert.ok(roots[id], `panel ${id} je v katalogu`);
    const r = reach(roots[id]);
    for (const f of files) if (!r.has(f)) bad.push(`${id}: ${f} se z ${roots[id]} nevykreslí`);
    for (const [other, root] of Object.entries(roots)) {
      if (other === id) continue;
      const o = reach(root);
      for (const f of files) if (o.has(f)) bad.push(`${f} vykresluje i panel ${other}`);
    }
  }
  assert.deepEqual(bad, []);
});

const SRC = fileURLToPath(new URL("..", import.meta.url));
/** Všechny skoky do Nastavení v kódu: `gotoSettingsPanel("…")`, `onSettings("…")`, cíle tipů (`go: "…"`). */
function gotoTargets() {
  const out = [];
  const walk = (dir) => {
    for (const n of readdirSync(dir)) {
      const p = join(dir, n);
      if (statSync(p).isDirectory()) walk(p);
      else if (/\.tsx?$/.test(n) && !n.endsWith(".d.ts")) {
        const text = readFileSync(p, "utf8");
        for (const m of text.matchAll(/(?:gotoSettingsPanel|onSettings)\("([^"]+)"\)|\bgo: "([^"]+)"/g)) out.push({ file: relative(SRC, p), id: m[1] ?? m[2] });
      }
    }
  };
  walk(SRC);
  return out;
}

test("skoky odjinud: každý cíl vede na existující panel a jeho řádek je v tom panelu", () => {
  const ids = PANEL_META.map((m) => m.id);
  const f = fields();
  const targets = gotoTargets();
  // karty, paleta, hlavička, Začínáme — ať test nezačne potichu procházet naprázdno
  for (const want of ["f-actions.add", "akce", "f-personal.calendars", "f-personal.weather", "f-autostart", "f-clipboard.enabled", "f-claude.hooks", "f-claude.statusLine", "f-appearance.style"]) {
    assert.ok(targets.some((x) => x.id === want), `cíl ${want} se v kódu najde`);
  }
  const bad = [];
  for (const { file, id } of targets) {
    const to = resolveTarget(id, ids, PANEL_ALIASES, (x) => f[x]?.panel ?? panelForField(x));
    if (!to.panel) {
      bad.push(`${file}: ${id} → žádný panel`);
      continue;
    }
    if (to.anchor && !PANEL_FILES[to.panel].some((pf) => declares(src[pf], to.anchor))) bad.push(`${file}: ${id} → kotva chybí v panelu ${to.panel}`);
  }
  assert.deepEqual(bad, []);
});

test("staré id panelů (před vlnou 6) vedou na nové; neznámé nikam", () => {
  const ids = PANEL_META.map((m) => m.id);
  const cases = [
    ["vitejte", "zaciname"],
    ["projekty", "akce"],
    ["akce", "akce"],
    ["otazky", "claude"],
    ["zkratky", "chovani"],
    ["zaloha", "aplikace"],
    ["oaplikaci", "aplikace"],
    ["suplik", "suplik"],
    ["vzhled", "vzhled"],
    ["osobni", "osobni"],
    ["poznamky", "poznamky"],
    ["neco", null],
    ["f-neznamo", null],
  ];
  for (const [id, want] of cases) assert.equal(resolveTarget(id, ids, PANEL_ALIASES, () => null).panel, want, id);
  for (const to of Object.values(PANEL_ALIASES)) assert.ok(ids.includes(to), `alias → ${to}`);
  assert.deepEqual(resolveTarget("f-drawer.keepWarmS", ids, PANEL_ALIASES, () => "chovani"), { panel: "chovani", anchor: "drawer.keepWarmS" });
  assert.deepEqual(resolveTarget("f-x", ids, PANEL_ALIASES, () => "smazany-panel"), { panel: null, anchor: "x" });
});

/** Položky hledání jako v `catalog.searchItems` (česky): panely z registru + volby z textu labels.ts. */
function searchCs() {
  const str = (line, k) => line.match(new RegExp(`\\b${k}: "((?:[^"\\\\]|\\\\.)*)"`))?.[1] ?? "";
  const out = PANEL_META.map((p) => ({ panel: p.id, anchor: "", label: p.cs, hint: "", kw: p.kw[1] }));
  for (const l of read("labels.ts").split("\n")) {
    const m = l.match(/^\s*"?([\w.]+)"?: \{ panel: "(\w+)"/);
    if (m) out.push({ panel: m[2], anchor: m[1], label: str(l, "label"), hint: str(l, "hint"), kw: str(l, "kw") });
  }
  return out;
}

test("hledání najde i staré názvy panelů a voleb (bez diakritiky)", () => {
  const items = searchCs();
  const cases = [
    ["okna s taby", "drawer.barWindows"],
    ["uvolnit pamet", "drawer.keepWarmS"],
    ["setrit pamet", "drawer.keepWarmS"],
    ["otazky clauda", "claude.questionHoldS"],
    ["rychle odkazy", "actions"],
    ["rychle akce", "actions"],
    ["pruvodce", "onboarding.wizard"],
    ["spousteni po startu", "autostart"],
    ["monitor", "strip.monitor"],
    ["historie schranky", "clipboard.enabled"],
  ];
  for (const [q, anchor] of cases) assert.ok(search(items, q).some((x) => x.anchor === anchor), `„${q}“ najde ${anchor}`);
  const panelHit = (q, panel) => search(items, q).some((x) => !x.anchor && x.panel === panel);
  for (const [q, panel] of [["zkratky", "chovani"], ["vzhled a animace", "vzhled"], ["zaloha", "aplikace"], ["o aplikaci", "aplikace"], ["otazky", "claude"], ["odkazy", "akce"], ["vitejte", "zaciname"]]) {
    assert.ok(panelHit(q, panel), `„${q}“ najde panel ${panel}`);
  }
  // žádný interní klíč configu v názvu volby
  for (const it of items.filter((x) => x.anchor)) assert.ok(!/\b[a-z]+\.[a-z]+[A-Z]\w*/.test(it.label) && !/keepWarm|barWindows/.test(it.label), `název ${it.anchor}: ${it.label}`);
});

test("jazyk rozhraní: volba ve Vzhledu, hledání ji najde česky i anglicky", () => {
  assert.equal(fields()["ui.lang"].panel, "vzhled");
  const line = read("labels.ts").split("\n").find((l) => l.includes('"ui.lang"'));
  for (const w of ["jazyk", "language"]) assert.ok(line.includes(w), `kw obsahuje ${w}`);
  assert.ok(/settings\.f\.ui\.lang\.kw": "[^"]*language/.test(read("en.ts")), "anglická klíčová slova");
});

test("chyba pole → panel (nejdelší prefix vyhrává)", () => {
  const cases = [
    ["drawer.trigger", "chovani"],
    ["drawer.barWindows.pushMs", "chovani"],
    ["drawer.keepWarmS", "chovani"],
    ["drawer.glass", "vzhled"],
    ["drawer.width", "vzhled"],
    ["drawer.widthX", "chovani"],
    ["strip.items", "vzhled"],
    ["strip.opacity", "vzhled"],
    ["appearance.motion.speed", "vzhled"],
    ["layout.pages.0.cards", "suplik"],
    ["projects.2.port", "akce"],
    ["actions.3.target", "akce"],
    ["personal.links.0.target", "akce"],
    ["personal.weather.lat", "osobni"],
    ["personal.calendars.1.url", "osobni"],
    ["claude.questionHoldS", "claude"],
    ["hotkeys.toggle", "chovani"],
    ["autostart", "chovani"],
    ["update.repo", "aplikace"],
    ["onboarding.done", "zaciname"],
    ["ui.lang", "vzhled"],
    ["neznamo.pole", null],
    ["", null],
    [null, null],
  ];
  for (const [field, want] of cases) assert.equal(panelForField(field), want, String(field));
});
