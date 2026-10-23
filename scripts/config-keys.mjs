// Guard dokumentace configu: každý klíč `config.json` (odvozený ze struktur v Rustu, tedy z téhož, z čeho
// schemars generuje `config.schema.json`) musí být zmíněn v docs/agents/*.md. Jinak agent, který edituje
// config ručně, o klíči neví (dřív takhle chyběly `drawer.keepWarmS`, `claude.enforceCommit`…).
// Čisté funkce (bez I/O) + `readSources`/`checkConfigDocs` jako jediné I/O; test: scripts/config-keys.test.mjs.
//   node scripts/config-keys.mjs      → vypíše nezdokumentované klíče (exit 1, je-li nějaký)
//
// Pravidlo: koncové jméno klíče (`keepWarmS` z `drawer.keepWarmS`) musí být v textu docs jako samostatné slovo.
// Je to záměrně prosté (doc píše klíče i v prózce, v příkladech JSON i jako `a.b`), ale chytí klíč,
// o kterém není nikde ani zmínka. Výjimky: ALLOW (klíč → důvod).
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// Výjimky: klíč → důvod (zbytečná výjimka shodí test). Teď žádné — i `$schema` je popsán v config-drawer.md.
export const ALLOW = {}

const camel = (s) => s.replace(/_(\w)/g, (_, c) => c.toUpperCase())

/** Rust zdroj → struktury s poli: [{ name, rename, fields: [{ name, type, attrs }] }] (jen `struct X { … }`). */
export function parseStructs(src) {
  const out = []
  let attrs = []
  let cur = null
  for (const line of src.split(/\r?\n/)) {
    const t = line.trim()
    if (cur) {
      if (t === '}') { cur = null; attrs = []; continue }
      if (t.startsWith('#[')) { attrs.push(t); continue }
      if (t.startsWith('//')) continue
      const m = /^(?:pub(?:\([^)]*\))?\s+)?(\w+)\s*:\s*(.+?),?$/.exec(t)
      if (m) cur.fields.push({ name: m[1], type: m[2], attrs })
      attrs = []
      continue
    }
    if (t.startsWith('#[')) { attrs.push(t); continue }
    const m = /^(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)\s*\{$/.exec(t)
    if (m) {
      cur = { name: m[1], rename: /rename_all\s*=\s*"(\w+)"/.exec(attrs.join(' '))?.[1], fields: [] }
      out.push(cur)
    }
    if (t && !t.startsWith('//')) attrs = []
  }
  return out
}

/** Klíč pole v JSON: `#[serde(rename = "x")]`, jinak podle `rename_all` struktury. null = pole se v JSON nevyskytuje. */
export function jsonKey(struct, field) {
  const a = field.attrs.join(' ')
  if (/serde\([^)]*\bskip\b(?!_)/.test(a) || /skip_deserializing/.test(a)) return null
  const r = /rename\s*=\s*"([^"]+)"/.exec(a)
  if (r) return r[1]
  return struct.rename === 'camelCase' ? camel(field.name) : field.name
}

/** Všechny cesty klíčů od kořene (`drawer.barWindows.pushMs`); pole struktur (`projects[]`) se rozvinou stejně. */
export function keyPaths(structs, root = 'Config') {
  const byName = new Map(structs.map((s) => [s.name, s]))
  const out = []
  const walk = (name, prefix, depth, seen) => {
    const s = byName.get(name)
    if (!s || seen.has(name) || depth > 8) return
    for (const f of s.fields) {
      const key = jsonKey(s, f)
      if (key === null) continue
      const p = prefix ? `${prefix}.${key}` : key
      out.push(p)
      const types = new Set((f.type.match(/[A-Za-z_][\w:]*/g) ?? []).map((x) => x.split('::').pop()).filter((x) => byName.has(x)))
      for (const tn of types) walk(tn, p, depth + 1, new Set([...seen, name]))
    }
  }
  walk(root, '', 0, new Set())
  return out
}

const wordIn = (text, w) => new RegExp('(?<![A-Za-z0-9_])' + w.replace(/[.*+?^${}()|[\]\\]/g, (x) => String.fromCharCode(92) + x) + '(?![A-Za-z0-9_])').test(text)

/** Cesty, jejichž koncové jméno není v docs; výjimky ven. → [path] */
export function undocumented(paths, docs, allow = ALLOW) {
  return paths.filter((p) => !(p in allow) && !wordIn(docs, p.split('.').pop()))
}

// ---------- I/O ----------

function rsFiles(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name)
    if (e.isDirectory()) rsFiles(p, out)
    else if (e.name.endsWith('.rs')) out.push(p)
  }
  return out
}

/** Načte Rust struktury z src-tauri/src a text všech docs/agents/*.md → { structs, docs }. */
export function readSources(root) {
  const structs = rsFiles(path.join(root, 'src-tauri', 'src')).flatMap((f) => parseStructs(fs.readFileSync(f, 'utf8')))
  const dir = path.join(root, 'docs', 'agents')
  const docs = fs.readdirSync(dir).filter((n) => n.endsWith('.md')).map((n) => fs.readFileSync(path.join(dir, n), 'utf8')).join('\n')
  return { structs, docs }
}

export function checkConfigDocs(root, allow = ALLOW) {
  const { structs, docs } = readSources(root)
  const paths = keyPaths(structs)
  return { total: paths.length, missing: undocumented(paths, docs, allow) }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const { total, missing } = checkConfigDocs(root)
  for (const p of missing) console.log(`config klíč bez zmínky v docs/agents: ${p}`)
  console.log(missing.length ? `CONFIG-DOCS: ${missing.length} z ${total} klíčů nezdokumentováno` : `CONFIG-DOCS: v pořádku (${total} klíčů)`)
  process.exit(missing.length ? 1 : 0)
}
