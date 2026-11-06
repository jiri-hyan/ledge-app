// Co okno volá ⊆ co smí (capabilities). Čistý node test bez buildu: od vstupních HTML projde graf importů
// (statických i `import()`), z wrapperů `obj.name: … call("příkaz")` a přímých `invoke/call("příkaz")` spočítá příkazy,
// které soubory okna používají, a porovná je s `src-tauri/capabilities/*.json`. Proč: `capabilities_match_tables`
// (ipcacl.rs) hlídá jen JSON × ruční tabulku; Studiu tak 5 volaných příkazů chybělo a tlačítka končila chybou ACL (audit kola 29).
// Přibližnost: použití wrapperu se hledá jako `obj.name` v souborech okna (nad-odhad), proto existuje seznam VÝJIMEK s důvodem.
import { test } from "node:test"
import assert from "node:assert/strict"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const norm = (p) => p.split(path.sep).join("/")

/** okno → vstupní HTML a capability soubor */
export const WINDOWS = {
  drawer: { html: "index.html", cap: "default.json" },
  studio: { html: "settings.html", cap: "studio.json" },
  pin: { html: "pin.html", cap: "pins.json" },
  desk: { html: "desk.html", cap: "desk.json" },
}

/**
 * Známé výjimky: příkaz, který kód okna statisticky dosáhne (sdílené moduly), ale za běhu ho tam nikdo nezavolá,
 * nebo je to vědomě odepřeno (bezpečnostní hranice). Každá s důvodem; nový záznam = vědomé rozhodnutí.
 * Test zároveň hlídá, že výjimka není zastaralá (příkaz okno už nevolá, nebo je povolený).
 */
const GUARD = "api.ts `drawerOnly`: ve Studiu (`inStudio`) vrací undefined bez volání; capability je schválně nepovoluje"
const IMPORT_ONLY = (what) => `Studio z toho modulu importuje jen ${what}, příkaz se tam nevolá`
const SHELL = "sdílený modul rozvržení šuplíku; ve widgetu na ploše není stránka k úpravě ani velikost k měnění"
export const EXCEPTIONS = {
  drawer: {},
  studio: {
    studio_open: "palette/goto.ts: ve Studiu (`inStudio`) se vrací dřív, než se příkaz zavolá",
    set_mode: GUARD,
    drawer_size: GUARD,
    drawer_size_end: GUARD,
    focus_drawer: GUARD,
    release_focus: GUARD,
    clip_list: IMPORT_ONLY("favErrText (media/FavoriteTiles.tsx)"),
    media_favorite_open: IMPORT_ONLY("favErrText (media/FavoriteTiles.tsx)"),
    onboarding_repos: IMPORT_ONLY("resetWizard (onboarding/store.ts); průvodce běží v šuplíku"),
    onboarding_apps: IMPORT_ONLY("resetWizard (onboarding/store.ts); průvodce běží v šuplíku"),
    onboarding_claude: IMPORT_ONLY("resetWizard (onboarding/store.ts); průvodce běží v šuplíku"),
    onboarding_pick_folder: IMPORT_ONLY("resetWizard (onboarding/store.ts); průvodce běží v šuplíku"),
    onboarding_themes: IMPORT_ONLY("resetWizard (onboarding/store.ts); průvodce běží v šuplíku"),
    onboarding_apply: IMPORT_ONLY("resetWizard (onboarding/store.ts); průvodce běží v šuplíku"),
    widget_output: IMPORT_ONLY("WidgetConsent a intervalText (widgets/WidgetCard.tsx), ne kartu"),
    widget_click: IMPORT_ONLY("WidgetConsent a intervalText (widgets/WidgetCard.tsx), ne kartu"),
  },
  pin: {},
  desk: {
    set_mode: SHELL,
    drawer_size: SHELL,
    drawer_size_end: SHELL,
    focus_drawer: SHELL,
    release_focus: SHELL,
    studio_win: "layout/edit.ts `askEdit` je jen ve Studiu",
    desk_pin: "desk/pin.tsx je ovládání šuplíku (připnout druh), okno widgetu ho nevykresluje",
    // F7 (audit kola 29): bezpečnostní hranice, okno plochy se NEROZŠIŘUJE. Sdílené karty příkaz dosáhnou, ACL ho odepře
    // (toast s chybou); řešení patří do UI (skrýt položku / `desk_open_drawer`), ne do capability.
    save_config: "F7: CalendarCard (`readOnly` jen z části), WidgetCard `toggleStrip`, LimitsSources; plocha config zapisovat nesmí",
    studio_open: "F7: palette/goto.ts `openSettings`; na ploše tiše nic (fallback), Nastavení se otevírá přes desk_open_drawer",
    widget_consent: "F7: souhlas se skriptem smí dát jen člověk v šuplíku",
    costs_refresh: "UsageCard má `!p.desk`; CostsSection se na ploše nevykresluje",
  },
}

/** `allow-foo-bar` → `foo_bar` (jako `permission_of` v ipcacl.rs) */
export const commandOf = (perm) => perm.replace(/^allow-/, "").replace(/-/g, "_")

export function allowedCommands(capJson) {
  return new Set(capJson.permissions.filter((p) => p.startsWith("allow-")).map(commandOf))
}

function walk(d, out = []) {
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    const p = path.join(d, e.name)
    if (e.isDirectory()) walk(p, out)
    else out.push(norm(p))
  }
  return out
}

const CALL = String.raw`(?:invoke|call|demoCall)\w*\s*(?:<[^>()]*(?:<[^>]*>)?[^>()]*>)?\(\s*["'\x60](\w+)["'\x60]`
const WRAP_HEAD = new RegExp(String.raw`^\s{2,4}(\w+):\s*(?:async\s*)?\([^\n]*?\)\s*(?::\s*[^=]+)?=>\s*(?:drawerOnly\(\(\)\s*=>\s*)?`)
const WRAP_ONE = new RegExp(WRAP_HEAD.source + CALL)
const WRAP_OPEN = new RegExp(String.raw`^\s{2,4}(\w+):[^\n]*=>\s*$`)
const NEXT_CALL = new RegExp(String.raw`^\s+` + CALL)
const DIRECT = new RegExp(String.raw`\b` + CALL)

/** Z textu zdrojů: wrappery {obj,name,cmd} (i s voláním na dalším řádku) a přímá volání {file,cmd}. */
export function scanCalls(files, src) {
  const wrap = []
  const direct = []
  for (const f of files) {
    let obj = null
    let open = null // wrapper, jehož volání je až na dalším řádku
    for (const l of src[f].split("\n")) {
      const mo = /^(?:export )?const (\w+)\s*=\s*\{\s*$/.exec(l)
      if (mo) obj = mo[1]
      const prev = open
      open = WRAP_OPEN.exec(l)?.[1] ?? null
      const one = WRAP_ONE.exec(l)
      const next = prev ? NEXT_CALL.exec(l) : null
      if (one) wrap.push({ file: f, obj, name: one[1], cmd: one[2] })
      else if (next) wrap.push({ file: f, obj, name: prev, cmd: next[1] })
      else {
        const md = DIRECT.exec(l)
        if (md) direct.push({ file: f, cmd: md[1] })
      }
    }
  }
  return { wrap, direct }
}

export function importsOf(f, text, has) {
  const out = new Set()
  const re = /(?:from\s+|import\s*\(\s*|import\s+)["']([^"']+)["']/g
  let m
  while ((m = re.exec(text))) {
    if (!m[1].startsWith(".")) continue
    const base = norm(path.resolve(path.dirname(f), m[1]))
    for (const c of [base, base + ".ts", base + ".tsx", base + "/index.ts", base + "/index.tsx"]) if (has(c)) out.add(c)
  }
  return out
}

export function closure(entry, src) {
  const seen = new Set([entry])
  const q = [entry]
  while (q.length) {
    const f = q.pop()
    for (const d of importsOf(f, src[f], (c) => c in src)) {
      if (!seen.has(d)) {
        seen.add(d)
        q.push(d)
      }
    }
  }
  return seen
}

/** příkaz → soubory okna, které ho (nejspíš) volají */
export function usedBy(files, src, calls) {
  const used = {}
  const add = (cmd, f) => (used[cmd] ??= new Set()).add(f)
  for (const w of calls.wrap) {
    if (!w.obj) continue
    const rx = new RegExp(String.raw`(?<![\w$])${w.obj}[.]${w.name}(?![\w$])`)
    for (const f of files) if (rx.test(src[f])) add(w.cmd, f)
  }
  for (const d of calls.direct) if (files.has(d.file)) add(d.cmd, d.file)
  return used
}

/** bez komentářů: zmínka `api.xxx` v dokumentačním řádku není použití */
const STR_OR_COMMENT = new RegExp(
  String.raw`("(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|\x60(?:\\.|[^\x60\\])*\x60)|/\*[^]*?\*/|//[^\n]*`,
  "g",
)
export const stripComments = (t) => t.replace(STR_OR_COMMENT, (_m, str) => str ?? "")

export function load() {
  const all = walk(path.join(root, "src")).filter((f) => /\.tsx?$/.test(f) && !/\.test\./.test(f))
  const src = Object.fromEntries(all.map((f) => [f, stripComments(fs.readFileSync(f, "utf8"))]))
  return { src, calls: scanCalls(all, src) }
}

export function entryOf(html) {
  const t = fs.readFileSync(path.join(root, html), "utf8")
  const m = /<script[^>]*src="\/?([^"]+)"/.exec(t)
  assert.ok(m, `${html}: bez <script src>`)
  return norm(path.join(root, m[1]))
}

/** příkazy volané oknem, které capability nepovoluje (před výjimkami) → soubory */
export function violations(win, ctx = load()) {
  const { html, cap } = WINDOWS[win]
  const files = closure(entryOf(html), ctx.src)
  const used = usedBy(files, ctx.src, ctx.calls)
  const allowed = allowedCommands(JSON.parse(fs.readFileSync(path.join(root, "src-tauri/capabilities", cap), "utf8")))
  return Object.fromEntries(
    Object.entries(used)
      .filter(([c]) => !allowed.has(c))
      .map(([c, set]) => [c, [...set].map((f) => norm(path.relative(root, f))).sort()]),
  )
}

const ctx = load()
for (const win of Object.keys(WINDOWS)) {
  test(`okno ${win}: co volá ⊆ ACL okna`, () => {
    const bad = violations(win, ctx)
    const ex = EXCEPTIONS[win]
    const open = Object.keys(bad).filter((c) => !(c in ex))
    assert.deepEqual(
      open.map((c) => `${c} ← ${bad[c].slice(0, 3).join(", ")}`),
      [],
      `okno ${win} volá příkazy, které capabilities/${WINDOWS[win].cap} nepovoluje (doplň ACL, nebo zapiš výjimku s důvodem)`,
    )
    const stale = Object.keys(ex).filter((c) => !(c in bad))
    assert.deepEqual(stale, [], `zastaralé výjimky (okno příkaz už nedosáhne, nebo je povolený): ${stale.join(", ")}`)
  })
}

test("studio: dřív chybějící příkazy Nastavení jsou povolené", () => {
  const cap = JSON.parse(fs.readFileSync(path.join(root, "src-tauri/capabilities/studio.json"), "utf8"))
  const ok = allowedCommands(cap)
  for (const c of ["repos_rescan", "update_install", "widget_refresh", "widget_edit", "system_view"]) assert.ok(ok.has(c), c)
})

// --- čisté funkce nad syntetickými zdroji ---
test("commandOf / allowedCommands", () => {
  assert.equal(commandOf("allow-repos-rescan"), "repos_rescan")
  assert.deepEqual([...allowedCommands({ permissions: ["core:event:allow-listen", "allow-a-b"] })], ["a_b"])
})

test("scanCalls + usedBy: wrapper se počítá jen tam, kde se použije; volání na dalším řádku je wrapper", () => {
  const src = {
    "/a/api.ts": 'export const api = {\n  foo: (id: string) => call<void>("cmd_foo", { id }),\n  bar: () => call("cmd_bar"),\n  baz: (x: number) =>\n    call<void>("cmd_baz", { x }),\n}\n',
    "/a/x.ts": 'import { api } from "./api"\napi.foo("1"); api.baz(1); void invoke("raw_cmd")\n',
  }
  const files = Object.keys(src)
  const calls = scanCalls(files, src)
  assert.deepEqual(calls.direct.map((d) => d.cmd), ["raw_cmd"])
  const used = usedBy(new Set(files), src, calls)
  assert.deepEqual(Object.keys(used).sort(), ["cmd_baz", "cmd_foo", "raw_cmd"])
  assert.deepEqual(Object.keys(usedBy(new Set(["/a/api.ts"]), src, calls)), [])
})

test("stripComments: řádkové i blokové komentáře pryč, URL v řetězci zůstane", () => {
  assert.equal(stripComments("a // b\nc"), "a \nc")
  assert.equal(stripComments("a /* x\ny */ b"), "a  b")
  assert.equal(stripComments('u = "http://example.com"'), 'u = "http://example.com"')
})
