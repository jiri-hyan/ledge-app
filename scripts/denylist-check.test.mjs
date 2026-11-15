// Kontrola zakázaných slov — čisté funkce tabulkově + CLI nad dočasným repem s VLASTNÍM denylistem (nikdy skutečný soubor).
// Slovo v testu je vymyšlené („zakazanoslovo“); skutečný denylist ani jeho obsah se zde nečte.
//   node --experimental-strip-types --test scripts/denylist-check.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync, spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { parsePatterns, parseSkip, scanBlock, scanFile, scanSubjects, formatHit, summarize } from './denylist-check.mjs'

const script = path.join(path.dirname(fileURLToPath(import.meta.url)), 'denylist-check.mjs')
const W = 'zakazanoslovo'
const pats = parsePatterns(`# komentář\n\n${W}\nsecret-\\d+\n`)

test('parsePatterns: komentáře a prázdné řádky se vynechají, index = pořadí platných vzorů', () => {
  assert.deepEqual(pats.map((p) => p.index), [0, 1])
  assert.ok(pats[0].re.test(W.toUpperCase()), 'bez ohledu na velikost písmen')
  assert.throws(() => parsePatterns('(\n'), /vzor #0/)
  assert.deepEqual(parsePatterns(''), [])
})

test('parseSkip: SHA před #, komentáře pryč', () => {
  const s = parseSkip(`aaa # důvod\n# jen komentář\n\nbbb\n`)
  assert.deepEqual([...s], ['aaa', 'bbb'])
})

test('scanFile: obsah (první řádek každého vzoru), jméno, binární přípona bez obsahu', () => {
  for (const [rel, content, want] of [
    ['a.ts', 'ok\nok', []],
    ['a.ts', `ok\nmá ${W.toUpperCase()} tady\n${W}`, [['a.ts:2', 0]]],
    ['a.ts', `${W} a secret-12`, [['a.ts:1', 0], ['a.ts:1', 1]]],
    [`${W}.md`, 'čisté', [['***.md (jméno souboru)', 0]]],
    ['img.png', W, []],
    [`${W}.png`, W, [['***.png (jméno souboru)', 0]]],
    ['b.rs', null, []],
  ]) {
    assert.deepEqual(scanFile(rel, content, pats).map((h) => [h.where, h.idx]), want, `${rel} ${content}`)
  }
})

test('výstup nikdy neobsahuje vzor ani nalezený text', () => {
  const hits = [...scanFile(`${W}/x.ts`, `řádek ${W}`, pats), ...scanSubjects([{ sha: 'a'.repeat(40), subject: `feat: ${W}` }], pats)]
  assert.ok(hits.length >= 3)
  for (const h of hits) {
    const line = formatHit(h)
    assert.ok(!line.toLowerCase().includes(W), line)
    assert.match(line, /^ZAKÁZÁNO: .+ ~ vzor #\d+$/)
  }
})

test('scanSubjects: vynechá SHA ze skip množiny', () => {
  const c = [{ sha: 'a'.repeat(40), subject: `fix: ${W}` }, { sha: 'b'.repeat(40), subject: `fix: ${W}` }, { sha: 'c'.repeat(40), subject: 'fix: čisté' }]
  assert.deepEqual(scanSubjects(c, pats, new Set(['a'.repeat(40)])).map((h) => h.where), ['commit bbbbbbbb (předmět)'])
})

test('summarize: souhrn podle prvního dílu cesty', () => {
  assert.deepEqual(summarize([{ where: 'docs/a.md:3' }, { where: 'docs/b.md:1' }, { where: 'src/x.ts:2' }, { where: '***.md (jméno souboru)' }, { where: 'commit 1234abcd (předmět)' }]),
    { docs: 2, src: 1, '***.md': 1, commity: 1 })
})

// ---- CLI nad dočasným repem ----
function sh(cwd, ...a) { return execFileSync('git', a, { cwd, encoding: 'utf8', windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }) }
function setup() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ledge-denylist-'))
  const put = (rel, text) => { fs.mkdirSync(path.dirname(path.join(dir, rel)), { recursive: true }); fs.writeFileSync(path.join(dir, rel), text) }
  sh(dir, 'init', '-q', '-b', 'main')
  sh(dir, 'config', 'user.email', 't@example.test'); sh(dir, 'config', 'user.name', 't'); sh(dir, 'config', 'commit.gpgsign', 'false')
  put('.gitattributes', 'scripts/private/ export-ignore\n.github/ export-ignore\n')
  put('scripts/private/denylist.txt', `${W}\n`)
  put('src/a.ts', 'čisté\n')
  put('.github/workflows/x.yml', `interní ${W}\n`) // export-ignore → nehlásí se
  put('.github/verejne-repo/ci.yml', 'čisté\n')
  put('notes.txt', `řádek 1\nřádek 2 ${W}\n`)
  sh(dir, 'add', '-A'); sh(dir, 'commit', '-q', '-m', 'init')
  sh(dir, 'tag', 'v1.0.0')
  return { dir, put }
}
const run = (dir, ...a) => spawnSync(process.execPath, [script, '--root', dir, ...a], { encoding: 'utf8', windowsHide: true })

test('CLI: HEAD hlásí jen soubory, které jdou ven; export-ignore a vlastní denylist se nehlásí', () => {
  const { dir } = setup()
  try {
    const r = run(dir)
    assert.equal(r.status, 1, r.stdout + r.stderr)
    assert.match(r.stdout, /ZAKÁZÁNO: notes\.txt:2 ~ vzor #0/)
    assert.ok(!r.stdout.includes('workflows') && !r.stdout.includes('denylist.txt:'), r.stdout)
    assert.ok(!r.stdout.toLowerCase().includes(W), 'výstup neobsahuje slovo')
    assert.match(r.stdout, /ZÁSAHY: 1 \(notes\.txt: 1\)/)
  } finally { fs.rmSync(dir, { recursive: true, force: true }) }
})

test('CLI: --tree vidí necommitnuté změny, HEAD ne; opravený strom = exit 0', () => {
  const { dir, put } = setup()
  try {
    put('notes.txt', 'čisté\n')
    put('src/new.ts', `export const x = '${W}'\n`) // nový, necommitnutý
    assert.equal(run(dir).status, 1, 'HEAD pořád vidí starý notes.txt')
    const t = run(dir, '--tree')
    assert.equal(t.status, 1)
    assert.match(t.stdout, /src\/new\.ts:1/)
    assert.ok(!/notes\.txt:/.test(t.stdout), t.stdout)
    fs.rmSync(path.join(dir, 'src/new.ts'))
    assert.equal(run(dir, '--tree').status, 0)
  } finally { fs.rmSync(dir, { recursive: true, force: true }) }
})

test('CLI: --dir kontroluje rozbalený klon včetně .github, mimo .git', () => {
  const { dir } = setup()
  const clone = fs.mkdtempSync(path.join(os.tmpdir(), 'ledge-denylist-clone-'))
  try {
    fs.mkdirSync(path.join(clone, '.git')); fs.writeFileSync(path.join(clone, '.git', 'x'), W)
    fs.mkdirSync(path.join(clone, '.github')); fs.writeFileSync(path.join(clone, '.github', 'ci.yml'), `a\n${W}\n`)
    const r = run(dir, '--dir', clone)
    assert.equal(r.status, 1)
    assert.match(r.stdout, /ZAKÁZÁNO: \.github\/ci\.yml:2 ~ vzor #0/)
    assert.ok(!r.stdout.includes('.git/x'))
  } finally { fs.rmSync(dir, { recursive: true, force: true }); fs.rmSync(clone, { recursive: true, force: true }) }
})

test('CLI: --since kontroluje předměty (ne tělo), vynechá SHA z notes-skip.txt, auto = poslední tag', () => {
  const { dir, put } = setup()
  try {
    put('src/b.ts', 'čisté\n'); sh(dir, 'add', '-A'); sh(dir, 'commit', '-q', '-m', `feat: ${W} věc`)
    const bad = sh(dir, 'rev-parse', 'HEAD').trim()
    put('src/c.ts', 'čisté\n'); sh(dir, 'add', '-A'); sh(dir, 'commit', '-q', '-m', 'feat: čisté', '-m', `tělo ${W}`)
    const files = ['notes.txt']; fs.rmSync(path.join(dir, files[0])); sh(dir, 'add', '-A'); sh(dir, 'commit', '-q', '-m', 'chore: úklid')
    const r = run(dir, '--since', 'v1.0.0')
    assert.equal(r.status, 1)
    assert.match(r.stdout, new RegExp(`ZAKÁZÁNO: commit ${bad.slice(0, 8)} \\(předmět\\) ~ vzor #0`))
    assert.equal((r.stdout.match(/ZAKÁZÁNO/g) ?? []).length, 1, r.stdout)
    assert.equal(run(dir, '--since', 'auto').status, 1)
    put('scripts/private/notes-skip.txt', `${bad} # důvod\n`)
    sh(dir, 'add', '-A'); sh(dir, 'commit', '-q', '-m', 'chore: skip') // jen scripts/private → mimo kontrolu
    const ok = run(dir, '--since', 'v1.0.0')
    assert.equal(ok.status, 0, ok.stdout + ok.stderr)
  } finally { fs.rmSync(dir, { recursive: true, force: true }) }
})

test('CLI: bez denylistu exit 0 „přeskočeno“; vadný vzor exit 2 bez výpisu vzoru', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ledge-denylist-'))
  try {
    sh(dir, 'init', '-q', '-b', 'main')
    const r = run(dir)
    assert.equal(r.status, 0)
    assert.match(r.stdout, /přeskočeno/)
    const bad = path.join(dir, 'bad.txt'); fs.writeFileSync(bad, `${W}\n(\n`)
    const e = run(dir, '--denylist', bad)
    assert.equal(e.status, 2)
    assert.match(e.stderr, /vzor #1/)
    assert.ok(!e.stderr.includes(W))
  } finally { fs.rmSync(dir, { recursive: true, force: true }) }
})

// ---- --exe: umělá binárka v temp ----
const u16 = (s) => Buffer.from(s, 'utf16le')
const FAKE = String.raw`C:\Users\jmenotestu\.cargo\registry\src\x.rs` // vymyšlené jméno, ne skutečné
test('scanBlock: ASCII i UTF-16LE (sudý i lichý offset), obecná cesta uživatele; atrapy a zástupné tvary ne', () => {
  const buf = Buffer.concat([
    Buffer.from([1, 2, 3]), Buffer.from(`x ${W} y\0`), u16(`a${W}`), Buffer.from([9]), u16(`b${W}`), // ASCII, UTF-16 sudý, UTF-16 lichý
    Buffer.from(FAKE), Buffer.from('\0'), u16(FAKE),
    Buffer.from(String.raw`C:\Users\demo\a C:\Users\Public\b C:\Users\%USERNAME%\c c:/users/<user>/d`),
  ])
  const hits = scanBlock(buf, 100, pats)
  const kinds = hits.map((h) => `${h.enc}|${h.name}`).sort()
  assert.deepEqual(kinds, [
    'ascii|obecný: cesta uživatele', 'ascii|vzor #0', 'utf16le|obecný: cesta uživatele', 'utf16le|vzor #0', 'utf16le|vzor #0',
  ])
  assert.ok(hits.every((h) => h.offset >= 100), 'offset je absolutní (base + pozice)')
  assert.equal(hits.find((h) => h.enc === 'ascii' && h.name === 'vzor #0').offset, 100 + 3 + 2)
  assert.deepEqual(scanBlock(buf, 0, pats, 5).map((h) => h.offset).filter((o) => o >= 5), [], 'limit odřízne nálezy patřící dalšímu bloku')
})

test('CLI --exe: velký soubor, nález na hranici bloků jen jednou; výstup bez textu; exit 1 / 0 / 2', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ledge-exe-'))
  try {
    const dl = path.join(dir, 'dl.txt'); fs.writeFileSync(dl, `${W}\n`)
    const exe = path.join(dir, 'a.exe')
    const big = Buffer.alloc(9 << 20, 0x41)
    Buffer.from(W).copy(big, (4 << 20) - 3) // přes hranici 1. bloku (přesah)
    u16(FAKE).copy(big, 8 << 20)
    fs.writeFileSync(exe, big)
    const r = spawnSync(process.execPath, [script, '--exe', exe, '--denylist', dl], { encoding: 'utf8', windowsHide: true })
    assert.equal(r.status, 1, r.stdout + r.stderr)
    assert.equal((r.stdout.match(/ZAKÁZÁNO/g) ?? []).length, 2, r.stdout)
    assert.ok(r.stdout.includes(`ZAKÁZÁNO: exe @${(4 << 20) - 3} (ascii) ~ vzor #0`), r.stdout)
    assert.match(r.stdout, /\(utf16le\) ~ obecný: cesta uživatele/)
    assert.ok(!r.stdout.toLowerCase().includes(W) && !r.stdout.includes('jmenotestu'), 'výstup neobsahuje nalezený text')
    fs.writeFileSync(exe, Buffer.from(String.raw`nic C:\Users\demo\x`))
    assert.equal(spawnSync(process.execPath, [script, '--exe', exe, '--denylist', dl], { encoding: 'utf8', windowsHide: true }).status, 0)
    assert.equal(spawnSync(process.execPath, [script, '--exe', path.join(dir, 'neni.exe'), '--denylist', dl], { encoding: 'utf8', windowsHide: true }).status, 2)
  } finally { fs.rmSync(dir, { recursive: true, force: true }) }
})

test('scanFile: jméno souboru se kontroluje u každého typu (GIF, WebM, SVG), obsah binárních jen u nebinárních přípon', () => {
  for (const ext of ['gif', 'webm', 'svg', 'woff2']) assert.equal(scanFile(`${W}.${ext}`, 'x', pats)[0]?.idx, 0, ext)
})
