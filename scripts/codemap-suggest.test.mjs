// Návrh přesunu do detailu — tabulkově.
//   node --experimental-strip-types --test scripts/codemap-suggest.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { suggestMoves, formatSuggestion } from './codemap-suggest.mjs'

const long = (n, tail = '') => 'x'.repeat(n) + tail
const md = [
  '# Mapa', '', 'Detail: [a-detail.md](a-detail.md).', '',
  '| soubor | účel | symboly |', '|---|---|---|',
  `| \`a.rs\` | ${long(300)} | \`A\` |`,
  `| \`b.rs\` | ${long(200, ' (kolo 12)')} | \`B\` |`,
  '| `c.rs` | krátký | `C` |',
  `Historie: ${long(150, ' (kolo 7: přidáno)')}`,
].join('\n')

test('suggestMoves: pod stropem nic', () => {
  const s = suggestMoves(md, 100000)
  assert.deepEqual([s.excess, s.moves.length, s.enough], [0, 0, true])
})

test('suggestMoves: historie kol má přednost, pak nejdelší; hlavičku a krátké řádky nenavrhne', () => {
  const total = Buffer.byteLength(md)
  const s = suggestMoves(md, total - 100) // stačí jeden řádek, a to historie (kratší, ale kolo)
  assert.equal(s.moves.length, 1)
  assert.equal(s.moves[0].reason, 'historie kola')
  assert.ok(s.enough)
  const all = suggestMoves(md, total - 400) // vyžaduje víc řádků
  assert.deepEqual(all.moves.map((m) => m.line), [7, 8, 10].filter((l) => all.moves.some((m) => m.line === l)))
  assert.ok(all.moves.every((m) => ![1, 3, 5, 6, 9].includes(m.line)))
  assert.ok(all.enough)
})

test('suggestMoves: nestačí → enough=false', () => {
  const s = suggestMoves(md, 10)
  assert.equal(s.enough, false)
  assert.equal(s.moves.length, 3)
})

test('formatSuggestion: prázdné bez přebytku, jinak řádky', () => {
  assert.deepEqual(formatSuggestion('a.md', suggestMoves(md, 100000)), [])
  const out = formatSuggestion('a.md', suggestMoves(md, 10))
  assert.match(out[0], /a\.md.*a-detail\.md.*nic se nepřepisuje/)
  assert.match(out.at(-1), /nestačí/)
})
