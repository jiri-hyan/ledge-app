// wt.mjs: čisté funkce + celý cyklus new/rm v DOČASNÉM repu (nikdy skutečné ledge-wt-*).
//   node --experimental-strip-types --test scripts/wt.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { execFileSync } from 'node:child_process'
import { planNames, unlinkLink, isLink, selectStale, findIncrementalDirs, prune, newWorktree, removeWorktree } from './wt.mjs'

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'ledge-wt-test-'))
const git = (cwd, ...a) => execFileSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@example.com', ...a], { cwd, encoding: 'utf8', windowsHide: true }).trim()

test('planNames: tabulka', () => {
  assert.deepEqual(planNames('proces', 29), { branch: 'k29/proces', dirName: 'ledge-wt-proces' })
  assert.deepEqual(planNames('acl-ui', '30'), { branch: 'k30/acl-ui', dirName: 'ledge-wt-acl-ui' })
  for (const [o, k] of [['', 1], ['A', 1], ['../x', 1], ['a b', 1], ['ok', ''], ['ok', 'x'], ['ok', '1;rm']]) assert.throws(() => planNames(o, k), o + '/' + k)
})

test('selectStale: hranice stáří', () => {
  const now = 10 * 86400000
  const items = [{ path: 'a', mtimeMs: now - 4 * 86400000 }, { path: 'b', mtimeMs: now - 2 * 86400000 }, { path: 'c', mtimeMs: now - 3 * 86400000 }]
  assert.deepEqual(selectStale(items, now, 3).map((i) => i.path), ['a', 'c'])
  assert.deepEqual(selectStale(items, now, 0).length, 3)
  assert.deepEqual(selectStale([], now, 3), [])
})

test('unlinkLink: odpojí junction a obsah cíle zůstane', () => {
  const d = tmp()
  try {
    const target = path.join(d, 'cil'); fs.mkdirSync(path.join(target, 'sub'), { recursive: true })
    fs.writeFileSync(path.join(target, 'a.txt'), 'x'); fs.writeFileSync(path.join(target, 'sub', 'b.txt'), 'y')
    const link = path.join(d, 'odkaz'); fs.symlinkSync(target, link, 'junction')
    assert.ok(isLink(link))
    assert.ok(unlinkLink(link))
    assert.ok(!fs.existsSync(link))
    assert.equal(fs.readFileSync(path.join(target, 'a.txt'), 'utf8'), 'x')
    assert.equal(fs.readFileSync(path.join(target, 'sub', 'b.txt'), 'utf8'), 'y')
    // skutečná složka se nikdy nemaže
    assert.equal(unlinkLink(target), false)
    assert.ok(fs.existsSync(target))
  } finally { fs.rmSync(d, { recursive: true, force: true }) }
})

test('new + rm v dočasném repu: junction, dist, rerere; rm nesmaže cíl junctionu', () => {
  const d = tmp()
  try {
    const main = path.join(d, 'Ledge'); fs.mkdirSync(main)
    git(main, 'init', '-b', 'main'); fs.writeFileSync(path.join(main, '.gitignore'), 'node_modules\ndist\n'); fs.writeFileSync(path.join(main, 'f.txt'), '1')
    git(main, 'add', '.'); git(main, 'commit', '-m', 'init')
    fs.mkdirSync(path.join(main, 'node_modules', 'pkg'), { recursive: true }); fs.writeFileSync(path.join(main, 'node_modules', 'pkg', 'i.js'), 'z')
    fs.mkdirSync(path.join(main, 'dist')); fs.writeFileSync(path.join(main, 'dist', 'index.html'), 'h')
    const r = newWorktree(main, 'abc', 5)
    assert.equal(r.branch, 'k5/abc')
    assert.equal(r.wt, path.join(d, 'ledge-wt-abc'))
    assert.ok(isLink(path.join(r.wt, 'node_modules')))
    assert.equal(fs.readFileSync(path.join(r.wt, 'node_modules', 'pkg', 'i.js'), 'utf8'), 'z')
    assert.ok(!isLink(path.join(r.wt, 'dist')))
    assert.equal(fs.readFileSync(path.join(r.wt, 'dist', 'index.html'), 'utf8'), 'h')
    assert.equal(git(main, 'config', 'rerere.enabled'), 'true')
    assert.throws(() => newWorktree(main, 'abc', 5), /už existuje/)
    // špinavý worktree se odmítne a nic se nepoškodí
    fs.writeFileSync(path.join(r.wt, 'nove.txt'), 'n')
    assert.throws(() => removeWorktree(main, 'abc'), /není čistý/)
    assert.ok(isLink(path.join(r.wt, 'node_modules')))
    fs.rmSync(path.join(r.wt, 'nove.txt'))
    const out = removeWorktree(main, 'abc')
    assert.deepEqual(out.unlinked, ['node_modules'])
    assert.ok(!fs.existsSync(r.wt))
    // OBSAH CÍLE JUNCTIONU PŘEŽIL
    assert.equal(fs.readFileSync(path.join(main, 'node_modules', 'pkg', 'i.js'), 'utf8'), 'z')
    assert.match(out.branchMsg, /smazána/)
    assert.ok(!git(main, 'branch', '--list', 'k5/abc'))
  } finally { fs.rmSync(d, { recursive: true, force: true }) }
})

test('rm: nesloučená větev se ponechá (jen -d)', () => {
  const d = tmp()
  try {
    const main = path.join(d, 'Ledge'); fs.mkdirSync(main)
    git(main, 'init', '-b', 'main'); fs.writeFileSync(path.join(main, 'f.txt'), '1'); git(main, 'add', '.'); git(main, 'commit', '-m', 'init')
    const r = newWorktree(main, 'x', 1)
    fs.writeFileSync(path.join(r.wt, 'g.txt'), '2'); git(r.wt, 'add', 'g.txt'); git(r.wt, 'commit', '-m', 'wip')
    const out = removeWorktree(main, 'x')
    assert.match(out.branchMsg, /ponechána/)
    assert.ok(git(main, 'branch', '--list', 'k1/x'))
  } finally { fs.rmSync(d, { recursive: true, force: true }) }
})

test('prune: dry-run nic nemaže, --apply maže jen staré v incremental/', () => {
  const d = tmp()
  try {
    const mk = (rel, ageDays) => {
      const p = path.join(d, rel); fs.mkdirSync(p, { recursive: true }); fs.writeFileSync(path.join(p, 'x.bin'), 'x'.repeat(100))
      const t = new Date(Date.now() - ageDays * 86400000); fs.utimesSync(path.join(p, 'x.bin'), t, t); fs.utimesSync(p, t, t); return p
    }
    const old = mk('debug/incremental/ledge_lib-old', 10), fresh = mk('debug/incremental/ledge_lib-new', 0)
    const rel = mk('release/incremental/ledge-old', 9)
    const dep = mk('debug/deps/keep', 30) // deps se nikdy nehledá
    assert.equal(findIncrementalDirs(d).length, 2)
    const dry = prune(d, { days: 3 })
    assert.equal(dry.stale.length, 2); assert.equal(dry.total, 200); assert.equal(dry.kept, 1)
    assert.ok(fs.existsSync(old) && fs.existsSync(rel))
    const real = prune(d, { days: 3, apply: true })
    assert.equal(real.applied, true)
    assert.ok(!fs.existsSync(old) && !fs.existsSync(rel))
    assert.ok(fs.existsSync(fresh) && fs.existsSync(dep))
  } finally { fs.rmSync(d, { recursive: true, force: true }) }
})
