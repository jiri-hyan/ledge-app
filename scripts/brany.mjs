// Brány Ledge jedním příkazem s KOMPAKTNÍM výstupem: jeden řádek na bránu, při selhání jen relevantní
// řádky (chyby, padlé testy) a cesta k plnému logu. Proč: výstup bran šel workerům do kontextu celý a pak se
// při každém dalším volání znovu četl z cache (token-audit vln 5–6, 18. 10. 2026).
// Těžké běhy jdou přes sdílenou frontu stroje (%USERPROFILE%\.devtools\fronta.mjs), pokud existuje.
//
// Použití: node scripts/brany.mjs                 → test, clippy, tsc, ui, build
//          node scripts/brany.mjs test ui         → jen vybrané (test | clippy | tsc | ui | build)
//          node scripts/brany.mjs --changed       → brány podle změněných souborů (výběr: scripts/brany-select.mjs)
//          node scripts/brany.mjs --changed --since <ref>  → totéž, ale diff proti <ref> (např. --since HEAD~1 po non-ff mergi)
//          --changed s 0 soubory = VAROVÁNÍ + exit 2 (nic se neověřilo); záměr potvrdí --allow-empty
//          node scripts/brany.mjs --force         → plný běh i když razítko `.brany-ok` říká „strom se nezměnil od posledního PASS“
//          node scripts/brany.mjs --bez-opakovani → padlý test se nezkouší podruhé (jinak: jen padlé testy 1× znovu, výsledek FLAKY)
//          cargo test běží pod zámkem `<CARGO_TARGET_DIR>/.brany-cargo.lock` (dva agenti nelinkují stejnou test binárku → LNK1104);
//          LNK1104 přesto → 1× opakování po pauze.
//          test, clippy, tsc a ui běží souběžně (clippy má vlastní target dir `<CARGO_TARGET_DIR>/clippy`), výstup a exit kód beze změny.
// CARGO_TARGET_DIR se bere z prostředí (workeři: sdílený src-tauri\target-agents). Exit 0 = vše prošlo.
import { execFileSync, spawn } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { selectGates } from './brany-select.mjs'
import { parseFailedTests, canRetry, retryArgs, flakyLine, stampText, stampMatches, isFullRun, buildCommand, parseSince, emptyChangedWarning, isLinkLockError, lockIsStale } from './brany-lib.mjs'

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
  denylist: {
    // zakázaná slova v pracovním stromu a v předmětech commitů od posledního vydání (scripts/denylist-check.mjs); levná, bez fronty;
    // výstup nese jen `soubor:řádek ~ vzor #index`, nikdy zakázaný text. Bez scripts/private/denylist.txt (veřejný klon) projde.
    cwd: root, cmd: 'node scripts/denylist-check.mjs --tree --since auto',
    sum: (t) => { const m = /ZÁSAHY: (\d+)/.exec(t); return m ? `${m[1]} zásahů` : /přeskočeno/.test(t) ? 'přeskočeno' : '0 zásahů' },
    ok: (t, code) => code === 0,
    bad: (t) => [...pick(/^(ZAKÁZÁNO|denylist:)/, t, 12), ...pick(/^ZÁSAHY/, t, 1)]
  },
  test: {
    cwd: tauri, cmd: viaQueue('cargo test --no-fail-fast'),
    sum: (t) => `${num(/(\d+) passed/g, t)} ok, ${num(/(\d+) failed/g, t)} fail`,
    ok: (t, code) => code === 0 && num(/(\d+) failed/g, t) === 0,
    bad: (t) => pick(/FAILED|panicked|^error|^---- /, t)
  },
  clippy: {
    // vlastní target dir (podsložka sdíleného) → clippy neblokuje zámek `cargo test` a běží s ním souběžně; 1. běh je studený
    cwd: tauri, cmd: viaQueue('cargo clippy --all-targets'), env: { ...process.env, CARGO_TARGET_DIR: path.join(process.env.CARGO_TARGET_DIR || path.join(tauri, 'target'), 'clippy') },
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

// Zámek kolem celého `cargo test` (překlad + linkování + běh) pro stejný CARGO_TARGET_DIR: cargo zamyká jen překlad,
// takže druhý agent po jeho skončení znovu linkuje test binárku, kterou první ještě spouští → LNK1104.
const targetDir = process.env.CARGO_TARGET_DIR || path.join(tauri, 'target')
const lockFile = path.join(targetDir, '.brany-cargo.lock')
const alive = (pid) => { try { process.kill(pid, 0); return true } catch (e) { return e.code === 'EPERM' } }
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
async function withCargoLock(fn) {
  fs.mkdirSync(targetDir, { recursive: true })
  let told = false
  for (;;) {
    try {
      fs.writeFileSync(lockFile, `${process.pid}
${new Date().toISOString()}
${root}`, { flag: 'wx' })
      break
    } catch (e) {
      if (e.code !== 'EEXIST') return fn() // zámek je jen pojistka, nikdy neshodí bránu
      let text = ''
      try { text = fs.readFileSync(lockFile, 'utf8') } catch { continue }
      const pid = Number(text.split(/\r?\n/)[0])
      if (lockIsStale(text, alive(pid), Date.now())) { try { fs.rmSync(lockFile, { force: true }) } catch {} ; continue }
      if (!told) { console.error(`BRÁNY: čekám na jiný běh cargo test nad ${targetDir} (pid ${pid}) — zámek ${lockFile}`); told = true }
      await sleep(3000)
    }
  }
  try { return await fn() } finally { try { fs.rmSync(lockFile, { force: true }) } catch {} }
}

function exec(cmd, cwd, env = process.env) {
  return new Promise((resolve) => {
    const child = spawn(cmd, { cwd, shell: true, windowsHide: true, env })
    let out = ''
    child.stdout.on('data', (d) => (out += d))
    child.stderr.on('data', (d) => (out += d))
    child.on('close', (code) => resolve({ // eslint-disable-next-line no-control-regex
      text: out.replace(/\x1b\[[0-9;]*m/g, ''), code }))
  })
}

async function run(name) {
  const g = GATES[name]
  const started = Date.now()
  const cargoRun = (cmd) => (name === 'test' ? withCargoLock(async () => {
    const r = await exec(cmd, g.cwd)
    if (isLinkLockError(r.text) && r.code !== 0) { await sleep(15000); return exec(cmd, g.cwd) } // LNK1104: 1× znovu
    return r
  }) : exec(cmd, g.cwd, g.env))
  const first = await cargoRun(g.cmd)
  const text = first.text
  const log = path.join(logDir, `${name}.log`)
  fs.writeFileSync(log, text)
  let ok = g.ok(text, first.code)
  let flaky = null // jména testů, které prošly až na 2. pokus
  let retryLine = null
  if (!ok && name === 'test' && retryOn) {
    const failed = parseFailedTests(text)
    const ra = canRetry(text, failed) ? retryArgs(failed) : null
    if (ra) {
      const t2 = Date.now()
      const second = await cargoRun(viaQueue(`cargo test --no-fail-fast -- ${ra.join(' ')}`))
      fs.writeFileSync(path.join(logDir, 'test-retry.log'), second.text)
      // prošlo = brána OK a filtr opravdu spustil všechny padlé (jinak by prázdný běh vypadal jako úspěch)
      const ran = num(/(\d+) passed/g, second.text)
      const passed = g.ok(second.text, second.code) && ran >= failed.length
      retryLine = flakyLine(failed, passed, Math.round((Date.now() - t2) / 1000))
      if (passed) { ok = true; flaky = failed }
    }
  }
  const secs = Math.round((Date.now() - started) / 1000)
  const lines = [`${ok ? (flaky ? 'FLAKY' : 'PASS') : 'FAIL'}  ${name.padEnd(6)} ${g.sum(text)} (${secs} s)`]
  if (retryLine) lines.push(`      ${retryLine}`)
  if (!ok) {
    for (const l of g.bad(text)) lines.push(`      ${l}`)
    lines.push(`      plný log: ${log}`)
  } else if (flaky) lines.push(`      logy: ${log}, ${path.join(logDir, 'test-retry.log')}`)
  return { ok, flaky, lines }
}

// Hash celého pracovního stromu (sledované i nové neignorované soubory) přes dočasný index — skutečný index se nemění.
function treeHash() {
  const git = (args, env) => execFileSync('git', args, { cwd: root, encoding: 'utf8', windowsHide: true, env: { ...process.env, ...env }, stdio: ['ignore', 'pipe', 'ignore'] }).trim()
  const tmpIdx = path.join(os.tmpdir(), `ledge-brany-idx-${process.pid}`)
  try {
    fs.copyFileSync(path.resolve(root, git(['rev-parse', '--git-path', 'index'])), tmpIdx)
    git(['add', '-A'], { GIT_INDEX_FILE: tmpIdx })
    return git(['write-tree'], { GIT_INDEX_FILE: tmpIdx })
  } catch { return null } finally { fs.rmSync(tmpIdx, { force: true }) }
}

// --changed: brány podle `git diff --name-only main...HEAD` (s --since <ref>: `git diff --name-only <ref>`) + necommitnutých a nových souborů (viz brany-select.mjs).
function changedFiles(since) {
  const git = (args) => execFileSync('git', ['-c', 'core.quotepath=off', ...args], { cwd: root, encoding: 'utf8' }).split(/\r?\n/).filter(Boolean)
  if (since) return [...new Set([...git(['diff', '--name-only', since]), ...git(['ls-files', '--others', '--exclude-standard'])])]
  const base = ['main', 'origin/main'].find((b) => { try { git(['rev-parse', '--verify', '--quiet', b]); return true } catch { return false } })
  const files = new Set([...(base ? git(['diff', '--name-only', `${base}...HEAD`]) : []), ...git(['diff', '--name-only', 'HEAD']), ...git(['ls-files', '--others', '--exclude-standard'])])
  return [...files]
}

const args = process.argv.slice(2)
const retryOn = !args.includes('--bez-opakovani')
const stampFile = path.join(root, '.brany-ok')
let names
if (args.includes('--changed')) {
  const { since, allowEmpty } = parseSince(args)
  if (since === '') { console.error('BRÁNY: --since vyžaduje ref (např. --since HEAD~1)'); process.exit(2) }
  let files
  try { files = changedFiles(since) } catch (e) { console.error(`BRÁNY: git diff proti ${since} selhal — ${String(e.message).split(/\r?\n/)[0]}`); process.exit(2) }
  const warn = emptyChangedWarning(files.length, allowEmpty)
  if (warn) { console.log(warn); process.exit(2) }
  const { gates, why } = selectGates(files)
  console.log(`BRÁNY --changed: ${gates.length ? gates.join(' ') : '(žádná)'} — ${why}`)
  names = gates
} else {
  const wanted = args.filter((a) => a in GATES)
  names = wanted.length ? wanted : Object.keys(GATES)
}
GATES.build.cmd = buildCommand(names, viaQueue) // tsc už v tomtéž běhu proběhl → build je jen `vite build`
const full = isFullRun(names, Object.keys(GATES))
const hash = full ? treeHash() : null
if (full && hash && !args.includes('--force') && stampMatches(fs.existsSync(stampFile) ? fs.readFileSync(stampFile, 'utf8') : '', hash)) {
  console.log(`BRÁNY: strom ${hash.slice(0, 10)} beze změny od posledního plného PASS (${fs.readFileSync(stampFile, 'utf8').split(/\r?\n/)[1]}) — přeskočeno; --force vynutí`)
  process.exit(0)
}
// Souběh: test, clippy (vlastní target dir), tsc a ui běží každé v své dráze, build až po všem.
// Výstup (pořadí řádků, exit kód) je stejný jako při běhu za sebou: řádky se tisknou v pořadí `names` po dokončení.
const results = {}
const lane = (list) => list.reduce((p, n) => p.then(async () => { results[n] = await run(n) }), Promise.resolve())
const freeLanes = names.filter((n) => n !== 'build').map((n) => [n])
await Promise.all(freeLanes.map(lane))
await lane(names.filter((n) => n === 'build'))
let allOk = true
const flakies = []
for (const n of names) {
  for (const l of results[n].lines) console.log(l)
  allOk = results[n].ok && allOk
  if (results[n].flaky) flakies.push(...results[n].flaky)
}
if (allOk && full && hash && !flakies.length) fs.writeFileSync(stampFile, stampText(hash))
else if (fs.existsSync(stampFile)) fs.rmSync(stampFile, { force: true }) // neúspěch/flaky nesmí nechat starou „čistou“ pečeť
console.log(!allOk ? 'BRÁNY: SELHALO — viz výše' : flakies.length ? `BRÁNY: prošlo S OPAKOVÁNÍM (flaky: ${flakies.join(', ')}) — nečistý průchod, razítko nezapsáno` : 'BRÁNY: vše prošlo')
process.exit(allOk ? 0 : 1)
