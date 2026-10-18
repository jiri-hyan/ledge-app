// Kolize globálních CSS tříd (bez DOM): `npm run test:ui`. Všechny `src/**/*.css` jsou globální — třída,
// kterou jeden soubor vlastní (`.proj { … }`), se nesmí objevit v jiném souboru (ani jako `.st-item.proj`),
// jinak se pravidla dvou oblastí potkají na jednom prvku. Oprava = prefix oblasti (docs/codemap/ui.md).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { classesIn, collisions, cssRules, definedClasses, missingClasses, ownedClass, usedClasses } from "./cssClasses.ts";

// Záměrně sdílené třídy: soubory, které je smějí zmínit, a proč. Nový soubor = nové rozhodnutí sem.
const SHARED = {
  // základní stavebnice ze styles.css; oblasti je rozšiřují jen v kontextu nebo variantou
  btn: { files: ["styles.css", "onboarding/onboarding.css"], why: "průvodce: `.ob-foot .btn.ghost/.pri` v patičce" },
  card: { files: ["styles.css", "settings/panels/appearance.css"], why: "náhled vzhledu kreslí skutečné karty (`.ap .card`)" },
  seg: { files: ["styles.css", "settings/panels/appearance.css"], why: "náhled vzhledu kreslí skutečný přepínač (`.ap-head .seg`)" },
  row: { files: ["styles.css", "sessions.css"], why: "řádek sezení je varianta řádku karty (`.row.sess`)" },
  mini: { files: ["styles.css", "notes/notes.css", "settings.css"], why: "varianty malého tlačítka: `.mini.on` (výběr připomínky), `.mini.danger` (potvrzení)" },
  banner: { files: ["styles.css", "notes/notes.css"], why: "varianta hlášky `.banner.warn` v editoru poznámky" },
  // strop výšky (fit.ts) je průřezový: ořízne obsah šuplíku a nechá posouvat jen hlavní plochy stránek
  content: { files: ["styles.css", "fit.css"], why: "fit.css doplní obsahu šuplíku strop výšky" },
  grid: { files: ["styles.css", "fit.css"], why: "mřížka karet je posuvná plocha hlavní stránky" },
  deck: { files: ["layout/layout.css", "fit.css", "styles.css"], why: "mřížka karet záložky (layout/Deck.tsx) je posuvná plocha hlavní stránky; mezery a okraj podle rozložení design stylu (`html[data-layout] .deck`)" },
  "notes-page": { files: ["notes/notes.css", "fit.css"], why: "posuvná plocha stránky poznámek" },
  "note-page": { files: ["notes/notes.css", "fit.css"], why: "posuvná plocha editoru poznámky" },
  st: { files: ["settings.css", "fit.css"], why: "Nastavení se vejde pod strop výšky" },
};

// Třídy, které kód přiděluje, ale žádné CSS pravidlo je nezmiňuje: jen JS/test háčky nebo stav bez vlastního
// vzhledu, každá s důvodem. Chybějící styl (v0.2.7 `.opt`) sem NEPATŘÍ — doplň pravidlo do CSS oblasti.
const HOOKS = {};

// Místa, kde se název třídy skládá za běhu (nejde ověřit staticky): klíč = soubor + kód místa, `values` = všechny
// hodnoty, které výraz může dát (z typu/tabulky v `why`). Hodnoty se ověří jako definované modifikátory, které
// žádný soubor nevlastní (jinak na prvek padnou cizí pravidla — `ap-strip pill` bral písmo z `.pill`).
const DYNAMIC = [
  { file: "cards/Projects.tsx", expr: "class={`ring ${pc().phase}`}", values: ["preparing", "starting", "ready", "running", "exited", "failed"], why: "`Phase` v types.ts" },
  { file: "cards/Running.tsx", expr: "class={`pill ${ph()[0]}`}", values: ["a", "g", "n", "r"], why: "tabulka `PHASE` + záloha \"n\"" },
  { file: "ClaudeSessions.tsx", expr: "class={`pill ${st()[0]}`}", values: ["a", "v", "n", "r"], why: "tabulka `STATUS`" },
  { file: "notes/NotesCard.tsx", expr: "class={`pill ${l()[0]}`}", values: ["a", "b", "n", "r"], why: "`remindLabel` (a/b/n) a `syncLabel` (a/n/r) v notes/store.ts" },
  { file: "settings/fields/SortList.tsx", expr: "class={`st-sort ${p.class ?? \"\"}`}", values: [], why: "prop `class` předává volající atributem `class=\"…\"` — ten se kontroluje staticky u něj" },
  { file: "settings/panels/Projects.tsx", expr: "class={`st-tag ${pr.kind}`}", values: ["dev", "app"], why: "`ProjectView.kind` v types.ts" },
  { file: "Settings.tsx", expr: "class={`st-bar ${status().tone}`}", values: ["ok", "warn", "bad"], why: "`status()` v Settings.tsx" },
];

const SRC = fileURLToPath(new URL(".", import.meta.url));
const walk = (d, ext) => readdirSync(d).flatMap((n) => {
  const p = join(d, n);
  return statSync(p).isDirectory() ? walk(p, ext) : ext.test(p) ? [p] : [];
});
const rel = (p) => relative(SRC, p).split(sep).join("/");
const tree = () => walk(SRC, /\.css$/).map((p) => ({ file: rel(p), css: readFileSync(p, "utf8") }));
const sources = () => walk(SRC, /\.tsx?$/).filter((p) => !/\.test\./.test(p)).map((p) => ({ file: rel(p), src: readFileSync(p, "utf8") }));

test("ownedClass: vlastní jen samostatná třída na začátku selektoru", () => {
  const cases = [
    [".proj", "proj"],
    [".proj:hover::before", "proj"],
    [".proj > .logo img", "proj"],
    ['html[data-glass="on"] .shell', "shell"],
    [".st-tabs button.on", "st-tabs"],
    [".st-item.proj", null],
    [".mini.danger", null],
    ["button:disabled", null],
    [":is(.a, .b) :focus-visible", null],
    [".x:not(.y) .z", "x"],
    ['input[type="text"].pp-q', "pp-q"],
  ];
  for (const [sel, want] of cases) assert.equal(ownedClass(sel), want, sel);
});

test("classesIn: i třídy uvnitř :is/:has/:not, bez hodnot atributů", () => {
  assert.deepEqual(classesIn(".a > .b:has(+ :is(.c, .d))"), ["a", "b", "c", "d"]);
  assert.deepEqual(classesIn('[data-x=".nope"] .y'), ["y"]);
});

test("cssRules: rozbalí @media, přeskočí @keyframes, rozdělí seznam selektorů", () => {
  const r = cssRules("/* .skryto {} */ .a, .b { x: 1 } @media (max-width: 9px) { .c { y: 2 } } @keyframes k { from { o: 0 } } .d{}");
  assert.deepEqual(r.map((x) => x.selector), [".a", ".b", ".c", ".d"]);
});

test("collisions: tabulka", () => {
  const f = (file, css) => ({ file, css });
  const cases = [
    // kolo 3: Nastavení použilo `.proj` jako modifikátor, hlavní stránka ho vlastní
    [[f("projects.css", ".proj { display: flex }"), f("settings.css", ".st-item.proj .grow { gap: 8px }")], ["proj"]],
    // dvě oblasti vlastní stejné jméno
    [[f("head.css", ".chip { height: 30px }"), f("settings.css", ".chip { padding: 2px }")], ["chip"]],
    // prefixované třídy a modifikátory, které nikdo nevlastní, jsou v pořádku
    [[f("a.css", ".a-x.on { }"), f("b.css", ".b-x.on { } .b-x .grow { }")], []],
    // sdílená třída jen v povolených souborech
    [[f("styles.css", ".mini { }"), f("notes.css", ".mini.on { }")], [], { mini: { files: ["styles.css", "notes.css"], why: "" } }],
    // … a nový soubor mimo allowlist je zase kolize
    [[f("styles.css", ".mini { }"), f("x.css", ".x .mini { }")], ["mini"], { mini: { files: ["styles.css", "notes.css"], why: "" } }],
  ];
  for (const [files, want, allow] of cases) assert.deepEqual(collisions(files, allow).map((c) => c.cls), want);
});

test("src/**/*.css: žádná kolize globálních tříd", () => {
  const found = collisions(tree(), SHARED);
  const msg = found.map((c) => `.${c.cls}: vlastní ${c.owners.join(", ")}; zmiňují ${c.files.join(", ")}`).join("\n");
  assert.deepEqual(found, [], `Kolize tříd (přejmenuj prefixem oblasti, nebo přidej do SHARED s důvodem):\n${msg}`);
});

test("allowlist SHARED je přesný (žádná zastaralá položka)", () => {
  const files = tree();
  const mentions = (cls) => files.filter((x) => cssRules(x.css).some((r) => classesIn(r.selector).includes(cls))).map((x) => x.file).sort();
  for (const [cls, { files: allowed, why }] of Object.entries(SHARED)) {
    assert.ok(why, `.${cls}: chybí důvod`);
    assert.deepEqual(mentions(cls), [...allowed].sort(), `.${cls}: allowlist nesedí se soubory, které třídu zmiňují`);
    assert.ok(collisions(files, {}).some((c) => c.cls === cls), `.${cls}: už nekoliduje — odeber ze SHARED`);
  }
});

test("usedClasses: tabulka", () => {
  const cases = [
    ['<div class="a b-c" />', ["a", "b-c"], 0],
    ["<i class={`x ${on() ? \"y\" : \"\"} z`} />", ["x", "y", "z"], 0],
    ['<i class={k === "dev" ? "b" : k === "app" ? "v" : "n"} />', ["b", "n", "v"], 0],
    ['<i classList={{ on: a(), "has-img": !!b, [c]: 1 }} />', ["has-img", "on"], 1],
    ['Object.assign(el, { className: "md-alt" }); el.classList.add("p", "q"); el.classList.toggle(name)', ["md-alt", "p", "q"], 1],
    ["<i class={`pill ${tone()}`} />", ["pill"], 1],
    ["<i class={`pill-${tone()}`} />", [], 1],
    ['<i class={"st-" + x} />', [], 1],
    ["<i class={p.class} />", [], 1],
    ['<i class={ok() && "on"} />', ["on"], 0],
    ['// <i class="komentar" />\n{/* <i class="jsx" /> */}<i accept="image/*" class="po" />', ["po"], 0],
    ['el.closest(".md-link"); el.classList.contains("on")', [], 0],
  ];
  for (const [src, classes, dyn] of cases) {
    const u = usedClasses(src);
    assert.deepEqual(u.classes, classes, src);
    assert.equal(u.dynamic.length, dyn, src);
  }
});

test("missingClasses: tabulka", () => {
  const css = [{ file: "a.css", css: ".a { } .b.on { } :is(.c, .d) { }" }];
  const src = (s) => [{ file: "x.tsx", src: s }];
  assert.deepEqual(missingClasses(src('<i class="a on c d" />'), css), []);
  // v0.2.7: řádek volby `.opt` bez pravidla
  assert.deepEqual(missingClasses(src('<i class="opt" />'), css), [{ cls: "opt", files: ["x.tsx"] }]);
  assert.deepEqual(missingClasses(src('<i class="hook" />'), css, { hook: "háček" }), []);
});

test("src/**: každá přidělená třída má CSS pravidlo", () => {
  const found = missingClasses(sources(), tree(), HOOKS);
  const msg = found.map((m) => `.${m.cls}: ${m.files.join(", ")}`).join("\n");
  assert.deepEqual(found, [], `Třída bez CSS pravidla (chybí styl → doplň do CSS oblasti; mrtvá → smaž z kódu; háček → HOOKS s důvodem):\n${msg}`);
});

test("allowlist HOOKS je přesný (žádná zastaralá položka)", () => {
  const defined = definedClasses(tree());
  const used = new Set(sources().flatMap((f) => usedClasses(f.src).classes));
  for (const [cls, why] of Object.entries(HOOKS)) {
    assert.ok(why, `.${cls}: chybí důvod`);
    assert.ok(used.has(cls), `.${cls}: kód ji už nepřiděluje — odeber z HOOKS`);
    assert.ok(!defined.has(cls), `.${cls}: už má CSS pravidlo — odeber z HOOKS`);
  }
});

test("dynamicky skládané třídy: každé místo v DYNAMIC, hodnoty jsou definované cizí-nevlastněné modifikátory", () => {
  const found = sources().flatMap((f) => usedClasses(f.src).dynamic.map((expr) => `${f.file} | ${expr}`)).sort();
  const listed = DYNAMIC.map((d) => `${d.file} | ${d.expr}`).sort();
  assert.deepEqual(found, listed, "Místa se jménem třídy skládaným za běhu: nové přidej do DYNAMIC (s hodnotami a důvodem), zmizelé odeber. Lépe: classList={{ a: …, b: … }} — ten se ověří staticky.");
  const css = tree();
  const defined = definedClasses(css);
  const owned = new Set(css.flatMap((f) => cssRules(f.css).map((r) => ownedClass(r.selector)).filter(Boolean)));
  for (const d of DYNAMIC) {
    assert.ok(d.why, `${d.file}: chybí důvod`);
    for (const v of d.values) {
      assert.ok(defined.has(v), `${d.file} ${d.expr}: hodnota .${v} nemá CSS pravidlo`);
      assert.ok(!owned.has(v), `${d.file} ${d.expr}: hodnota .${v} je třída, kterou vlastní jiná oblast — na prvek padnou její pravidla`);
    }
  }
});
