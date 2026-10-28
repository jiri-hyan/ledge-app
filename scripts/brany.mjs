// Brány Ledge jedním příkazem s KOMPAKTNÍM výstupem: jeden řádek na bránu, při selhání jen relevantní
// řádky (chyby, padlé testy) a cesta k plnému logu. Proč: výstup bran šel workerům do kontextu celý a pak se
// při každém dalším volání znovu četl z cache (token-audit vln 5–6, 18. 10. 2026).
// Těžké běhy jdou přes sdílenou frontu stroje (%USERPROFILE%\.devtools\fronta.mjs), pokud existuje.
//
// Použití: node scripts/brany.mjs                 → test, clippy, tsc, ui, build
//          node scripts/brany.mjs test ui         → jen vybrané (test | clippy | tsc | ui | build)
//          node scripts/brany.mjs --changed       → brány podle změněných souborů (výběr: scripts/brany-select.mjs)
//          tsc a ui běží souběžně s cargo bránami (test → clippy za sebou), výstup a exit kód beze změny.
// CARGO_TARGET_DIR se bere z prostředí (workeři: sdílený src-tauri\target-agents). Exit 0 = vše prošlo.
import { execFileSync, spawn } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { selectGates } from './brany-select.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const tauri = path.join(root, 'src-tauri')
// relativní CARGO_TARGET_DIR by se rozbil o cwd `src-tauri` (→ src-tauri/src-tauri/…) → absolutně vůči kořeni repa
if (process.env.CARGO_TARGET_DIR && !path.isAbsolute(process.env.CARGO_TARGET_DIR)) process.env.CARGO_TARGET_DIR = path.resolve(root, process.env.CARGO_TARGET_DIR)
const logDir = path.join(os.tmpdir(), 'ledge-brany', path.basename(root))
fs.mkdirSync(logDir, { recursive: true })
const queue = path.join(os.homedir(), '.devtools', 'fronta.mjs')
const viaQueue = (cmd) => (fs.existsSync(queue) ? `node "${queue}" --auto -- ${cmd}` : cmd)

const num = (re, text) => [...text.matchAll(re)].reduce((a, m) => a + Number(m[1]), 0)
const pick = (re, text, n = 25) => text.split(/\r?\n/).filter((l) => re.test(l)).slice(0, n)

const GATES = {
  test: {
    cwd: tauri, cmd: viaQueue('cargo test'),
    sum: (t) => `${num(/(\d+) passed/g, t)} ok, ${num(/(\d+) failed/g, t)} fail`,
    ok: (t, code) => code === 0 && num(/(\d+) failed/g, t) === 0,
    bad: (t) => pick(/FAILED|panicked|^error|^---- /, t)
  },
  clippy: {
    cwd: tauri, cmd: viaQueue('cargo clippy --all-targets'),
    sum: (t) => `${pick(/^warning: (?!.*generated \d+ warning)/, t, 1e6).length} varování, ${pick(/^error/, t, 1e6).length} chyb`,
    ok: (t, code) => code === 0 && pick(/^(warning|error)/, t, 1).length === 0,
    bad: (t) => {
      const ls = t.split(/\r?\n/)
      return ls.flatMap((l, i) => (/^(warning|error)/.test(l) && !/generated \d+ warning/.test(l) ? [l, ls[i + 1] ?? ''] : [])).slice(0, 30)
    }
  },
  tsc: {
    cwd: root, cmd: 'npx tsc --noEmit',
    sum: (t) => `${pick(/error TS/, t, 1e6).length} chyb`,
    ok: (t, code) => code === 0,
    bad: (t) => pick(/error TS/, t)
  },
  ui: {
    cwd: root, cmd: viaQueue('corepack pnpm test:ui'),
    sum: (t) => `${num(/ℹ pass (\d+)/g, t)} ok, ${num(/ℹ fail (\d+)/g, t)} fail`,
    ok: (t, code) => code === 0 && num(/ℹ fail (\d+)/g, t) === 0,
    bad: (t) => pick(/^not ok|✖|Error|expected|actual/i, t)
  },
  build: {
    cwd: root, cmd: viaQueue('corepack pnpm build'),
    sum: (t) => {
      const main = /index-[\w-]+\.js\s+([\d.,]+ kB)/.exec(t)
      return main ? `hlavní chunk ${main[1]}` : 'hotovo'
    },
    ok: (t, code) => code === 0,
    bad: (t) => pick(/error|Error|failed/, t)
  }
}

function run(name) {
  const g = GATES[name]
  const started = Date.now()
  return new Promise((resolve) => {
    const child = spawn(g.cmd, { cwd: g.cwd, shell: true, windowsHide: true, env: process.env })
    let out = ''
    child.stdout.on('data', (d) => (out += d))
    child.stderr.on('data', (d) => (out += d))
    child.on('close', (code) => {
      // eslint-disable-next-line no-control-regex
      const text = out.replace(/\x1b\[[0-9;]*m/g, '')
      const log = path.join(logDir, `${name}.log`)
      fs.writeFileSync(log, text)
      const ok = g.ok(text, code)
      const secs = Math.round((Date.now() - started) / 1000)
      const lines = [`${ok ? 'PASS' : 'FAIL'}  ${name.padEnd(6)} ${g.sum(text)} (${secs} s)`]
      if (!ok) {
        for (const l of g.bad(text)) lines.push(`      ${l}`)
        lines.push(`      plný log: ${log}`)
      }
      resolve({ ok, lines })
    })
  })
}

// --changed: brány podle `git diff --name-only main...HEAD` + necommitnutých a nových souborů (viz brany-select.mjs).
function changedFiles() {
  const git = (args) => execFileSync('git', ['-c', 'core.quotepath=off', ...args], { cwd: root, encoding: 'utf8' }).split(/\r?\n/).filter(Boolean)
  const base = ['main', 'origin/main'].find((b) => { try { git(['rev-parse', '--verify', '--quiet', b]); return true } catch { return false } })
  const files = new Set([...(base ? git(['diff', '--name-only', `${base}...HEAD`]) : []), ...git(['diff', '--name-only', 'HEAD']), ...git(['ls-files', '--others', '--exclude-standard'])])
  return [...files]
}

const args = process.argv.slice(2)
let names
if (args.includes('--changed')) {
  const { gates, why } = selectGates(changedFiles())
  console.log(`BRÁNY --changed: ${gates.length ? gates.join(' ') : '(žádná)'} — ${why}`)
  names = gates
} else {
  const wanted = args.filter((a) => a in GATES)
  names = wanted.length ? wanted : Object.keys(GATES)
}
// Souběh: tsc + ui běží vedle cargo řetězce (test → clippy za sebou — sdílený zámek target), build až po všem.
// Výstup (pořadí řádků, exit kód) je stejný jako při běhu za sebou: řádky se tisknou v pořadí `names` po dokončení.
const results = {}
const lane = (list) => list.reduce((p, n) => p.then(async () => { results[n] = await run(n) }), Promise.resolve())
const cargoLane = names.filter((n) => n === 'test' || n === 'clippy')
const freeLanes = names.filter((n) => n === 'tsc' || n === 'ui').map((n) => [n])
await Promise.all([lane(cargoLane), ...freeLanes.map(lane)])
await lane(names.filter((n) => n === 'build'))
let allOk = true
for (const n of names) {
  for (const l of results[n].lines) console.log(l)
  allOk = results[n].ok && allOk
}
console.log(allOk ? 'BRÁNY: vše prošlo' : 'BRÁNY: SELHALO — viz výše')
process.exit(allOk ? 0 : 1)
