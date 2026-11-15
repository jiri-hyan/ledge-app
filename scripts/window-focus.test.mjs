// „Ledge nekrade fokus“ (nepřekročitelné pravidlo č. 1) — statický strážce míst, kde vzniká okno. Čte zdroje
// Rust jako ostatní zrcadlové testy (ipc-window-acl, config-keys): každé místo, které vytváří okno
// (`CreateWindowExW(`, `WebviewWindowBuilder::new(`, `win::create_window(`), musí být v tabulce MISTA a splnit
// její požadavky na styl (NOACTIVATE / `.focused(false)` / SW_SHOWNOACTIVATE / HWND_MESSAGE) — nebo být
// VÝJIMKA s důvodem. Nové okno mimo tabulku = červený test, dokud někdo vědomě nerozhodne o jeho stylu.
// Proč statický test: GUI (myš, okna) se v testech spouštět nesmí; bez něj by okno bez NOACTIVATE prošlo všemi branami.
// Audit J6 / Q (kolo 34). Tabulka: docs/codemap/edge.md (okna a styly).
import { test } from "node:test"
import assert from "node:assert/strict"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const src = path.join(root, "src-tauri", "src")

/** Co vytváří okno (v produkčním kódu, bez komentářů). */
export const CREATORS = [
  { name: "CreateWindowExW", re: /\bCreateWindowExW\s*\(/g },
  { name: "WebviewWindowBuilder", re: /\bWebviewWindowBuilder::new\s*\(/g },
  { name: "win::create_window", re: /\bwin::create_window\s*\(/g },
]

const NOACT = "WS_EX_NOACTIVATE"
// kolo 34: styl bez aktivace žije v `winstyle.rs`; okna ho dostávají voláním (při vzniku `no_activate_ex`, za běhu `no_activate`)
const NOACT_EX = "winstyle::no_activate_ex"
const NOACT_FN = "winstyle::no_activate"
const PROFILE = "winstyle::apply_profile"
const FALSE = ".focused(false)"
const SHOWNA = "SW_SHOWNOACTIVATE"

/**
 * soubor → { creates: {Tvůrce: počet}, need: [podřetězce, které musí být v kódu souboru], exception?: důvod }.
 * `exception` = okno schválně NEMÁ plnou ochranu; musí mít důvod. `need` se přesto ověřuje (co z ochrany platí).
 * Nové okno v už uvedeném souboru zvýší počet a test selže, dokud se řádek vědomě neupraví.
 */
export const MISTA = {
  "strip.rs": { creates: { CreateWindowExW: 1 }, need: [NOACT_EX, SHOWNA], what: "proužek (nativní D2D, LAYERED/TRANSPARENT/TOPMOST)" },
  "shelf/drop.rs": { creates: { CreateWindowExW: 1 }, need: [NOACT_EX], what: "cíl přetažení přihrádky" },
  "drawer.rs": { creates: { WebviewWindowBuilder: 1 }, need: [NOACT_FN, FALSE, SHOWNA], what: "šuplík (NOACTIVATE se při výslovném kliku odebírá a vrací)" },
  "ext/wall/mod.rs": { creates: { "win::create_window": 2 }, need: [NOACT, SHOWNA], count: [NOACT, 2], what: "watcher a okno tapety (obě NOACTIVATE)" },
  "ext/host/win.rs": { creates: { CreateWindowExW: 1 }, need: [], what: "obecný `create_window`: styl určuje volající (ext/wall/mod.rs — viz výše)" },
  "cliphist/win.rs": { creates: { CreateWindowExW: 1 }, need: ["HWND_MESSAGE"], what: "message-only okno schránky (nezobrazuje se)" },
  "rawpush.rs": { creates: { CreateWindowExW: 1 }, need: ["HWND_MESSAGE"], what: "message-only okno raw vstupu (nezobrazuje se)" },
  "notes/pdf.rs": {
    creates: { WebviewWindowBuilder: 1 },
    need: [FALSE, ".visible(false)"],
    exception: "skryté tiskové okno bez NOACTIVATE: nikdy se nezobrazí (visible(false) + focused(false)); styl přidat bezpečně lze, ale nemá co chránit",
    what: "skryté okno pro tisk PDF",
  },
  "desk/win.rs": { creates: { WebviewWindowBuilder: 1 }, need: [PROFILE, "Win::Desk", FALSE, SHOWNA], what: "widgety na ploše (profil Desk po build; nikdy se neaktivují)" },
  "pins/win.rs": { creates: { WebviewWindowBuilder: 1 }, need: [PROFILE, "Win::Pin", NOACT_FN, FALSE], what: "lístky (profil Pin po build; NOACTIVATE se odebírá výslovným klikem `pin_activate`)" },
  "webpanel.rs": {
    creates: { WebviewWindowBuilder: 1 },
    need: [],
    exception: "`.focused(true)` záměrně: panel se otvírá výslovnou akcí uživatele a má být aktivní",
    what: "webové panely",
  },
  "studio.rs": {
    creates: { WebviewWindowBuilder: 1 },
    need: [],
    exception: "Studio/Nastavení se otvírá výslovnou akcí a je to běžné okno v panelu úloh — záměrně aktivní",
    what: "Studio / Nastavení",
  },
}

/** Zdroj bez komentářů `//` a bez bloků `#[cfg(test)] mod tests { … }` (od `#[cfg(test)]` do konce souboru). */
export function productionCode(text) {
  const t = text.replace(/\r\n/g, "\n")
  const i = t.search(/^#\[cfg\(test\)\]\s*\n\s*mod\s+\w+\s*\{/m)
  const prod = i >= 0 ? t.slice(0, i) : t
  return prod.split("\n").map((l) => l.replace(/\/\/.*$/, "")).join("\n")
}

/** Počty tvůrců oken v kódu. */
export function countCreators(code) {
  const out = {}
  for (const { name, re } of CREATORS) {
    // `win::create_window(` obsahuje podřetězec `create_window`, ale ne `CreateWindowExW` — počty se nepletou
    const n = (code.match(re) ?? []).length
    if (n) out[name] = n
  }
  return out
}

/** Porušení tabulkového řádku: pole chyb (prázdné = OK). Čistá funkce, testovaná níže na vzorcích. */
export function violations(code, spec) {
  const errs = []
  const got = countCreators(code)
  for (const k of new Set([...Object.keys(got), ...Object.keys(spec.creates)])) {
    if ((got[k] ?? 0) !== (spec.creates[k] ?? 0)) errs.push(`${k}: v kódu ${got[k] ?? 0}×, v tabulce ${spec.creates[k] ?? 0}×`)
  }
  for (const s of spec.need) if (!code.includes(s)) errs.push(`chybí ${s}`)
  if (spec.count) {
    const [s, n] = spec.count
    const have = code.split(s).length - 1
    if (have < n) errs.push(`${s} aspoň ${n}×, je ${have}×`)
  }
  return errs
}

function* rsFiles(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name)
    if (e.isDirectory()) yield* rsFiles(p)
    else if (e.name.endsWith(".rs") && !/(^|[\\/])tests?\.rs$|_tests\.rs$/.test(p)) yield p
  }
}

test("každé místo, které vytváří okno, je v tabulce a splňuje styl", () => {
  const found = {}
  for (const f of rsFiles(src)) {
    const rel = path.relative(src, f).split(path.sep).join("/")
    const code = productionCode(fs.readFileSync(f, "utf8"))
    if (Object.keys(countCreators(code)).length) found[rel] = code
  }
  const errs = []
  for (const rel of Object.keys(found)) {
    if (!(rel in MISTA)) errs.push(`${rel}: vytváří okno, ale není v MISTA (scripts/window-focus.test.mjs) — rozhodni o stylu (NOACTIVATE / focused(false) / výjimka s důvodem)`)
  }
  for (const [rel, spec] of Object.entries(MISTA)) {
    if (!(rel in found)) { errs.push(`${rel}: v tabulce, ale okno už nevytváří — zastaralý řádek`); continue }
    for (const e of violations(found[rel], spec)) errs.push(`${rel} (${spec.what}): ${e}`)
  }
  assert.deepEqual(errs, [])
})

test("výjimky mají důvod; jen okna se zobrazením bez NOACTIVATE smí být výjimka", () => {
  for (const [rel, spec] of Object.entries(MISTA)) {
    if (spec.exception !== undefined) assert.ok(spec.exception.length > 30, `${rel}: výjimka bez důvodu`)
    const code = productionCode(fs.readFileSync(path.join(src, rel), "utf8"))
    // bez výjimky a bez message-only okna se vyžaduje NOACTIVATE (přímo, nebo v kódu volajícího)
    const message = spec.need.includes("HWND_MESSAGE")
    const generic = rel === "ext/host/win.rs"
    if (spec.exception === undefined && !message && !generic) assert.ok(code.includes(NOACT) || code.includes(NOACT_FN) || code.includes(PROFILE), `${rel}: bez výjimky musí mít ${NOACT}, ${NOACT_FN} nebo ${PROFILE}`)
    if (spec.exception !== undefined) assert.ok(!(code.includes(NOACT) || code.includes(NOACT_FN) || code.includes(PROFILE)) || rel === "drawer.rs", `${rel}: výjimka je zastaralá — soubor už ${NOACT} má, přesuň řádek mezi plné`)
  }
})

test("winstyle.rs: sdílený styl opravdu nese NOACTIVATE + TOOLWINDOW (jinak by volání `winstyle::` nic nechránilo)", () => {
  const code = productionCode(fs.readFileSync(path.join(src, "winstyle.rs"), "utf8"))
  for (const s of [NOACT, "WS_EX_TOOLWINDOW", "pub fn no_activate_ex", "pub fn no_activate(", "pub fn apply_profile", "pub fn profile"]) assert.ok(code.includes(s), `winstyle.rs: chybí ${s}`)
})

test("violations: detektor opravdu selže na okně bez stylu", () => {
  const spec = { creates: { CreateWindowExW: 1 }, need: [NOACT] }
  assert.deepEqual(violations("CreateWindowExW(ex, c) WS_EX_NOACTIVATE", spec), [])
  assert.deepEqual(violations("CreateWindowExW(ex, c)", spec), ["chybí WS_EX_NOACTIVATE"])
  const ex = { creates: { CreateWindowExW: 1 }, need: [NOACT_EX] }
  assert.deepEqual(violations("CreateWindowExW(ex | crate::winstyle::no_activate_ex(), c)", ex), [])
  assert.deepEqual(violations("CreateWindowExW(WS_EX_TOPMOST, c)", ex), ["chybí winstyle::no_activate_ex"])
  assert.match(violations("CreateWindowExW(a); CreateWindowExW(b); WS_EX_NOACTIVATE", spec)[0], /v kódu 2×, v tabulce 1×/)
  assert.match(violations("WebviewWindowBuilder::new(app, l, u).focused(false)", { creates: {}, need: [] })[0], /v kódu 1×, v tabulce 0×/)
  assert.deepEqual(violations("win::create_window(a) win::create_window(b) WS_EX_NOACTIVATE WS_EX_NOACTIVATE", { creates: { "win::create_window": 2 }, need: [NOACT], count: [NOACT, 2] }), [])
  assert.match(violations("win::create_window(a) win::create_window(b) WS_EX_NOACTIVATE", { creates: { "win::create_window": 2 }, need: [NOACT], count: [NOACT, 2] })[0], /aspoň 2×/)
})

test("productionCode: komentáře a testovací modul se nepočítají", () => {
  const t = "fn a() { // CreateWindowExW(\n}\nfn b() { CreateWindowExW(x) }\n#[cfg(test)]\nmod tests {\n  fn c() { CreateWindowExW(y) }\n}\n"
  assert.deepEqual(countCreators(productionCode(t)), { CreateWindowExW: 1 })
})
