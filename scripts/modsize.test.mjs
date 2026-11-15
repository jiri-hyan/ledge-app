// Strop modulů: čistá pravidla (tabulkově) — Rust se testy za `#[cfg(test)] mod` se počítá jen produkčně.
//   node --experimental-strip-types --test scripts/modsize.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { isTestFile, oversize, prodLines } from './modsize.mjs'

const rows = (n, s = 'x') => Array.from({ length: n }, () => s).join('\n') + '\n'

test('prodLines: Rust řeže na #[cfg(test)] mod, ostatní celý soubor', () => {
  for (const [file, text, want] of [
    ['a.rs', rows(10) + '#[cfg(test)]\nmod tests {\n' + rows(50) + '}\n', 10],
    ['a.rs', rows(10) + '#[cfg(test)]\npub(crate) mod t {\n}\n', 10],
    ['a.rs', rows(10) + '#[cfg(test)]\nfn helper() {}\n', 12], // cfg(test) na funkci není blok testů
    ['a.rs', rows(7), 7],
    ['a.rs', '', 0],
    ['a.tsx', rows(9), 9],
    ['a.ts', rows(3) + '#[cfg(test)]\nmod tests {}\n', 5], // jen Rust se řeže
  ]) assert.equal(prodLines(file, text), want, `${file} ${want}`)
})

test('isTestFile: testovací soubory ne, produkční ano', () => {
  for (const [f, want] of [
    ['src-tauri/src/looks/tests.rs', true],
    ['src-tauri/src/notes/folder_sync_tests.rs', true],
    ['src-tauri/src/claude/testkit.rs', true],
    ['src/pin/model.test.ts', true],
    ['src/vite-env.d.ts', true],
    ['src-tauri/src/strip.rs', false],
    ['src/pin/PinApp.tsx', false],
  ]) assert.equal(isTestFile(f), want, f)
})

test('oversize: nad stropem jen produkční řádky, seřazeno sestupně', () => {
  const e = [
    { file: 'a.rs', text: rows(500) + '#[cfg(test)]\nmod tests {\n' + rows(400) + '}\n' }, // prod 500 → ne
    { file: 'b.rs', text: rows(700) },
    { file: 'c.tsx', text: rows(650) },
    { file: 'd_tests.rs', text: rows(900) }, // čistě testy → ne
  ]
  assert.deepEqual(oversize(e, 600).map((o) => [o.file, o.prod]), [['b.rs', 700], ['c.tsx', 650]])
  assert.deepEqual(oversize(e, 800), [])
})
