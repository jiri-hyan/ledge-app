// Testy Nastavení bez DOM: chyba validace → panel (`panelForField`) a katalog hledání (`labels.ts`) proti
// panelům — každá volba z hledání má kotvu `f-<id>` právě v JEDNOM panelu, a to ve svém (žádný mrtvý
// odkaz, žádná volba nastavitelná na dvou místech). Panely se čtou jako text (JSX bez transpilace).
//   node --experimental-strip-types --test src/settings/settings.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { panelForField } from "./model.ts";

const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");

/** panel id → soubory, které ho vykreslují */
const PANEL_FILES = {
  projekty: ["panels/Projects.tsx", "panels/ProjectEditor.tsx"],
  suplik: ["panels/Drawer.tsx"],
  vzhled: ["panels/Appearance.tsx"],
  claude: ["panels/Claude.tsx"],
  poznamky: ["panels/Notes.tsx"],
  osobni: ["panels/Personal.tsx"],
  zkratky: ["panels/Hotkeys.tsx"],
  zaloha: ["panels/Backup.tsx"],
  oaplikaci: ["panels/About.tsx"],
  akce: ["panels/Actions.tsx", "../actions/ScriptsSettings.tsx"],
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

test("sjednocené volby: pozadí, šířka a animace jen ve Vzhledu, starý klíč nikde", () => {
  const f = fields();
  assert.equal(f["drawer.glass"].panel, "vzhled");
  assert.equal(f["drawer.width"].panel, "vzhled");
  assert.equal(f["appearance.motion"].panel, "vzhled");
  assert.equal(f["drawer.animationMs"], undefined);
  for (const file of ALL_FILES) assert.ok(!src[file].includes("animationMs"), `${file} nepoužívá drawer.animationMs`);
  const drawer = src["panels/Drawer.tsx"];
  for (const k of ["drawer.glass", "drawer.width", "appearance."]) assert.ok(!drawer.includes(`"${k}`), `Šuplík a proužek needituje ${k}`);
});

test("jazyk rozhraní: volba ve Vzhledu, hledání ji najde česky i anglicky", () => {
  assert.equal(fields()["ui.lang"].panel, "vzhled");
  const line = read("labels.ts").split("\n").find((l) => l.includes('"ui.lang"'));
  for (const w of ["jazyk", "language"]) assert.ok(line.includes(w), `kw obsahuje ${w}`);
  assert.ok(/settings\.f\.ui\.lang\.kw": "[^"]*language/.test(read("en.ts")), "anglická klíčová slova");
});

test("chyba pole → panel (nejdelší prefix vyhrává)", () => {
  const cases = [
    ["drawer.trigger", "suplik"],
    ["drawer.barWindows.pushMs", "suplik"],
    ["drawer.keepWarmS", "suplik"],
    ["drawer.glass", "vzhled"],
    ["drawer.width", "vzhled"],
    ["drawer.widthX", "suplik"],
    ["strip.items", "suplik"],
    ["strip.opacity", "vzhled"],
    ["appearance.motion.speed", "vzhled"],
    ["projects.2.port", "projekty"],
    ["autostart", "suplik"],
    ["ui.lang", "vzhled"],
    ["neznamo.pole", null],
    ["", null],
    [null, null],
  ];
  for (const [field, want] of cases) assert.equal(panelForField(field), want, String(field));
});
