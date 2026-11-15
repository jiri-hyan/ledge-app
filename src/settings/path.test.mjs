// Guard cest v hláškách (kolo 33, L107): každé „Nastavení → X → Y“ / „Settings → X → Y“ ve zdrojácích (src/**, src-tauri/src/**)
// míří na skutečnou stránku (`registry.ts` `PANEL_META`, nebo část Studia `studio/parts.ts`) a sekci (`SECTIONS`, stránky);
// zastaralý název („Šuplík a karty“, „Útrata“, „Osobní“…) shodí test s výpisem soubor:řádek. Komentáře se nekontrolují
// (nezobrazí se). Cizí aplikace (Outlook, Claude Desktop, Windows → Soukromí) jsou výjimka. Text se skládá z `settingsPath`
// (`path.ts`); tady se ověřuje i to, že `SECTIONS` mají skutečný nadpis (kotvu) ve zdrojích a anglický název.
//   node --experimental-strip-types --test src/settings/path.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { PANEL_META, SECTIONS } from "./registry.ts";
import { PARTS } from "../studio/parts.ts";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
const walk = (dir, exts, out = []) => {
  for (const n of readdirSync(dir)) {
    if (n === "node_modules" || n === "target" || n === "target-agents" || n.startsWith(".")) continue;
    const p = join(dir, n);
    if (statSync(p).isDirectory()) walk(p, exts, out);
    else if (exts.some((e) => n.endsWith(e)) && !/\.test\.mjs$/.test(n)) out.push(p);
  }
  return out;
};
const FILES = [...walk(join(ROOT, "src"), [".ts", ".tsx"]), ...walk(join(ROOT, "src-tauri", "src"), [".rs"])];
const TEXT = new Map(FILES.map((f) => [f, readFileSync(f, "utf8")]));

/** Anglické slovníky: klíč → text (`"klíč": "text"` v `src/**` `en*.ts`). */
const EN = new Map();
for (const [f, s] of TEXT) {
  if (!/[\\/]en(-[\w]+)?\.ts$/.test(f)) continue;
  for (const m of s.matchAll(/^\s*"([\w.]+)":\s*"((?:[^"\\]|\\.)*)"/gm)) EN.set(m[1], m[2]);
}

const panelNames = PANEL_META.flatMap((p) => [p.cs, EN.get(p.key)]);
const partNames = PARTS.flatMap((p) => [p.name[1], EN.get(p.name[0])]);
const sectionNames = Object.values(SECTIONS).flatMap((s) => [s.cs, EN.get(s.key)]);
/** Ovládací prvky, které cesta jmenuje na konci (tlačítko, ne sekce). */
const CONTROLS = ["Z Launcheru…", "From Launcher…"];
const FIRST = [...panelNames, ...partNames].filter(Boolean);
const NEXT = [...sectionNames, ...panelNames, ...CONTROLS].filter(Boolean);
/** Cesta v cizí aplikaci / systému (před shodou, nebo jméno stránky). */
const FOREIGN_BEFORE = /(?:Outlook[^.;]{0,25}|Claude Desktop → |[Pp]ropojen\S* aplikac\S* \()$/;
const FOREIGN_FIRST = /^(?:Soukromí|Privacy)(?!\p{L})/u;
const STALE = /Šuplík a karty|Drawer and cards/;

/** Zkusí na začátku `s` najít nejdelší známý název (ne uprostřed slova, ne „Claude (MCP)“ jako „Claude“). */
function eat(s, names) {
  const hit = names.filter((n) => s.startsWith(n) && !/^(?: \(|\p{L}|\d)/u.test(s.slice(n.length))).sort((a, b) => b.length - a.length)[0];
  return hit ?? null;
}

/** Vrátí seznam chyb `soubor:řádek text` pro jeden zdrojový text. */
export function badPaths(file, text) {
  const bad = [];
  text.split("\n").forEach((line, i) => {
    const tr = line.trim();
    if (/^(\/\/|\/\*|\*)/.test(tr)) return;
    const where = `${file}:${i + 1}`;
    if (STALE.test(line)) bad.push(`${where}: zastaralý název stránky — ${tr.slice(0, 110)}`);
    for (const m of line.matchAll(/(?:Nastavení|Settings) → /g)) {
      const before = line.slice(Math.max(0, m.index - 40), m.index);
      if (FOREIGN_BEFORE.test(before)) continue;
      let rest = line.slice(m.index + m[0].length);
      if (FOREIGN_FIRST.test(rest)) continue;
      const first = eat(rest, FIRST);
      if (!first) { bad.push(`${where}: neznámá stránka v „${m[0]}${rest.slice(0, 40)}“`); continue; }
      rest = rest.slice(first.length);
      while (rest.startsWith(" → ")) {
        rest = rest.slice(3);
        const sec = eat(rest, NEXT);
        if (!sec) { bad.push(`${where}: neznámá sekce v „…${first} → ${rest.slice(0, 40)}“`); break; }
        rest = rest.slice(sec.length);
      }
    }
  });
  return bad;
}

test("hlášky „Nastavení → …“ míří na existující stránky a sekce (src + Rust)", () => {
  const bad = FILES.flatMap((f) => badPaths(relative(ROOT, f).replaceAll("\\", "/"), TEXT.get(f)));
  assert.deepEqual(bad, [], "cesta v hlášce neexistuje — použij `settingsPath` (path.ts) nebo oprav název");
});

test("badPaths: tabulka (dobré / špatné cesty, výjimky)", () => {
  const cases = [
    ['t("a", "Zapni v Nastavení → Claude → Hodnota v cenách API.")', 0],
    ['t("a", "Zapni v Nastavení → Claude → Útrata.")', 1],
    ['"Settings → Card contents → Calendars → Advanced"', 0],
    ['"Settings → Drawer and cards → Card contents"', 2],
    ['"Nastavení → Osobní"', 1],
    ['"Nastavení → Claude (MCP)"', 1],
    ['"Nastavení → Claude → Claude (MCP)"', 0],
    ['"Nastavení → Projekty, akce a odkazy"', 1],
    ['"Nastavení → Aplikace → Jazyk a průvodce"', 0],
    ['"(Outlook na webu → Nastavení → Kalendář → Sdílené kalendáře)"', 0],
    ['"Klíč propojené aplikace (Nastavení → MCP)"', 0],
    ['"(Nastavení → Soukromí → Rádia)"', 0],
    ['// Nastavení → Útrata (komentář)', 0],
    ['"Nastavení → Projekty, odkazy a akce → Z Launcheru…"', 0],
  ];
  for (const [src, n] of cases) assert.equal(badPaths("x", src).length, n, src);
});

test("SECTIONS: stránka existuje, anglický název je ve slovníku, kotva je skutečný nadpis ve zdrojích", () => {
  const ids = new Set(PANEL_META.map((p) => p.id));
  const all = [...TEXT.values()].join("\n");
  for (const [id, s] of Object.entries(SECTIONS)) {
    assert.ok(ids.has(s.panel), `${id}: stránka ${s.panel}`);
    assert.ok(EN.get(s.key), `${id}: chybí anglický název ${s.key}`);
    const esc = s.anchor.replace(/\./g, "\\.");
    const viaT = [...all.matchAll(new RegExp(`"${esc}",\\s*"((?:[^"\\\\]|\\\\.)*)"`, "g"))].map((m) => m[1]);
    const viaField = [...all.matchAll(new RegExp(`"${esc}":\\s*\\{[^}]*?label:\\s*"([^"]*)"`, "g"))].map((m) => m[1]);
    assert.ok([...viaT, ...viaField].some((cs) => cs.startsWith(s.cs)), `${id}: kotva ${s.anchor} nemá nadpis začínající „${s.cs}“`);
  }
  for (const p of PANEL_META) assert.ok(EN.get(p.key), `stránka ${p.id}: chybí anglický název ${p.key}`);
});
