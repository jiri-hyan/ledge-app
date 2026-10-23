// Kde je symbol? Jedním příkazem: definice v kódu + které mapy a návody ho zmiňují. Proč: workeři hledali
// symboly sérií grepů přes celý strom a četli kvůli tomu celé soubory; tohle projde strom jednou a vrátí
// jen řádky (soubor:řádek), takže se pak čte přesně ten rozsah. Žádné závislosti, jen node:fs.
//
// Použití: node scripts/kde.mjs <symbol|slovo>
//   1) definice v src/** a src-tauri/src/** (fn/struct/enum/const/static/mod/type/trait, function,
//      interface/class/let/var, `X =`, pole `pub x:` / `x?:`; camelCase klíč configu najde i snake_case
//      pole v Rustu: `muteDirty` → `mute_dirty`), max 15; bez přesné shody hledá jména, která slovo obsahují
//   2) docs/codemap/*.md a docs/agents/*.md, kde se slovo vyskytuje (soubor:řádek + první výskyt)
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const MAX = 15
const CODE_DIRS = ['src', 'src-tauri/src']
const CODE_EXT = /\.(rs|ts|tsx|mjs|js|css)$/
const SKIP = new Set(['node_modules', 'target', 'target-agents', 'dist', '.git'])

const DEF = /\b(?:fn|struct|enum|const|static|mod|type|trait|union|function\*?|interface|class|let|var)\s+(\w+)/g
const ASSIGN = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:export\s+)?(?:default\s+)?(\w+)\s*(?::[^=]*)?=[^=>]/

function walk(dir, out) {
  let ents
  try { ents = fs.readdirSync(dir, { withFileTypes: true }) } catch { return out }
  for (const e of ents) {
    if (SKIP.has(e.name)) continue
    const p = path.join(dir, e.name)
    if (e.isDirectory()) walk(p, out)
    else if (CODE_EXT.test(e.name)) out.push(p)
  }
  return out
}

const rel = (p) => path.relative(root, p).split(path.sep).join('/')
const cut = (s, n = 120) => { s = s.trim().replace(/\s+/g, ' '); return s.length > n ? s.slice(0, n - 1) + '…' : s }

// Čistá funkce: řádky jednoho souboru → nálezy { tier, line, text }. tier 1 = definice, 2 = `X =`, 3 = částečná shoda.
const snake = (w) => w.replace(/[A-Z]/g, (c) => '_' + c.toLowerCase())
const escRe = (w) => w.replace(/\W/g, '') // hledá se identifikátor; cokoli jiného by se jen zahodilo

export function findDefs(lines, word) {
  const low = word.toLowerCase()
  const names = [...new Set([word, snake(word)])]
  const fieldRe = new RegExp(String.raw`^\s*(?:pub(?:\([^)]*\))?\s+)?(?:` + names.map(escRe).join('|') + String.raw`)\??\s*:\s*(?:bool\b|boolean\b|string\b|number\b|str\b|[ui]\d+\b|f\d+\b|[ui]size\b|[A-Z]|&|\[|\()`)
  const hits = []
  lines.forEach((text, i) => {
    if (!text.toLowerCase().includes(low) && !(names.length > 1 && text.includes(names[1]))) return
    let tier = 0
    for (const m of text.matchAll(DEF)) {
      if (m[1] === word) { tier = 1; break }
      if (m[1].toLowerCase().includes(low)) tier = tier || 3
    }
    if (tier !== 1) {
      const a = ASSIGN.exec(text)
      if (a && a[1] === word) tier = 2
      else if (fieldRe.test(text)) tier = 2
    }
    if (tier) hits.push({ tier, line: i + 1, text })
  })
  return hits
}

export function firstMention(lines, word) {
  const isW = (c) => c !== undefined && /[A-Za-z0-9_]/.test(c)
  const has = (l) => {
    for (let at = l.indexOf(word); at >= 0; at = l.indexOf(word, at + 1)) if (!isW(l[at - 1]) && !isW(l[at + word.length])) return true
    return false
  }
  const i = lines.findIndex(has)
  return i < 0 ? null : { line: i + 1, text: lines[i] }
}

function main() {
  const word = process.argv[2]
  if (!word) {
    console.error('použití: node scripts/kde.mjs <symbol|slovo>')
    process.exit(2)
  }
  const all = []
  for (const d of CODE_DIRS) for (const f of walk(path.join(root, d), [])) {
    const lines = fs.readFileSync(f, 'utf8').split(/\r?\n/)
    for (const h of findDefs(lines, word)) all.push({ file: rel(f), ...h })
  }
  let tiers = all.filter((h) => h.tier < 3)
  if (!tiers.length) tiers = all
  tiers.sort((a, b) => a.tier - b.tier || a.file.localeCompare(b.file) || a.line - b.line)
  console.log(`# definice „${word}“ (${tiers.length}${tiers.length > MAX ? `, ukázáno ${MAX}` : ''})`)
  if (!tiers.length) console.log('  (žádná definice v src/**, src-tauri/src/**)')
  for (const h of tiers.slice(0, MAX)) console.log(`${h.file}:${h.line}  ${cut(h.text)}`)

  const docs = []
  for (const d of ['docs/codemap', 'docs/agents']) {
    let names = []
    try { names = fs.readdirSync(path.join(root, d)).filter((n) => n.endsWith('.md')).sort() } catch { /* chybí složka */ }
    for (const n of names) {
      const m = firstMention(fs.readFileSync(path.join(root, d, n), 'utf8').split(/\r?\n/), word)
      if (m) docs.push(`${d}/${n}:${m.line}  ${cut(m.text)}`)
    }
  }
  console.log(`# zmínky v docs (${docs.length})`)
  for (const l of docs) console.log(l)
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main()
