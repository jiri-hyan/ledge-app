// Lint hniloby codemap: tabulky v docs/codemap/*.md tvrdí, které soubory a symboly existují. Když kód
// uteče (přejmenování, přesun do podsložky), mapa lže a worker pak čte špatná místa — tohle to chytí.
// Čisté funkce (bez I/O) + `loadTree` jako jediné I/O; test: scripts/codemap.test.mjs.
//   node scripts/codemap-lint.mjs      → vypíše nálezy (exit 1, je-li nějaký)
//
// Pravidla:
//  (a) sloupec „soubor…“: backtickové cesty (obsahují `/` nebo končí .rs/.ts/.tsx/.css/.mjs) musí existovat.
//      `{a,b}` se rozbalí, `*` je glob. Cesta může být zkrácená (`conf.rs` v calendar.md, `cards/x.ts`
//      v ui.md) — stačí, aby ji některý soubor/složka v repu měl jako příponu cesty. `x.rs` ≙ `x/mod.rs`,
//      `x.ts(x)` ≙ `x/index.ts(x)` (modul rozdělený do složky).
//  (b) sloupec „… symboly“: backtickové čisté identifikátory se musí vyskytovat jako slovo v kódu
//      (src/**, src-tauri/src/**, scripts/**, pin.html, index.html).
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const SKIP_DIRS = new Set(['node_modules', 'target', 'target-agents', 'dist', '.git'])
const CODE_ROOTS = ['src', 'src-tauri/src', 'scripts']
const CODE_FILES = ['pin.html', 'index.html']
const CODE_EXT = /\.(rs|ts|tsx|mjs|js|css|html|ps1)$/
// Soubory, které samy obsahují vymyšlené symboly (testovací data) — nesmí „dokazovat“ existenci symbolů.
const NOT_CORPUS = new Set(['scripts/codemap.test.mjs', 'scripts/codemap-lint.mjs'])
const PATH_EXT = /\.(rs|ts|tsx|css|mjs)$/
const IDENT = /^[A-Za-z_][A-Za-z0-9_]*$/

// ---------- čisté funkce ----------

/** `a/{b,c}.rs` → [`a/b.rs`, `a/c.rs`] (rekurzivně, bez vnořených chyb). */
export function expandBraces(p) {
  const m = /\{([^{}]*)\}/.exec(p)
  if (!m) return [p]
  return m[1].split(',').flatMap((alt) => expandBraces(p.slice(0, m.index) + alt + p.slice(m.index + m[0].length)))
}

export function isPathToken(t) {
  return !/\s/.test(t) && (t.includes('/') || PATH_EXT.test(t))
}

function globRe(pattern) {
  let re = ''
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i]
    if (c === '*') {
      if (pattern[i + 1] === '*') { re += '.*'; i++ } else re += '[^/]*'
    } else re += c.replace(/[.+?^${}()|[\]]/g, (x) => String.fromCharCode(92) + x)
  }
  return re
}

/** Existuje cesta `p` mezi `entries` (relativní cesty souborů i složek, lomítka `/`)? */
export function pathExists(p, entries) {
  const clean = p.replace(/^\.\//, '').replace(/\/+$/, '')
  const variants = []
  for (const q of expandBraces(clean)) {
    variants.push(q)
    if (q.endsWith('.rs')) variants.push(q.slice(0, -3) + '/mod.rs')
    else if (/\.tsx?$/.test(q)) variants.push(q.replace(/\.tsx?$/, (e) => '/index' + e))
  }
  return variants.some((v) => {
    const re = new RegExp('(^|/)' + globRe(v) + '$')
    return entries.some((e) => re.test(e))
  })
}

/** Markdown → tabulky: [{ header: [..], rows: [{ line, cells: [..] }] }]. */
export function parseTables(md) {
  const lines = md.split(/\r?\n/)
  const tables = []
  let cur = null
  const BS_PIPE = String.fromCharCode(92) + '|' // v buňce zalomené svislítko
  const MARK = String.fromCharCode(1)
  const split = (l) => l.trim().split(BS_PIPE).join(MARK).replace(/^[|]/, '').replace(/[|]$/, '').split('|').map((c) => c.split(MARK).join('|').trim())
  lines.forEach((l, i) => {
    if (!l.trim().startsWith('|')) { cur = null; return }
    const cells = split(l)
    if (!cur) { cur = { header: cells.map((c) => c.toLowerCase()), rows: [] }; tables.push(cur); return }
    if (cells.every((c) => /^:?-{2,}:?$/.test(c))) return
    cur.rows.push({ line: i + 1, cells })
  })
  return tables
}

const ticks = (cell) => [...cell.matchAll(/`([^`]+)`/g)].map((m) => m[1].trim())

/**
 * Najde zastaralé položky v jednom codemap souboru.
 * ctx: { entries: string[], words: Set<string>, allowFiles?: Set<string>, allowSymbols?: Set<string> }
 * → [{ kind: 'file' | 'symbol', value, line }]
 */
export function lintMarkdown(md, ctx) {
  const out = []
  for (const t of parseTables(md)) {
    const fileCols = t.header.flatMap((h, i) => (h.startsWith('soubor') ? [i] : []))
    const symCols = t.header.flatMap((h, i) => (/symboly/.test(h) ? [i] : []))
    for (const r of t.rows) {
      for (const i of fileCols) {
        for (const tok of ticks(r.cells[i] ?? '')) {
          if (!isPathToken(tok) || ctx.allowFiles?.has(tok)) continue
          if (!pathExists(tok, ctx.entries)) out.push({ kind: 'file', value: tok, line: r.line })
        }
      }
      for (const i of symCols) {
        for (const tok of ticks(r.cells[i] ?? '')) {
          if (!IDENT.test(tok) || ctx.allowSymbols?.has(tok)) continue
          if (!ctx.words.has(tok)) out.push({ kind: 'symbol', value: tok, line: r.line })
        }
      }
    }
  }
  return out
}

// ---------- I/O ----------

function walk(dir, root, files, dirs) {
  let ents
  try { ents = fs.readdirSync(dir, { withFileTypes: true }) } catch { return }
  for (const e of ents) {
    if (SKIP_DIRS.has(e.name)) continue
    const p = path.join(dir, e.name)
    const rel = path.relative(root, p).split(path.sep).join('/')
    if (e.isDirectory()) { dirs.push(rel); walk(p, root, files, dirs) } else files.push(rel)
  }
}

/** Načte strom repa: entries (soubory + složky) a množinu slov z kódu. */
export function loadTree(root) {
  const files = []
  const dirs = []
  walk(root, root, files, dirs)
  const words = new Set()
  const code = files.filter((f) => !NOT_CORPUS.has(f) && CODE_EXT.test(f) &&
    (CODE_FILES.includes(f) || CODE_ROOTS.some((r) => f.startsWith(r + '/'))))
  for (const f of code) {
    for (const w of fs.readFileSync(path.join(root, f), 'utf8').matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) words.add(w[0])
  }
  return { entries: [...files, ...dirs], words }
}

/** Projde docs/codemap/*.md a vrátí nálezy [{ file, kind, value, line }]. */
export function lintCodemaps(root, opts = {}) {
  const tree = loadTree(root)
  const ctx = { ...tree, ...opts }
  const dir = path.join(root, 'docs', 'codemap')
  const found = []
  for (const n of fs.readdirSync(dir).filter((x) => x.endsWith('.md')).sort()) {
    for (const f of lintMarkdown(fs.readFileSync(path.join(dir, n), 'utf8'), ctx)) found.push({ file: `docs/codemap/${n}`, ...f })
  }
  return found
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const found = lintCodemaps(root)
  for (const f of found) console.log(`${f.file}:${f.line}  ${f.kind === 'file' ? 'soubor neexistuje' : 'symbol v kódu není'}: ${f.value}`)
  console.log(found.length ? `CODEMAP: ${found.length} zastaralých položek` : 'CODEMAP: v pořádku')
  process.exit(found.length ? 1 : 0)
}
