// Strop velikosti modulu (~600 řádků PRODUKČNÍHO kódu): nepočítá `#[cfg(test)] mod …` v Rustu a čistě testovací soubory
// (`tests.rs`, `*_tests.rs`, `*.test.*`). Čisté funkce + `loadSizes` jako jediné I/O; test: scripts/modsize.test.mjs.
//   node scripts/modsize.mjs   → vypíše produkční moduly nad stropem (informačně, exit 0); `codemap-lint` je připojí také.
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const MAX_MODULE_LINES = 600
const SKIP_DIRS = new Set(['node_modules', 'target', 'target-agents', 'dist', '.git', '.claude'])
const SRC = /\.(rs|ts|tsx)$/
const TEST_FILE = /(^|\/)(tests\.rs|[^/]*_tests\.rs|testkit\.rs)$|\.test\.[a-z]+$|\.d\.ts$/

export const isTestFile = (f) => TEST_FILE.test(f.replace(/\\/g, '/'))

/** Počet produkčních řádků: Rust bez bloku od `#[cfg(test)]` (těsně před `mod …`) dál, ostatní celý soubor. */
export function prodLines(file, text) {
  const lines = text.split(/\r?\n/)
  if (lines.at(-1) === '') lines.pop()
  if (!file.endsWith('.rs')) return lines.length
  const i = lines.findIndex((l, k) => /^#\[cfg\(test\)\]\s*$/.test(l) && /^(pub(\([a-z]+\))?\s+)?mod\s+\w+/.test(lines[k + 1] ?? ''))
  return i < 0 ? lines.length : i
}

/** [{ file, prod, total }] seřazené sestupně; jen produkční soubory nad `max`. */
export function oversize(entries, max = MAX_MODULE_LINES) {
  return entries
    .filter((e) => !isTestFile(e.file))
    .map((e) => ({ file: e.file, prod: prodLines(e.file, e.text), total: e.text.split(/\r?\n/).length }))
    .filter((e) => e.prod > max)
    .sort((a, b) => b.prod - a.prod)
}

export function loadSizes(root, max = MAX_MODULE_LINES) {
  const entries = []
  const walk = (dir) => {
    for (const d of fs.readdirSync(dir, { withFileTypes: true })) {
      if (d.isDirectory()) { if (!SKIP_DIRS.has(d.name)) walk(path.join(dir, d.name)); continue }
      const rel = path.relative(root, path.join(dir, d.name)).replace(/\\/g, '/')
      if (SRC.test(d.name) && /^(src|src-tauri\/src)\//.test(rel)) entries.push({ file: rel, text: fs.readFileSync(path.join(root, rel), 'utf8') })
    }
  }
  walk(path.join(root, 'src'))
  walk(path.join(root, 'src-tauri', 'src'))
  return oversize(entries, max)
}

export const formatOversize = (o) => `strop ${MAX_MODULE_LINES} řádků: ${o.file} má ${o.prod} produkčních (celkem ${o.total})`

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const over = loadSizes(root)
  for (const o of over) console.log(formatOversize(o))
  console.log(over.length ? `MODSIZE: ${over.length} modulů nad stropem (informačně)` : 'MODSIZE: v pořádku')
}
