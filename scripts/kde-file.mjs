// kde.mjs --file <cesta>: řádek codemapy pro soubor + osnova deklarací s čísly řádků (bez těl).
// Proč: agent místo čtení celého souboru uvidí, co v něm je, a přečte jen výřez. Čisté funkce (bez I/O) —
// `outline`, `mapRows`; I/O (čtení souboru a map) je v kde.mjs. Test: scripts/kde.test.mjs.
import { compileToken, pathTokens } from './codemap-coverage.mjs'

const OUTLINE_RS = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+)*(?:fn|struct|enum|trait|impl|mod|type)\b/
const OUTLINE_TS = /^\s*export\s+(?:default\s+)?(?:async\s+)?(?:function\*?|const|let|class|type|interface|enum)\b/

const cut = (s, n) => { s = s.trim().replace(/\s+/g, ' '); return s.length > n ? s.slice(0, n - 1) + '…' : s }

/** Osnova zdroje: [{ line, text }] — jen hlavičky deklarací (fn/struct/enum/trait/impl/mod/type; export function|const|type…). */
export function outline(lines, file) {
  const re = /\.rs$/.test(file) ? OUTLINE_RS : /\.(tsx?|mjs|js)$/.test(file) ? OUTLINE_TS : null
  if (!re) return []
  const out = []
  lines.forEach((text, i) => {
    if (re.test(text)) out.push({ line: i + 1, text: cut(text.replace(/\s*\{\s*$/, ''), 110) })
  })
  return out
}

/**
 * Řádky map, které jmenují soubor: [{ map, line, text }]. mapTexts = [[název, text]] jen JADER (ne -detail).
 * Přesná cesta vyhrává; zkrácený zápis (`oauth.rs`) platí, jen když nic přesně nepojmenuje.
 */
export function mapRows(mapTexts, file) {
  const hits = []
  for (const [map, md] of mapTexts) {
    md.split(/\r?\n/).forEach((l, i) => {
      if (!l.startsWith('|') && !/^`/.test(l)) return
      const toks = pathTokens(l).map(compileToken).filter((c) => !c.bare)
      if (toks.some((c) => c.rooted.some((re) => re.test(file)))) hits.push({ map, line: i + 1, text: l, exact: true })
      else if (toks.some((c) => c.suffix.some((re) => re.test(file)))) hits.push({ map, line: i + 1, text: l, exact: false })
    })
  }
  return hits.some((h) => h.exact) ? hits.filter((h) => h.exact) : hits
}

/** Text výstupu (čisté): řádky mapy + osnova. */
export function renderFile(file, rows, lines, o = outline(lines, file)) {
  const out = ['# codemapa: ' + file]
  if (!rows.length) out.push('  (žádný řádek v docs/codemap/*.md)')
  for (const h of rows.slice(0, 3)) out.push(`docs/codemap/${h.map}:${h.line}  ${cut(h.text, 400)}`)
  out.push(`# osnova (${o.length} deklarací z ${lines.length} řádků)`)
  for (const e of o) out.push(`${file}:${e.line}  ${e.text}`)
  return out.join('\n')
}
