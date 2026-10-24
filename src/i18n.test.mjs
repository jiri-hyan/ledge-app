// Překlady UI (bez DOM): `npm run test:ui`. Čistá logika `i18n.ts` (množná čísla, `tp`), formátování
// podle jazyka (`format.ts`) a HLÍDAČ slovníků: projde `src/**/*.ts(x)` (parser TypeScriptu v `i18nScan.ts`)
// a ověří, že (a) každý klíč z t()/tp() a datových párů {key, cs} má anglický záznam, (b) jeden klíč
// nemá dvě různé české výchozí hodnoty, (c) proměnné `{x}` a tučné `<b>` v en sedí s češtinou,
// (d) v přeložených souborech nezbyl natvrdo český text (mimo `PENDING` a `ALLOW_CZECH`).
// Navíc: žádné t() při importu modulu, žádné osiřelé en klíče, všechny slovníky načtené v `i18n/dicts.ts` (líně).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { KEY, areaKey, boldCount, scanSource, varsOf } from "./i18nScan.ts";
import { lang, loadDict, plural, pluralForm, registerDict, setDictLoader, setLang, t, tp } from "./i18n.ts";
import { ago, fmtDate, num } from "./format.ts";

// Soubory (nebo složky `x/`), kde natvrdo česky psané texty ještě zůstaly — kontrola d je přeskočí,
// a–c platí i tady. Teď prázdné: celé UI jde přes t(); nový nepřeložený modul sem jen dočasně.
const PENDING = [
  // w7: dočasné kontrakty s workery I/J (`{ id, cs, en }` — přeložené daty, ne t()); lead při merge vezme jejich verzi
  "strip/looks.ts",
  "hover/modes.ts",
];

// Záměrně česky i v angličtině (kontrola d): soubor, přesný text, proč.
const ALLOW_CZECH = [
  { file: "onboarding/steps/Basics.tsx", text: "Čeština", why: "název jazyka ve volbě jazyka se píše v tom jazyce" },
  { file: "onboarding/steps/Basics.tsx", text: "Ahoj! Nastavíme si Ledge.", why: "ukázka češtiny u volby jazyka" },
  { file: "settings/panels/Start.tsx", text: "Čeština", why: "název jazyka ve volbě jazyka se píše v tom jazyce" },
  { file: "settings/panels/Personal.tsx", text: "Plzeň", why: "název města v nabídce polohy počasí (vlastní jméno, ukládá se do configu)" },
];

// Volání s klíčem mimo literál: soubor, zdrojový text klíče, proč je v pořádku.
const PAIRS = "klíč i čeština jsou v datech jako pár {key, cs} / [key, cs] — ty hlídá kontrola párů";
const TABLE = "texty tabulky FIELDS (labels.ts) — páry z TABLES";

// Tabulky popisků s klíčem složeným z id (scanner je počítá jako páry): soubor, proměnná, klíč pro vlastnost.
const TABLES = [
  {
    file: "settings/labels.ts",
    table: "FIELDS",
    key: (id, prop) => ({ label: `settings.f.${id}`, hint: `settings.f.${id}.hint`, kw: `settings.f.${id}.kw` })[prop] ?? null,
  },
];
const DYNAMIC = [
  { file: "actions/ActionEditor.tsx", key: "m.key", why: PAIRS },
  { file: "actions/ActionEditor.tsx", key: "e().key", why: "chyby `draftError` v actions/model.ts — " + PAIRS },
  { file: "actions/ActionEditor.tsx", key: "k.key", why: PAIRS },
  { file: "onboarding/steps/Basics.tsx", key: "s.key", why: "kroky průvodce STEPS — " + PAIRS },
  { file: "onboarding/Wizard.tsx", key: "s.key", why: "kroky průvodce STEPS — " + PAIRS },
  { file: "palette/items.ts", key: "pn.key", why: "panely z settings/registry.ts — " + PAIRS },
  { file: "palette/items.ts", key: "pn.kw[0]", why: "klíčová slova panelů z settings/registry.ts — " + PAIRS },
  { file: "palette/winsettings.ts", key: "p.name[0]", why: "stránky Nastavení Windows WIN_PAGES — " + PAIRS },
  { file: "palette/Palette.tsx", key: "...SECTION_LABEL[r().item.section]", why: PAIRS },
  { file: "ui/shortcuts.ts", key: "s.key", why: "tabulka SHORTCUTS — " + PAIRS },
  { file: "ui/shortcuts.ts", key: "g.key", why: "skupiny SHORTCUT_GROUPS — " + PAIRS },
  { file: "settings/catalog.ts", key: "p.key", why: "název panelu (`own()` a registry.ts) — " + PAIRS },
  { file: "settings/catalog.ts", key: "p.kw[0]", why: "klíčová slova panelu — " + PAIRS },
  { file: "settings/labels.ts", key: "`settings.f.${id}`", why: TABLE },
  { file: "settings/labels.ts", key: "`settings.f.${id}.hint`", why: TABLE },
  { file: "settings/labels.ts", key: "`settings.f.${id}.kw`", why: TABLE },
  { file: "settings/labels.ts", key: "p[0]", why: "Doporučené a věty sekcí (settings/structure.ts) — " + PAIRS },
  { file: "settings/labels.ts", key: "s.key", why: "názvy sekcí Nastavení (settings/structure.ts SECTIONS) — " + PAIRS },
  { file: "theme/label.ts", key: "p[0]", why: "styly, písma, materiály, rozložení, šířky (theme/looks.ts) a animace (AppearanceMotion.tsx) — " + PAIRS },
  { file: "onboarding/steps/Basics.tsx", key: "THEMES[id()].key", why: "motivy v theme/themes.ts — " + PAIRS },
  { file: "layout/text.ts", key: "x[0]", why: "názvy karet, částí a záložek z layout/model.ts (CARD_INFO, PART_INFO, builtinName), volby vzhledu karty z layout/look.ts (ACCENT_NAMES, BG_NAMES, DENSITY_NAMES, VIEW_NAMES) a vzhledu zástupce z tilelook/ (SHAPE_NAMES, GLYPHS, MODES) — " + PAIRS },
  { file: "settings/panels/Backup.tsx", key: "n[0]", why: "názvy sekcí SECTION_NAMES — " + PAIRS },
  { file: "settings/panels/Claude.tsx", key: "...HOOKS_TEXT[h().state]", why: PAIRS },
  { file: "settings/panels/StripSettings.tsx", key: "x.key", why: "položky proužku STRIP_ITEMS — " + PAIRS },
  { file: "settings/panels/Notes.tsx", key: "...(SYNC_TEXT[st().state] ?? [\"\", st().state])", why: PAIRS },
];

const SRC = fileURLToPath(new URL(".", import.meta.url));
const walk = (d) => readdirSync(d).flatMap((n) => {
  const p = join(d, n);
  return statSync(p).isDirectory() ? walk(p) : [p];
});
const rel = (p) => relative(SRC, p).split(sep).join("/");
const FILES = walk(SRC)
  .filter((p) => /\.(ts|tsx)$/.test(p) && !p.endsWith(".d.ts"))
  .map((p) => ({ file: rel(p), scan: scanSource(rel(p), readFileSync(p, "utf8"), TABLES.filter((x) => x.file === rel(p))) }));
const pending = (f) => PENDING.some((p) => (p.endsWith("/") ? f.startsWith(p) : f === p));

const CALLS = FILES.flatMap(({ file, scan }) => scan.calls.map((c) => ({ ...c, file })));
/** en slovník ze všech `registerDict("en", …)`: klíč → { value, file } */
const EN = new Map();
const DICT_FILES = new Set();
const dictErrors = [];
for (const { file, scan } of FILES) {
  for (const e of scan.dict) {
    if (e.lang !== "en") continue;
    DICT_FILES.add(file);
    if (e.value === null) dictErrors.push(`${file}:${e.line} ${e.key}: hodnota není řetězcový literál`);
    const prev = EN.get(e.key);
    if (prev && prev.value !== e.value) dictErrors.push(`${e.key}: dva různé překlady (${prev.file}, ${file})`);
    EN.set(e.key, { value: e.value ?? "", file });
  }
}
const at = (c) => `${c.file}:${c.line}`;
const enKeys = (c) => (c.fn === "tp" ? [`${c.key}.one`, `${c.key}.other`] : [c.key]);

// ---------- čistá logika ----------

test("pluralForm: cs 1 / 2–4 / jinak, en 1 / jinak", () => {
  const cases = [
    ["cs", 0, "other"], ["cs", 1, "one"], ["cs", 2, "few"], ["cs", 3, "few"], ["cs", 4, "few"], ["cs", 5, "other"],
    ["cs", 11, "other"], ["cs", 21, "other"], ["cs", 22, "other"], ["cs", 1.5, "other"], ["cs", -1, "other"],
    ["en", 0, "other"], ["en", 1, "one"], ["en", 2, "other"], ["en", 4, "other"], ["en", 1.5, "other"],
  ];
  for (const [l, n, f] of cases) assert.equal(pluralForm(l, n), f, `${l} ${n}`);
});

test("plural: bez `few` platí `other`", () => {
  const f = { one: "1 x", few: "2–4 x", other: "5+ x" };
  assert.equal(plural("cs", 1, f), "1 x");
  assert.equal(plural("cs", 3, f), "2–4 x");
  assert.equal(plural("cs", 7, f), "5+ x");
  assert.equal(plural("cs", 3, { one: "a", other: "b" }), "b");
  assert.equal(plural("en", 3, f), "5+ x");
});

test("t / tp: čeština z kódu, angličtina ze slovníku, chybějící klíč = čeština", () => {
  registerDict("en", { "test.hello": "Hi {name}", "test.files.one": "{n} file", "test.files.other": "{n} files" });
  const files = ["{n} soubor", "{n} soubory", "{n} souborů"];
  try {
    setLang("cs");
    assert.equal(t("test.hello", "Ahoj {name}", { name: "Jirko" }), "Ahoj Jirko");
    assert.deepEqual([1, 3, 5, 0].map((n) => tp("test.files", n, files)), ["1 soubor", "3 soubory", "5 souborů", "0 souborů"]);
    assert.equal(tp("test.fixed", 3, "{n}× pád"), "3× pád");
    setLang("en");
    assert.equal(t("test.hello", "Ahoj {name}", { name: "Jirko" }), "Hi Jirko");
    assert.equal(t("test.missing", "Jen česky"), "Jen česky");
    assert.deepEqual([1, 3, 0].map((n) => tp("test.files", n, files)), ["1 file", "3 files", "0 files"]);
    assert.equal(tp("test.fixed", 3, "{n}× pád"), "3× pád", "bez en tvarů zůstane čeština");
    assert.equal(t("test.hello", "Ahoj {name}"), "Hi {name}", "neznámá proměnná zůstane vidět");
  } finally {
    setLang("cs");
  }
});

test("format: datum, čas a čísla podle jazyka (skutečné slovníky)", () => {
  registerDict("en", Object.fromEntries([...EN].map(([k, v]) => [k, v.value])));
  const d = new Date(2026, 9, 16, 7, 5);
  const now = d.getTime();
  try {
    setLang("cs");
    assert.equal(fmtDate(d), "pátek 16. října");
    assert.deepEqual([ago(now - 10_000, now), ago(now - 5 * 60_000, now), ago(now - 3 * 3600_000, now)], ["teď", "před 5 min", "před 3 h"]);
    assert.equal(num(1234.5, 1), "1 234,5");
    setLang("en");
    assert.equal(fmtDate(d), "Friday, October 16");
    assert.deepEqual([ago(now - 10_000, now), ago(now - 5 * 60_000, now), ago(now - 3 * 3600_000, now)], ["now", "5 min ago", "3 h ago"]);
    assert.equal(num(1234.5, 1), "1,234.5");
  } finally {
    setLang("cs");
  }
});

test("i18nScan: volání, páry, zbytky češtiny (komentáře a console se nepočítají)", () => {
  const s = scanSource("x.tsx", [
    'const TOP = t("a.top", "Nahoře");',
    "// Čeština v komentáři se nepočítá",
    'const THEMES = { dark: { key: "a.dark", cs: "Tmavý" } };',
    'const LABEL = { x: ["ui.pair", "Akce"], y: ["#fff", "ui.acc", "Modrá"], z: ["drawer.glass", "vzhled"] };',
    'const FIELDS = { "a.b": { panel: "p", label: "Popisek", hint: "Nápověda" } };',
    "export function F(p) {",
    '  console.log("chyba č. 5");',
    '  registerDict("en", { "a.top": "Top" });',
    '  return <div title={t("a.title", "Název {x}", { x: tp("a.n", p.n, ["{n} kus", "{n} kusy", "{n} kusů"]) })}>Přidej {t(`a.f.${p.id}`, p.cs)}</div>;',
    "}",
  ].join("\n"), [{ table: "FIELDS", key: (id, prop) => ({ label: `x.${id}`, hint: `x.${id}.hint` })[prop] ?? null }]);
  const calls = s.calls.map((c) => [c.fn, c.key, c.cs?.join("|") ?? null, c.top, !!c.pair, c.prefix ?? null]);
  assert.deepEqual(calls, [
    ["t", "a.top", "Nahoře", true, false, null],
    ["t", "a.dark", "Tmavý", false, true, null],
    ["t", "ui.pair", "Akce", false, true, null],
    ["t", "ui.acc", "Modrá", false, true, null],
    ["t", "x.a.b", "Popisek", false, true, null],
    ["t", "x.a.b.hint", "Nápověda", false, true, null],
    ["t", "a.title", "Název {x}", false, false, null],
    ["tp", "a.n", "{n} kus|{n} kusy|{n} kusů", false, false, null],
    ["t", null, null, false, false, "a.f."],
  ]);
  assert.deepEqual(s.dict.map((e) => [e.lang, e.key, e.value]), [["en", "a.top", "Top"]]);
  assert.deepEqual(s.leftovers.map((l) => l.text), ["Přidej"]);
  assert.deepEqual(varsOf("{b} a {a} {b}"), ["a", "b"]);
  assert.equal(boldCount("<b>1</b> a <b>2</b>"), 2);
});

// ---------- hlídač slovníků nad celým src ----------

test("slovníky: hodnoty jsou literály a jeden klíč má jeden překlad", () => {
  assert.deepEqual(dictErrors, []);
  assert.ok(EN.size > 300, `málo en klíčů (${EN.size}) — načetly se slovníky?`);
});

test("klíče: literál ve tvaru oblast.klic, žádné t() při importu modulu", () => {
  const bad = CALLS.filter((c) => c.key !== null && !KEY.test(c.key)).map((c) => `${at(c)} ${c.key}`);
  assert.deepEqual(bad, [], "klíč musí být hierarchický (`cards.head.title`)");
  const area = CALLS.filter((c) => c.key !== null && !areaKey(c.key)).map((c) => `${at(c)} ${c.key}`);
  assert.deepEqual(area, [], "klíč začíná oblastí z AREAS (i18nScan.ts)");
  const top = CALLS.filter((c) => c.top).map((c) => `${at(c)} ${c.key}`);
  assert.deepEqual(top, [], "t() mimo funkci se vyhodnotí jednou česky — dej ho do funkce (getter, `() => t(…)`)");
});

test("klíče mimo literál jsou vědomé (DYNAMIC) a seznam nezastarává", () => {
  const dyn = CALLS.filter((c) => c.key === null || c.cs === null);
  const unknown = dyn.filter((c) => !DYNAMIC.some((d) => d.file === c.file && d.key === c.keyText)).map((c) => `${at(c)} t(${c.keyText}, …)`);
  assert.deepEqual(unknown, [], "nový dynamický klíč — dej data do párů {key, cs} nebo zapiš do DYNAMIC s důvodem");
  const stale = DYNAMIC.filter((d) => !dyn.some((c) => d.file === c.file && d.key === c.keyText)).map((d) => `${d.file} ${d.key}`);
  assert.deepEqual(stale, [], "položka DYNAMIC už v kódu není — smaž ji");
  for (const d of DYNAMIC) assert.ok(d.why, `${d.file}: chybí důvod`);
});

test("(a) každý klíč má anglický záznam", () => {
  const missing = CALLS.filter((c) => c.key !== null).flatMap((c) => enKeys(c).filter((k) => !EN.has(k)).map((k) => `${at(c)} ${k}`));
  assert.deepEqual([...new Set(missing)], [], "doplň angličtinu do src/i18n/en-*.ts");
});

test("(b) jeden klíč = jedna česká výchozí hodnota", () => {
  const seen = new Map();
  const clash = [];
  for (const c of CALLS) {
    if (c.key === null || c.cs === null) continue;
    const cs = c.cs.join(" | ");
    const prev = seen.get(c.key);
    if (prev && prev.cs !== cs) clash.push(`${c.key}: „${prev.cs}“ (${prev.at}) × „${cs}“ (${at(c)})`);
    else if (!prev) seen.set(c.key, { cs, at: at(c) });
  }
  assert.deepEqual(clash, []);
});

test("(c) proměnné {x} a tučné <b> v angličtině sedí s češtinou", () => {
  const bad = [];
  for (const c of CALLS) {
    if (c.key === null || c.cs === null) continue;
    const csVars = varsOf(c.cs.join(" "));
    const csBold = boldCount(c.cs[0]);
    for (const k of enKeys(c)) {
      const en = EN.get(k)?.value;
      if (en === undefined) continue;
      const enVars = varsOf(en);
      const ok = c.fn === "tp"
        ? enVars.every((v) => v === "n" || csVars.includes(v)) && csVars.every((v) => v === "n" || enVars.includes(v))
        : enVars.join() === csVars.join();
      if (!ok) bad.push(`${k}: cs {${csVars}} × en {${enVars}} (${at(c)})`);
      if (boldCount(en) !== csBold) bad.push(`${k}: <b> cs ${csBold}× × en ${boldCount(en)}× (${at(c)})`);
    }
  }
  assert.deepEqual([...new Set(bad)], []);
});

test("(d) v přeložených souborech nezbyl natvrdo český text", () => {
  const used = new Set();
  const left = [];
  for (const { file, scan } of FILES) {
    if (pending(file)) continue;
    for (const l of scan.leftovers) {
      const allow = ALLOW_CZECH.find((a) => a.file === file && a.text === l.text);
      if (allow) used.add(allow);
      else left.push(`${file}:${l.line} ${JSON.stringify(l.text.slice(0, 80))}`);
    }
  }
  assert.deepEqual(left, [], "převeď na t(\"oblast.klic\", \"čeština\") — nebo výjimku do ALLOW_CZECH s důvodem");
  assert.deepEqual(ALLOW_CZECH.filter((a) => !used.has(a)).map((a) => `${a.file} ${a.text}`), [], "výjimka už neplatí — smaž ji");
});

test("žádné osiřelé anglické klíče", () => {
  const used = new Set(CALLS.filter((c) => c.key !== null).flatMap(enKeys));
  const prefixes = CALLS.map((c) => c.prefix).filter(Boolean);
  const orphan = [...EN.keys()].filter((k) => !used.has(k) && !prefixes.some((p) => k.startsWith(p)));
  assert.deepEqual(orphan, [], "klíč už v kódu není — smaž ho ze slovníku (nebo je překlep)");
});

/** Slovníky načítané líně se svými daty (samostatný chunk) → importuje je jen ten modul, nikdy `dicts.ts`. */
const LAZY_DICTS = [{ dict: "i18n/en-emoji.ts", by: "palette/emojidata.ts", why: "jména emoji jen s tabulkou (dynamický import v palette/emoji.ts)" }];

test("všechny slovníky jsou v i18n/dicts.ts a load.ts je registruje jako líný loader (první import App.tsx)", () => {
  const load = readFileSync(join(SRC, "i18n", "dicts.ts"), "utf8");
  const imported = new Set([...load.matchAll(/^import "(.+)";$/gm)].map((m) => rel(join(SRC, "i18n", m[1] + ".ts"))));
  const lazy = new Set(LAZY_DICTS.map((x) => x.dict));
  assert.deepEqual([...DICT_FILES].filter((f) => !imported.has(f) && !lazy.has(f)).sort(), [], "slovník chybí v src/i18n/dicts.ts");
  for (const x of LAZY_DICTS) {
    assert.ok(DICT_FILES.has(x.dict) && !imported.has(x.dict), `${x.dict}: líný slovník nesmí být v dicts.ts`);
    const name = x.dict.split("/").pop();
    assert.ok(readFileSync(join(SRC, x.by), "utf8").includes(`import "../i18n/${name}";`), `${x.by} musí importovat ${x.dict}`);
    const stem = name.replace(/\.ts$/, "");
    const others = FILES.filter((f) => f.file !== x.by && f.file !== x.dict && readFileSync(join(SRC, f.file), "utf8").includes(`/${stem}`)).map((f) => f.file);
    assert.deepEqual(others, [], `${x.dict} importuje jen ${x.by} (${x.why})`);
  }
  assert.ok(readFileSync(join(SRC, "i18n", "load.ts"), "utf8").includes('setDictLoader(() => import("./dicts"));'), "load.ts musí registrovat líný import dicts");
  const app = readFileSync(join(SRC, "App.tsx"), "utf8");
  assert.match(app, /^import "\.\/i18n\/load";$/m);
  assert.ok(app.indexOf('import "./i18n/load";') < app.indexOf("import {"), "load musí být první import App.tsx");
});

test("setLang s líným slovníkem: čeština hned, jiný jazyk až po načtení, poslední volání vyhrává", async () => {
  let loads = 0;
  let release;
  setDictLoader(() => (loads++, new Promise((r) => (release = r))));
  try {
    setLang("cs");
    assert.equal(loads, 0, "čeština nenačítá nic");
    await loadDict("cs");
    assert.equal(loads, 0);
    setLang("en");
    assert.equal(lang(), "cs", "do načtení slovníku zůstává čeština");
    setLang("cs"); // uživatel to rozmyslel — pozdní odpověď loaderu nesmí přepsat
    release();
    await loadDict("en");
    await new Promise((r) => setTimeout(r, 0));
    assert.equal(lang(), "cs");
    setLang("en");
    assert.equal(lang(), "en", "slovník už je načtený → synchronně");
    assert.equal(loads, 1, "chunk se stahuje jednou");
  } finally {
    setDictLoader(null);
    setLang("cs");
  }
});

test("loadDict: selhání loaderu nezablokuje další pokus a setLang spadne na češtinu z kódu", async () => {
  let n = 0;
  setDictLoader(() => (++n === 1 ? Promise.reject(new Error("síť")) : Promise.resolve()));
  try {
    await assert.rejects(loadDict("en"));
    await loadDict("en");
    assert.equal(n, 2);
  } finally {
    setDictLoader(null);
  }
  let m = 0;
  setDictLoader(() => (++m, Promise.reject(new Error("síť"))));
  try {
    setLang("en");
    await new Promise((r) => setTimeout(r, 0));
    assert.equal(lang(), "en", "i při chybě se jazyk přepne (chybějící klíč = čeština)");
  } finally {
    setDictLoader(null);
    setLang("cs");
  }
});
