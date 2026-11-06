// Návrh přesunu řádků z jádra codemapy do `-detail.md`, když jádro překročí strop (viz codemap-lint.mjs --fix).
// Čistá funkce bez I/O; test: scripts/codemap-suggest.test.mjs. Nic nepřepisuje — jen navrhuje (přesun je úsudek:
// řádek tabulky nese soubory pro pokrytí, historie kol do detailu patří vždy).
const SEP = /^\|[\s|:-]+\|?\s*$/
const KOLO = /\(kolo \d+[^)]*\)|\bkolo \d+\b/i

/**
 * @returns {{ total:number, excess:number, moves:{line:number,bytes:number,reason:string,preview:string}[], enough:boolean }}
 * `moves` = nejdřív řádky s historií kol („(kolo N)“), pak nejdelší řádky; bere se, dokud nepokryjí přebytek.
 */
export function suggestMoves(md, max) {
  const total = Buffer.byteLength(md)
  const excess = total - max
  if (excess <= 0) return { total, excess: 0, moves: [], enough: true }
  const lines = md.split(/\r?\n/)
  const cands = []
  lines.forEach((l, i) => {
    const bytes = Buffer.byteLength(l) + 1
    if (bytes < 120 || /^#/.test(l) || l.includes('-detail.md') || SEP.test(l)) return
    if (SEP.test(lines[i + 1] ?? '')) return // hlavička tabulky
    cands.push({ line: i + 1, bytes, history: KOLO.test(l), preview: l.replace(/\s+/g, ' ').slice(0, 90) })
  })
  cands.sort((a, b) => Number(b.history) - Number(a.history) || b.bytes - a.bytes)
  const moves = []
  let saved = 0
  for (const c of cands) {
    if (saved >= excess) break
    moves.push({ line: c.line, bytes: c.bytes, reason: c.history ? 'historie kola' : 'nejdelší řádek', preview: c.preview })
    saved += c.bytes
  }
  moves.sort((a, b) => a.line - b.line)
  return { total, excess, moves, enough: saved >= excess }
}

/** Čitelný výpis návrhu (pole řádků). */
export function formatSuggestion(name, s) {
  if (!s.excess) return []
  const det = name.replace(/\.md$/, '-detail.md')
  const out = [`${name}: ${s.total} B, přebývá ${s.excess} B — návrh přesunu do ${det} (nic se nepřepisuje):`]
  for (const m of s.moves) out.push(`  řádek ${m.line} (${m.bytes} B, ${m.reason}): ${m.preview}…`)
  if (!s.enough) out.push('  ani všechny kandidáty nestačí — rozděl mapu po podoblastech nebo zkrať buňky ručně')
  return out
}
