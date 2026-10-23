// Sbalitelné „Pokročilé“ v panelech Nastavení: paměť rozbalení (čistá logika `advModel.ts`) a rozmístění
// technických voleb — kde je co v „Pokročilé“ a že panel Účty zmizel z navigace, ale účty jdou dohledat.
//   node --experimental-strip-types --test src/settings/advanced.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { ADV_KEY, parseAdvOpen, withAdv } from "./advModel.ts";
import { search } from "./model.ts";
import { PANEL_META } from "./registry.ts";

const read = (f) => readFileSync(new URL(f, import.meta.url), "utf8");

test("parseAdvOpen: platné záznamy projdou, cokoli jiného = vše sbaleno", () => {
  const cases = [
    [null, {}],
    [undefined, {}],
    ["", {}],
    ["nesmysl", {}],
    ["[]", {}],
    ["null", {}],
    ["42", {}],
    ['{"claude":true}', { claude: true }],
    ['{"claude":true,"akce":false,"x":1,"y":"ano"}', { claude: true }],
    [`{"${"a".repeat(65)}":true}`, {}],
    ['{"":true}', {}],
  ];
  for (const [raw, want] of cases) assert.deepEqual(parseAdvOpen(raw), want, String(raw));
  assert.equal(ADV_KEY, "ledge.settings.advanced");
});

test("withAdv: rozbalení se přidá, sbalení vypustí; původní mapa se nemění", () => {
  const a = { claude: true };
  const b = withAdv(a, "akce", true);
  assert.deepEqual(b, { claude: true, akce: true });
  assert.deepEqual(a, { claude: true });
  assert.deepEqual(withAdv(b, "claude", false), { akce: true });
  assert.deepEqual(withAdv({}, "nic", false), {});
});

/** Obsah prvního `<tag …>…</tag>` s ohledem na vnoření (JSX jako text). */
function regions(text, tag) {
  const out = [];
  const open = new RegExp(`<${tag}[\\s>]`, "g");
  for (let m; (m = open.exec(text)); ) {
    let depth = 1;
    const re = new RegExp(`<${tag}[\\s>]|</${tag}>`, "g");
    re.lastIndex = m.index + 1;
    for (let n; (n = re.exec(text)); ) {
      depth += n[0].startsWith("</") ? -1 : 1;
      if (depth === 0) {
        out.push(text.slice(m.index, n.index));
        break;
      }
    }
  }
  return out;
}
const inAdv = (file, needle) => regions(read(file), "Advanced").some((r) => r.includes(needle));

test("technické volby jsou v Pokročilé, ne mezi základními", () => {
  const where = [
    ["panels/Claude.tsx", 'id="claude.port"'],
    ["../mcp/McpSettings.tsx", 'id="mcp.desktop"'],
    ["../mcp/McpSettings.tsx", 'id="mcp.key"'],
    ["panels/Notes.tsx", 'id="notes.sync.remote"'],
    ["panels/Notes.tsx", 'id="notes.sync.intervalMin"'],
    ["panels/Personal.tsx", 'id="personal.weather.lat"'],
    ["panels/Personal.tsx", "<AccountsSettings"],
    ["panels/AppPanel.tsx", "<BackupAdvanced"],
    ["panels/AppPanel.tsx", "<AboutAdvanced"],
    ["panels/ProjectsActions.tsx", "<ScriptsSettings"],
    ["panels/ProjectsActions.tsx", "<WidgetsSettings"],
  ];
  const bad = where.filter(([f, n]) => !inAdv(f, n)).map(([f, n]) => `${f}: ${n}`);
  assert.deepEqual(bad, []);
  // a opravdu mimo ně už nejsou
  assert.ok(!read("panels/Actions.tsx").includes("ScriptsSettings"), "skripty nejsou v seznamu akcí");
  for (const id of ["update.repo", "about.files"]) assert.ok(read("panels/About.tsx").indexOf(`id="${id}"`) > read("panels/About.tsx").indexOf("export function AboutAdvanced"), id);
  assert.ok(read("panels/Backup.tsx").indexOf('id="backup.file"') > read("panels/Backup.tsx").indexOf("export function BackupAdvanced"));
});

test("Pokročilé se rozbalí skokem na volbu uvnitř a stav drží jen v try/catch úložišti", () => {
  const src = read("fields/Advanced.tsx");
  assert.match(src, /on\(hit,/, "sleduje zvýrazněnou kotvu");
  assert.match(src, /querySelector\(`\[id="f-/, "hledá řádek uvnitř sebe");
  assert.equal((src.match(/localStorage\./g) ?? []).length, 2);
  assert.equal((src.match(/try \{/g) ?? []).length, 2, "čtení i zápis v try/catch");
});

test("panel Účty není v navigaci; účty jdou dohledat hledáním i skokem", () => {
  assert.ok(!PANEL_META.some((p) => p.id === "ucty"));
  const labels = read("labels.ts");
  assert.ok(!labels.includes('panel: "ucty"'));
  const items = [];
  for (const l of labels.split("\n")) {
    const m = l.match(/^\s*"?([\w.]+)"?: \{ panel: "(\w+)"/);
    if (m) items.push({ panel: m[2], anchor: m[1], label: l.match(/label: "((?:[^"\\]|\\.)*)"/)?.[1] ?? "", hint: l.match(/hint: "((?:[^"\\]|\\.)*)"/)?.[1] ?? "", kw: l.match(/kw: "((?:[^"\\]|\\.)*)"/)?.[1] ?? "" });
  }
  for (const [q, anchor] of [["client id", "accounts.microsoft.clientId"], ["azure", "accounts.microsoft.clientId"], ["ucet google", "accounts.google"], ["outlook prihlasit", "accounts.microsoft"], ["pokrocile", null]]) {
    if (anchor) assert.ok(search(items, q).some((x) => x.anchor === anchor && x.panel === "osobni"), `„${q}“ najde ${anchor} v Osobní`);
  }
});

test("štítek Platí hned: jedna komponenta, použitá u voleb mimo lištu Uložit", () => {
  const row = read("fields/Row.tsx");
  assert.equal((row.match(/export function LiveBadge/g) ?? []).length, 1);
  assert.match(row, /p\.live/);
  for (const [f, n] of [["../mcp/McpSettings.tsx", 'id="mcp.enabled" live'], ["panels/Claude.tsx", 'id="claude.hooks" stack live'], ["panels/Claude.tsx", 'id="claude.statusLine" stack live']]) assert.ok(read(f).includes(n), `${f}: ${n}`);
  assert.ok(read("../accounts/AccountsSettings.tsx").includes("<LiveBadge />"));
});
