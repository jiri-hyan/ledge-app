// Worktree stavitelů jedním příkazem (místo opsaného postupu v CLAUDE.md a předávkách).
//   node scripts/wt.mjs new <oblast> <kolo>   → větev k<kolo>/<oblast>, worktree ../ledge-wt-<oblast>, junction node_modules,
//                                               kopie dist (bez ní neprojde tauri::generate_context!), zapne git rerere
//   node scripts/wt.mjs rm <oblast>           → odpojí junction (obsah cíle NEMAŽE), git worktree remove (bez --force),
//                                               větev jen `git branch -d` (nesloučenou ponechá)
//   node scripts/wt.mjs prune [--days N] [--dir <target>] [--apply]
//                                             → stará inkrementální cache (target-agents/**/incremental/<crate>-*), výchozí je
//                                               DRY-RUN s velikostí; maže jen s --apply. Stáří = nejnovější mtime položky
//                                               a jejích přímých potomků; výchozí N = 3 dny (živý build tak zůstane).
// Junction se tvoří přes fs.symlinkSync(…, 'junction') — bez shellu, bez mklink, bez práv správce.
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const SAFE = /^[a-z0-9][a-z0-9-]*$/

export function planNames(oblast, kolo) {
  if (!SAFE.test(oblast ?? '')) throw new Error(`neplatná oblast "${oblast}" (malá písmena, číslice, pomlčka)`)
  if (!/^\d+[a-z]?$/.test(String(kolo ?? ''))) throw new Error(`neplatné kolo "${kolo}" (např. 29)`)
  return { branch: `k${kolo}/${oblast}`, dirName: `ledge-wt-${oblast}` }
}

/** Je cesta symlink/junction (ne skutečná složka)? Junction na Windows hlásí lstat.isSymbolicLink(). */
export function isLink(p) {
  try { return fs.lstatSync(p).isSymbolicLink() } catch { return false }
}

/** Odpojí odkaz BEZ zásahu do cíle (unlink/rmdir na junction maže jen odkaz); skutečnou složku nikdy nemaže. */
export function unlinkLink(p) {
  if (!isLink(p)) return false
  try { fs.unlinkSync(p) } catch { fs.rmdirSync(p) }
  return true
}

/** Odpojí všechny odkazy v kořeni worktree (node_modules, případně jiné). Vrací jejich názvy. */
export function unlinkTopLevelLinks(wt) {
  const out = []
  for (const e of fs.readdirSync(wt)) if (unlinkLink(path.join(wt, e))) out.push(e)
  return out
}

const gitIn = (cwd, args) => execFileSync('git', ['-c', 'core.quotepath=off', ...args], { cwd, encoding: 'utf8', windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }).trim()

export function mainRootOf(cwd) {
  const first = gitIn(cwd, ['worktree', 'list', '--porcelain']).split(/\r?\n/)[0] ?? ''
  if (!first.startsWith('worktree ')) throw new Error('nejsem v git repu')
  return path.resolve(first.slice(9))
}

export function newWorktree(mainRoot, oblast, kolo, base = 'main') {
  const { branch, dirName } = planNames(oblast, kolo)
  const wt = path.resolve(mainRoot, '..', dirName)
  if (fs.existsSync(wt)) throw new Error(`${wt} už existuje`)
  gitIn(mainRoot, ['config', 'rerere.enabled', 'true']) // idempotentní, sdílené všemi worktree
  gitIn(mainRoot, ['worktree', 'add', '-b', branch, wt, base])
  const notes = []
  const nm = path.join(mainRoot, 'node_modules')
  if (fs.existsSync(nm)) { fs.symlinkSync(nm, path.join(wt, 'node_modules'), 'junction'); notes.push('node_modules: junction') } else notes.push('POZOR: main nemá node_modules')
  const dist = path.join(mainRoot, 'dist')
  if (fs.existsSync(dist)) { fs.cpSync(dist, path.join(wt, 'dist'), { recursive: true }); notes.push('dist: kopie') } else notes.push('POZOR: main nemá dist (tauri::generate_context! selže)')
  return { branch, wt, notes }
}

export function removeWorktree(mainRoot, oblast) {
  const { dirName } = planNames(oblast, '0')
  const wt = path.resolve(mainRoot, '..', dirName)
  if (!fs.existsSync(wt)) throw new Error(`${wt} neexistuje`)
  const dirty = gitIn(wt, ['status', '--porcelain'])
  if (dirty) throw new Error(`worktree není čistý, odmítám:\n${dirty.split(/\r?\n/).slice(0, 10).join('\n')}`)
  const branch = gitIn(wt, ['rev-parse', '--abbrev-ref', 'HEAD'])
  const unlinked = unlinkTopLevelLinks(wt) // NEJDŘÍV odpojit, ať remove nikdy nemůže projít do cíle
  gitIn(mainRoot, ['worktree', 'remove', wt]) // nikdy --force
  let branchMsg
  try { gitIn(mainRoot, ['branch', '-d', branch]); branchMsg = `větev ${branch} smazána` } catch (e) { branchMsg = `větev ${branch} ponechána (nesloučená): ${String(e.stderr ?? e.message).trim().split(/\r?\n/)[0]}` }
  return { wt, unlinked, branchMsg }
}

// ---- prune ----
export function dirSize(p) {
  let sum = 0
  let st
  try { st = fs.lstatSync(p) } catch { return 0 }
  if (st.isSymbolicLink()) return 0
  if (!st.isDirectory()) return st.size
  for (const e of fs.readdirSync(p)) sum += dirSize(path.join(p, e))
  return sum
}

/** Najde složky `incremental` pod `root` (hloubka ≤ depth; `deps`/`build`/`.fingerprint` a odkazy přeskakuje). */
export function findIncrementalDirs(root, depth = 4) {
  const out = []
  const walk = (d, lvl) => {
    if (lvl > depth) return
    let ents
    try { ents = fs.readdirSync(d, { withFileTypes: true }) } catch { return }
    for (const e of ents) {
      if (!e.isDirectory() || e.isSymbolicLink()) continue
      const p = path.join(d, e.name)
      if (e.name === 'incremental') out.push(p)
      else if (e.name !== 'deps' && e.name !== 'build' && e.name !== '.fingerprint') walk(p, lvl + 1)
    }
  }
  walk(root, 0)
  return out
}

/** Čistá funkce: které položky ({path, mtimeMs}) jsou starší než `days` dnů vůči `now`. */
export function selectStale(items, now, days) {
  const limit = now - days * 86400000
  return items.filter((i) => i.mtimeMs <= limit)
}

export function newestMtime(p) {
  let m = fs.statSync(p).mtimeMs
  for (const e of fs.readdirSync(p)) { try { m = Math.max(m, fs.statSync(path.join(p, e)).mtimeMs) } catch { /* zmizelo */ } }
  return m
}

export function prune(targetDir, { days = 3, apply = false, now = Date.now() } = {}) {
  const items = []
  for (const inc of findIncrementalDirs(targetDir)) {
    for (const e of fs.readdirSync(inc, { withFileTypes: true })) {
      if (!e.isDirectory() || e.isSymbolicLink()) continue
      const p = path.join(inc, e.name)
      items.push({ path: p, mtimeMs: newestMtime(p) })
    }
  }
  const stale = selectStale(items, now, days).map((i) => ({ ...i, bytes: dirSize(i.path) }))
  const total = stale.reduce((a, i) => a + i.bytes, 0)
  if (apply) for (const i of stale) {
    // pojistka: smaže se jen to, co leží uvnitř `<target>/**/incremental/`
    if (path.basename(path.dirname(i.path)) !== 'incremental' || !path.resolve(i.path).startsWith(path.resolve(targetDir) + path.sep)) throw new Error(`odmítám mazat ${i.path}`)
    fs.rmSync(i.path, { recursive: true, force: true })
  }
  return { stale, total, kept: items.length - stale.length, applied: apply }
}

const mb = (b) => `${(b / 1048576).toFixed(0)} MB`

function main(argv) {
  const [cmd, ...rest] = argv
  const here = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const mainRoot = mainRootOf(here)
  if (cmd === 'new') {
    const r = newWorktree(mainRoot, rest[0], rest[1])
    console.log(`OK  ${r.branch} → ${r.wt} (${r.notes.join(', ')}; rerere zapnuto)`)
    console.log(`export CARGO_TARGET_DIR=${path.join(mainRoot, 'src-tauri', 'target-agents').replaceAll('\\', '/')}`)
  } else if (cmd === 'rm') {
    const r = removeWorktree(mainRoot, rest[0])
    console.log(`OK  ${r.wt} odstraněn (odpojeno: ${r.unlinked.join(', ') || '—'}); ${r.branchMsg}`)
  } else if (cmd === 'prune') {
    const val = (f) => { const i = rest.indexOf(f); return i >= 0 ? rest[i + 1] : undefined }
    const dir = val('--dir') ?? process.env.CARGO_TARGET_DIR ?? path.join(mainRoot, 'src-tauri', 'target-agents')
    const r = prune(dir, { days: Number(val('--days') ?? 3), apply: rest.includes('--apply') })
    for (const i of r.stale) console.log(`  ${mb(i.bytes).padStart(8)}  ${path.relative(dir, i.path)}`)
    console.log(`${r.applied ? 'SMAZÁNO' : 'DRY-RUN (nic nesmazáno; přidej --apply)'}: ${r.stale.length} složek, ${mb(r.total)}; ponecháno mladších: ${r.kept}`)
  } else {
    console.log('použití: wt.mjs new <oblast> <kolo> | rm <oblast> | prune [--days N] [--dir <target>] [--apply]')
    process.exit(cmd ? 2 : 0)
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try { main(process.argv.slice(2)) } catch (e) { console.error(`CHYBA: ${e.message}`); process.exit(1) }
}
