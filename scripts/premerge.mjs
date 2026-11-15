// Předběžná kontrola před mergem větví kola: kolize a překryv horkých souborů BEZ zásahu do stromu (`git merge-tree`).
//   node scripts/premerge.mjs [--tsc] k34/a k34/b ...     (základ = main)
// --tsc: nad sekvenčním mergem pustí `tsc --noEmit` v dočasném worktree (`.claude/worktrees/premerge-tmp-<pid>`, junction
//   node_modules, vždy uklizen) a vypíše chyby; zdvojený identifikátor/klíč (TS2300, TS1117…) označí DUP.
// Vypíše: (1) pár větví, které by se při mergi střetly (soubory z `merge-tree --name-only`), (2) horké soubory
// změněné ve 2+ větvích (nemusí kolidovat, ale bývají zdrojem ručních oprav), (3) velikost jader codemap po sekvenčním mergi.
// Čisté funkce (hotFiles, overlap, coreSizes) jsou testované v premerge.test.mjs; zbytek je jen git.
import { execFileSync, spawnSync } from 'node:child_process'
import fs from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { mainRootOf, worktreeDir, unlinkTopLevelLinks } from './wt.mjs'

export const MAX_CORE_BYTES = 8192
// Soubory, na kterých se workeři opakovaně potkávají (jádra map, sdílené testy a slovníky).
export const HOT = [/^docs\/codemap\/(ui|core|README)\.md$/, /^src\/settings\/placement\.test\.mjs$/, /^src\/i18n\/en-[\w-]+\.ts$/]
export const isHot = (f) => HOT.some((re) => re.test(f))
export const hotFiles = (files) => files.filter(isHot)

/** { větev: [soubory] } → [{ file, branches }] pro soubory změněné ve 2+ větvích (jen horké, nebo všechny s all=true). */
export function overlap(byBranch, all = false) {
  const seen = new Map()
  for (const [b, files] of Object.entries(byBranch)) for (const f of new Set(files)) seen.set(f, [...(seen.get(f) ?? []), b])
  return [...seen].filter(([f, bs]) => bs.length > 1 && (all || isHot(f))).map(([file, branches]) => ({ file, branches })).sort((a, b) => a.file.localeCompare(b.file))
}

/** { cesta: bajty } → jádra codemap nad stropem (jen `docs/codemap/*.md` mimo `-detail.md`). */
export function coreSizes(sizes, max = MAX_CORE_BYTES) {
  return Object.entries(sizes)
    .filter(([f]) => /^docs\/codemap\/[^/]+\.md$/.test(f) && !/-detail\.md$/.test(f) && sizes[f] > max)
    .map(([file, bytes]) => ({ file, bytes, over: bytes - max }))
}

// Kódy tsc „něco je tam dvakrát“: identifikátor, klíč objektu, blokový název, funkce, třída/rozhraní, export default.
export const DUP_CODES = ['TS2300', 'TS1117', 'TS2451', 'TS2393', 'TS2394', 'TS2395', 'TS2528', 'TS2432']

/** Výstup `tsc --noEmit` → [{ file, line, col, code, msg, dup }] (jen řádky tvaru `cesta(ř,s): error TSxxxx: text`). */
export function parseTsc(out) {
  const re = /^(.+?)\((\d+),(\d+)\): error (TS\d+): (.*)$/
  return String(out).split(/\r?\n/).map((l) => re.exec(l.trim())).filter(Boolean)
    .map((m) => ({ file: m[1].replace(/\\/g, '/'), line: +m[2], col: +m[3], code: m[4], msg: m[5], dup: DUP_CODES.includes(m[4]) }))
}

/** `tsc --noEmit` nad commitem `tip` v dočasném worktree uvnitř repa; vždy uklidí (junction se odpojí dřív než remove). */
function tscAt(tip) {
  const root = mainRootOf(process.cwd())
  const wt = worktreeDir(root, `premerge-tmp-${process.pid}`)
  git(['-C', root, 'worktree', 'add', '--detach', wt, tip])
  try {
    const nm = path.join(root, 'node_modules')
    if (fs.existsSync(nm)) fs.symlinkSync(nm, path.join(wt, 'node_modules'), 'junction')
    const r = spawnSync(process.execPath, [path.join(root, 'node_modules', 'typescript', 'bin', 'tsc'), '--noEmit', '--pretty', 'false'], { cwd: wt, encoding: 'utf8', windowsHide: true, maxBuffer: 64 << 20 })
    return parseTsc(`${r.stdout ?? ''}
${r.stderr ?? ''}`)
  } finally {
    try { unlinkTopLevelLinks(wt) } catch { /* worktree už není */ }
    try { git(['-C', root, 'worktree', 'remove', '--force', wt]) } catch { try { fs.rmSync(wt, { recursive: true, force: true }) } catch { /* nic */ } }
  }
}

const git = (args, okCodes = [0]) => {
  try { return { out: execFileSync('git', ['-c', 'core.quotepath=off', ...args], { encoding: 'utf8', windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }), code: 0 } }
  catch (e) { if (okCodes.includes(e.status)) return { out: String(e.stdout ?? ''), code: e.status }; throw e }
}
const lines = (s) => s.split(/\r?\n/).filter(Boolean)

function main(argv) {
  const withTsc = argv.includes('--tsc')
  const branches = argv.filter((a) => a !== '--tsc')
  if (branches.length < 1) { console.error('použití: node scripts/premerge.mjs <větev> [větev…]'); process.exit(2) }
  const byBranch = {}
  for (const b of branches) byBranch[b] = lines(git(['diff', '--name-only', `main...${b}`]).out)
  let bad = 0
  for (let i = 0; i < branches.length; i++) {
    const r = git(['merge-tree', '--write-tree', '--name-only', 'main', branches[i]], [1])
    if (r.code === 1) { bad++; console.log(`KOLIZE main × ${branches[i]}: ${lines(r.out).slice(1).filter((l) => !/^(Auto-merging|CONFLICT)/.test(l)).join(', ')}`) }
    for (let j = i + 1; j < branches.length; j++) {
      const p = git(['merge-tree', '--write-tree', '--name-only', branches[i], branches[j]], [1])
      if (p.code === 1) { bad++; console.log(`KOLIZE ${branches[i]} × ${branches[j]}: ${lines(p.out).slice(1).filter((l) => !/^(Auto-merging|CONFLICT)/.test(l)).join(', ')}`) }
    }
  }
  for (const o of overlap(byBranch)) console.log(`horký soubor ve více větvích: ${o.file} ← ${o.branches.join(', ')}`)
  // sekvenční merge přes dočasné commity (visící objekty, strom ani větve se nemění)
  let tip = 'main'
  for (const b of branches) {
    const r = git(['merge-tree', '--write-tree', tip, b], [1])
    if (r.code === 1) { console.log(`sekvenční merge zastaven na ${b} (kolize) — velikost jader neznámá`); tip = null; break }
    tip = git(['commit-tree', lines(r.out)[0], '-p', tip, '-p', b, '-m', 'premerge']).out.trim()
  }
  if (tip) {
    const sizes = {}
    for (const f of lines(git(['ls-tree', '--name-only', tip, 'docs/codemap/']).out)) sizes[f] = Number(git(['cat-file', '-s', `${tip}:${f}`]).out)
    for (const c of coreSizes(sizes)) { bad++; console.log(`jádro po mergi nad stropem: ${c.file} ${c.bytes} B (+${c.over} B nad ${MAX_CORE_BYTES})`) }
    if (withTsc) {
      const t0 = Date.now()
      const errs = tscAt(tip)
      for (const e of errs.slice(0, 40)) console.log(`${e.dup ? 'DUP ' : ''}tsc ${e.file}(${e.line},${e.col}) ${e.code}: ${e.msg}`)
      if (errs.length > 40) console.log(`… a dalších ${errs.length - 40} chyb tsc`)
      bad += errs.length
      console.log(`tsc po mergi: ${errs.length ? `${errs.length} chyb (${errs.filter((e) => e.dup).length}× zdvojený název)` : 'čisté'} (${((Date.now() - t0) / 1000).toFixed(1)} s)`)
    }
  } else if (withTsc) console.log('tsc přeskočeno: sekvenční merge má kolize')
  console.log(bad ? `PREMERGE: ${bad} problémů` : 'PREMERGE: bez kolizí a bez překročených jader')
  process.exit(bad ? 1 : 0)
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main(process.argv.slice(2))
