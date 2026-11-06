// Pomocníci brany.mjs — tabulkově.
//   node --experimental-strip-types --test scripts/brany-lib.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { parseFailedTests, canRetry, retryArgs, flakyLine, stampText, stampMatches, isFullRun, buildCommand } from './brany-lib.mjs'

const ALL = ['test', 'clippy', 'tsc', 'ui', 'build']

const out1 = [
  'running 3 tests',
  'test notes::store::tests::concurrent_set_done_keeps_file_valid ... FAILED',
  'test a::ok ... ok',
  'failures:',
  'test result: FAILED. 2 passed; 1 failed; 0 ignored',
  'error: test failed, to rerun pass `--lib`',
  'error: 1 target failed:',
].join('\n')

test('parseFailedTests: jména z výstupu cargo test', () => {
  assert.deepEqual(parseFailedTests(out1), ['notes::store::tests::concurrent_set_done_keeps_file_valid'])
  assert.deepEqual(parseFailedTests('test a::b ... FAILED\r\ntest a::b ... FAILED\r\ntest c ... FAILED\r\n'), ['a::b', 'c'])
  assert.deepEqual(parseFailedTests('test a ... ok\ntest b ... ignored'), [])
  assert.deepEqual(parseFailedTests(''), [])
})

test('canRetry: opakuje se jen čistě padlé pojmenované testy', () => {
  const f = parseFailedTests(out1)
  assert.equal(canRetry(out1, f), true)
  assert.equal(canRetry(out1, []), false) // žádné jméno
  assert.equal(canRetry(out1, ['a', 'b', 'c', 'd', 'e', 'f']), false) // moc padlých = regrese
  assert.equal(canRetry(out1 + '\nerror[E0425]: cannot find value', f), false) // chyba sestavení
  assert.equal(canRetry(out1 + '\nerror: could not compile `ledge`', f), false)
  assert.equal(canRetry(out1.replace('1 failed', '3 failed'), f), false) // počty nesedí (padl i netestový cíl)
})

test('retryArgs: jen bezpečná jména', () => {
  assert.deepEqual(retryArgs(['a::b', 'c_d']), ['--exact', 'a::b', 'c_d'])
  assert.equal(retryArgs(['a b']), null)
  assert.equal(retryArgs(['a;rm']), null)
})

test('flakyLine: přizná opakování', () => {
  assert.match(flakyLine(['a::b'], true, 4), /FLAKY.*2\. pokus.*NENÍ.*čistý.*a::b/)
  assert.match(flakyLine(['a::b'], false, 4), /FAIL.*není flaky/)
})

test('razítko: shoda jen se stejným hashem', () => {
  const t = stampText('abc123', new Date('2026-01-02T03:04:05Z'))
  assert.equal(t, 'abc123\n2026-01-02T03:04:05.000Z\n')
  for (const [text, hash, want] of [[t, 'abc123', true], [t, 'abc124', false], ['', 'abc123', false], [t, null, false], ['abc123\r\nx', 'abc123', true]])
    assert.equal(stampMatches(text, hash), want, `${text}|${hash}`)
})

test('isFullRun: jen všech pět bran', () => {
  assert.equal(isFullRun(ALL, ALL), true)
  assert.equal(isFullRun(['ui', 'test', 'build', 'clippy', 'tsc'], ALL), true)
  assert.equal(isFullRun(['test', 'ui'], ALL), false)
  assert.equal(isFullRun([], ALL), false)
})

test('buildCommand: vite build samotný, když tsc běží ve stejném běhu', () => {
  const q = (c) => `Q(${c})`
  assert.equal(buildCommand(ALL, q), 'Q(corepack pnpm exec vite build)')
  assert.equal(buildCommand(['build'], q), 'Q(corepack pnpm build)')
  assert.equal(buildCommand(['ui', 'build'], q), 'Q(corepack pnpm build)')
})
