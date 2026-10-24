// Chytré hledání v Nastavení (`searchModel.ts`) nad SKUTEČNÝM katalogem v češtině (stránky z registru,
// volby z labels.ts s klíčem configu, prefixy z placement.ts jen pro klíč) — tabulka dotaz → co musí
// najít (a jako první, kde na pořadí záleží). Bez DOM.
//   node --experimental-strip-types --test src/settings/searchModel.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { keyWords, nearlyEqual, smartSearch, synonymsOf } from "./searchModel.ts";
import { PANEL_META } from "./registry.ts";
import { PLACEMENT } from "./placement.ts";

const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");
const str = (line, k) => line.match(new RegExp(`\\b${k}: "((?:[^"\\\\]|\\\\.)*)"`))?.[1] ?? "";

/** Položky jako `catalog.searchItems` (česky). */
function items() {
  const out = PANEL_META.map((p) => ({ panel: p.id, anchor: "", label: p.cs, hint: "", kw: p.kw[1] }));
  const ids = new Set();
  for (const l of read("labels.ts").split("\n")) {
    const m = l.match(/^\s*"?([\w.]+)"?: \{ panel: "(\w+)"/);
    if (!m) continue;
    ids.add(m[1]);
    out.push({ panel: m[2], anchor: str(l, "anchor") || m[1], label: str(l, "label"), hint: str(l, "hint"), kw: str(l, "kw"), key: m[1] });
  }
  for (const [k, page] of PLACEMENT) if (!ids.has(k)) out.push({ panel: page, anchor: "", label: page, hint: k, kw: "", key: k, keyOnly: true });
  return out;
}
const ITEMS = items();
const find = (q) => smartSearch(ITEMS, q);
const hasAnchor = (q, a) => find(q).some((x) => x.anchor === a);

test("hledání: název, popis, klíčová slova, bez diakritiky", () => {
  const cases = [
    ["jazyk", "ui.lang"],
    ["pruvodce", "onboarding.wizard"],
    ["setrit pamet", "drawer.keepWarmS"],
    ["okna s taby", "drawer.barWindows"],
    ["historie schranky", "clipboard.enabled"],
    ["widget plocha", "desk.layer"],
  ];
  for (const [q, a] of cases) assert.ok(hasAnchor(q, a), `„${q}“ najde ${a}`);
});

test("hledání: synonyma cs ↔ en", () => {
  const cases = [
    ["color", "appearance.theme"],
    ["dark", "appearance.theme"],
    ["shortcut", "hotkeys.toggle"],
    ["hotkey", "hotkeys.enabled"],
    ["weather", "personal.weather"],
    ["clipboard", "clipboard.enabled"],
    ["backup", "backup.export"],
    ["startup", "autostart"],
    ["language", "ui.lang"],
    ["sticky", "notes.pins.style"],
  ];
  for (const [q, a] of cases) assert.ok(hasAnchor(q, a), `„${q}“ najde ${a}`);
});

test("hledání: klíč configu (celý, část, camelCase) a prefix bez vlastní volby", () => {
  assert.equal(find("drawer.keepWarmS")[0]?.anchor, "drawer.keepWarmS", "celý klíč první");
  assert.ok(hasAnchor("keepwarm", "drawer.keepWarmS"));
  assert.ok(hasAnchor("questionHoldS", "claude.questionHoldS"));
  // klíč bez vlastní volby → stránka podle nejdelšího prefixu (jen přes klíč, slova ho nenajdou)
  const deep = find("drawer.hoverDelayMs");
  assert.ok(deep.some((x) => x.keyOnly && x.panel === "chovani"), "drawer.hoverDelayMs → Otevírání a klávesy");
  assert.ok(!find("prodlevy").some((x) => x.keyOnly), "slova položky jen pro klíč nenajdou");
});

test("hledání: drobný překlep u delších slov", () => {
  assert.ok(hasAnchor("kalendr", "personal.calendars"), "kalendr → Kalendáře");
  assert.ok(hasAnchor("schranky", "clipboard.enabled"));
  assert.ok(!find("xq").length, "nesmysl nic");
  assert.deepEqual(find(""), []);
  assert.deepEqual(find("   "), []);
});

test("pořadí: začátek názvu před shodou v popisu, synonymum a překlep až za přímou shodou", () => {
  const r = find("animace");
  assert.equal(r[0].anchor, "appearance.motion", "Animace první");
  const theme = find("barvy");
  assert.ok(theme.findIndex((x) => x.anchor === "appearance.theme") < theme.findIndex((x) => x.anchor === "strip.items"), "Barvy (název) před zmínkou v popisu");
});

test("čisté pomocné funkce", () => {
  assert.equal(keyWords("drawer.keepWarmS"), "drawer.keepwarms drawer keep warm s");
  for (const [a, b, want] of [["kalendar", "kalendr", true], ["abc", "abc", true], ["pocasi", "pocasy", true], ["pocasi", "pocas", true], ["abcd", "ab", false], ["animace", "animaze", true], ["animace", "anilmaze", false]]) {
    assert.equal(nearlyEqual(a, b), want, `${a} ~ ${b}`);
  }
  assert.ok(synonymsOf("color").includes("barva"));
  assert.ok(synonymsOf("barv").includes("color"), "i začátek slova od 3 znaků");
  assert.deepEqual(synonymsOf("xy"), []);
});
