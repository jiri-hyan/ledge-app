// Guard kostry Nastavení (kolo 21): každý klíč configu (odvozený z Rust struktur — `scripts/config-keys.mjs`,
// tedy z téhož, z čeho vzniká `config.schema.json`) má v Nastavení místo — stránku v některé sekci a úroveň
// Základní / Pokročilé (`placement.ts` `PLACEMENT`) —, nebo je výslovně v `NO_EDITOR` s důvodem. Žádná
// zastaralá položka. A úroveň Pokročilé v `placement.ts` sedí s tím, kde panel volbu opravdu kreslí
// (uvnitř `<Advanced>`), jinak by štítek ve výsledcích hledání lhal a režim Základní by ji skryl / neskryl špatně.
//   node --experimental-strip-types --test src/settings/placement.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { keyPaths, readSources } from "../../scripts/config-keys.mjs";
import { ADVANCED_ANCHORS, PLACEMENT, isAdvancedField, placeOf } from "./placement.ts";
import { PANEL_META } from "./registry.ts";
import { SECTIONS } from "./structure.ts";
import { ADVANCED } from "./appearanceModel.ts";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
const KEYS = keyPaths(readSources(ROOT).structs);
const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");

/** Klíče bez editoru v Nastavení — vědomě, s důvodem (zbytečná výjimka shodí guard). Platí i pro vnořené. */
const NO_EDITOR = {
  $schema: "odkaz na schéma pro editory JSON, ne volba",
  "strip.itemsRev": "značka migrace položek proužku; zapisuje ji jádro, ručně se nemění",
  "layout.upgrades": "jednorázové úpravy rozložení po aktualizaci; zapisuje je jádro",
  "ui.tips": "odklepnuté tipy a prohlídka; zapisuje je šuplík sám (smazáním se tip ukáže znovu)",
  "onboarding.done": "průvodce prvním spuštěním ho zapíše po dokončení; znovu spustit jde tlačítkem v Jazyk a průvodce",
  "gitAlerts.read": "„přečteno“ u upozornění na neuloženou práci; zapisuje ho čip v hlavičce šuplíku",
};

/** Je klíč (nebo jeho předek) vědomě bez editoru? → důvod, jinak null. */
function noEditor(path) {
  for (const [k, why] of Object.entries(NO_EDITOR)) if (path === k || path.startsWith(`${k}.`)) return why;
  return null;
}

test("každý klíč configu má místo v Nastavení (stránka v sekci + úroveň), nebo je v NO_EDITOR s důvodem", () => {
  assert.ok(KEYS.length > 200, `klíče configu se načetly (${KEYS.length})`);
  const inSection = new Set(SECTIONS.flatMap((s) => s.pages));
  const pages = new Set(PANEL_META.map((p) => p.id));
  const bad = [];
  for (const k of KEYS) {
    if (noEditor(k)) continue;
    const p = placeOf(k);
    if (!p) bad.push(`${k}: nemá místo (doplň PLACEMENT, nebo NO_EDITOR s důvodem)`);
    else if (!pages.has(p.page) || !inSection.has(p.page)) bad.push(`${k}: stránka ${p.page} není v registru / v žádné sekci`);
  }
  assert.deepEqual(bad, []);
});

test("NO_EDITOR a PLACEMENT nemají zastaralé ani zbytečné položky", () => {
  const set = new Set(KEYS);
  for (const [k, why] of Object.entries(NO_EDITOR)) {
    assert.ok(set.has(k), `NO_EDITOR ${k}: takový klíč config nemá`);
    assert.ok(why.trim().length > 10, `NO_EDITOR ${k}: chybí důvod`);
  }
  const seen = new Set();
  for (const [prefix, page, level] of PLACEMENT) {
    assert.ok(!seen.has(prefix), `PLACEMENT ${prefix} dvakrát`);
    seen.add(prefix);
    assert.ok(KEYS.some((k) => k === prefix || k.startsWith(`${prefix}.`)), `PLACEMENT ${prefix}: žádný klíč configu`);
    assert.ok(["basic", "advanced"].includes(level), `PLACEMENT ${prefix}: úroveň ${level}`);
    assert.ok(PANEL_META.some((p) => p.id === page), `PLACEMENT ${prefix}: stránka ${page}`);
    // vnořený prefix musí něco měnit (jinou stránku nebo úroveň), jinak je zbytečný
    const parent = PLACEMENT.filter(([q]) => q !== prefix && prefix.startsWith(`${q}.`)).sort((a, b) => b[0].length - a[0].length)[0];
    if (parent) assert.ok(parent[1] !== page || parent[2] !== level, `PLACEMENT ${prefix}: stejné jako ${parent[0]}`);
  }
});

test("placeOf: nejdelší prefix, vnořené cesty, neznámé = null", () => {
  const cases = [
    ["drawer.trigger", "chovani", "basic"],
    ["drawer.glass", "vzhled", "advanced"],
    ["drawer.widthX", "chovani", "basic"],
    ["claude.port", "claude", "advanced"],
    ["claude.enabled", "claude", "basic"],
    ["smart.incidents.everyMin", "claude", "basic"],
    ["smart.tabWheel", "chovani", "basic"],
    ["desk.snap", "naplose", "basic"],
    ["accounts.google.clientId", "osobni", "advanced"],
    ["projects.3.port", "akce", "basic"],
    ["personal.links.0.target", "akce", "basic"],
    ["strip.look", "vzhled", "basic"],
    ["strip.items", "vzhled", "basic"],
    ["strip.opacity", "vzhled", "advanced"],
    ["notes.folderLink.path", "poznamky", "basic"],
    ["notes.folderLink.propagateDeletes", "poznamky", "advanced"],
    ["drawer.hoverExpandMs", "vzhled", "advanced"],
    ["drawer.keepWarmS", "spousteni", "basic"],
  ];
  for (const [path, page, level] of cases) assert.deepEqual(placeOf(path), { page, level }, path);
  for (const x of ["", null, undefined, "neznamo", "drawerx"]) assert.equal(placeOf(x), null, String(x));
  assert.ok(noEditor("ui.tips"));
  assert.equal(noEditor("ui.lang"), null);
});

/** Úseky `<Advanced …>…</Advanced>` (JSX jako text, s vnořením). */
function advRegions(text) {
  const out = [];
  const open = /<Advanced[\s>]/g;
  for (let m; (m = open.exec(text)); ) {
    let depth = 1;
    const re = /<Advanced[\s>]|<\/Advanced>/g;
    re.lastIndex = m.index + 1;
    for (let n; (n = re.exec(text)); ) {
      depth += n[0].startsWith("</") ? -1 : 1;
      if (depth === 0) {
        out.push([m.index, n.index]);
        break;
      }
    }
  }
  return out;
}
/** Soubory, které rodič vykresluje CELÉ uvnitř „Pokročilé“ (hlídá i `advanced.test.mjs`). */
const WHOLE_ADV = ["../actions/ScriptsSettings.tsx", "../widgets/WidgetsSettings.tsx", "../accounts/AccountsSettings.tsx", "panels/AppearanceLayout.tsx"];
/** Soubory, kde pokročilá část začíná exportem `…Advanced` (AppPanel ji dá do „Pokročilé“). */
const ADV_FROM = { "panels/About.tsx": "export function AboutAdvanced", "panels/Backup.tsx": "export function BackupAdvanced" };
const PANEL_SRC = {
  zaciname: ["panels/Start.tsx"],
  suplik: ["panels/Drawer.tsx", "../layout/LayoutEditor.tsx"],
  naplose: ["../desk/DeskSettings.tsx"],
  vzhled: ["panels/AppearanceClosed.tsx", "panels/AppearanceHover.tsx", "panels/AppearanceOpen.tsx", "panels/AppearanceStyles.tsx", "panels/AppearanceThemes.tsx", "panels/AppearancePages.tsx", "panels/AppearanceLayout.tsx", "panels/AppearanceMotion.tsx", "panels/StripSettings.tsx"],
  akce: ["panels/Projects.tsx", "panels/Actions.tsx", "../actions/ScriptsSettings.tsx", "panels/LegacyLinks.tsx", "../widgets/WidgetsSettings.tsx"],
  claude: ["panels/Claude.tsx", "panels/Questions.tsx", "../mcp/McpSettings.tsx"],
  osobni: ["panels/Personal.tsx", "../media/MediaSettings.tsx", "../suggest/SuggestSettings.tsx", "../cliphist/ClipSettings.tsx", "../calendar/CalendarSettings.tsx", "../integrations/IntegrationsSettings.tsx", "../accounts/AccountsSettings.tsx"],
  poznamky: ["panels/Notes.tsx", "../notes/FolderLinkSection.tsx"],
  chovani: ["panels/Behavior.tsx", "panels/Hotkeys.tsx"],
  spousteni: ["panels/Startup.tsx"],
  aplikace: ["panels/Backup.tsx", "panels/About.tsx"],
};
const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

test("úroveň Pokročilé sedí s tím, kde panel volbu kreslí", () => {
  const fields = [...read("labels.ts").matchAll(/^\s*"?([\w.]+)"?: \{ panel: "(\w+)"(.*)$/gm)].map((m) => ({ id: m[1], panel: m[2], anchor: m[3].match(/anchor: "([\w.]+)"/)?.[1] ?? m[1] }));
  assert.ok(fields.length > 100);
  const bad = [];
  let checked = 0;
  for (const f of fields) {
    const files = PANEL_SRC[f.panel] ?? [];
    let actual = null;
    for (const file of files) {
      const text = read(file);
      const m = new RegExp(`\\b(id|anchor)="(f-)?${esc(f.anchor)}"`).exec(text);
      if (!m) continue;
      const from = ADV_FROM[file] ? text.indexOf(ADV_FROM[file]) : -1;
      actual = WHOLE_ADV.includes(file) || (from >= 0 && m.index > from) || advRegions(text).some(([a, b]) => m.index > a && m.index < b);
      break;
    }
    if (actual === null) continue; // kotvu kreslí komponenta mimo mapu (settings.test.mjs hlídá, že existuje)
    checked++;
    const claimed = f.panel === "vzhled" ? ADVANCED.includes(f.anchor) : isAdvancedField(f.id);
    if (claimed !== actual) bad.push(`${f.id}: placement říká ${claimed ? "Pokročilé" : "Základní"}, panel kreslí ${actual ? "v Pokročilé" : "mezi základními"}`);
  }
  assert.ok(checked > 100, `ověřeno ${checked} voleb`);
  assert.deepEqual(bad, []);
  for (const a of ADVANCED_ANCHORS) assert.ok(fields.some((f) => f.id === a), `ADVANCED_ANCHORS ${a} je v labels.ts`);
});
