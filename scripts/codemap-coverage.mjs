// Pokrytí codemap: opak lintu hniloby. Lint hlídá, že mapa neodkazuje na neexistující věci; tohle hlídá,
// že KAŽDÝ modul kódu nějaká mapa zmiňuje. Bez toho nový modul (soubor v src-tauri/src, složka v src/)
// zůstane pro dalšího workera neviditelný — a mapy mlčky zahnívají doprava.
// Čisté funkce (bez I/O); test: scripts/codemap-coverage.test.mjs; CLI: node scripts/codemap-lint.mjs.
//
// Dvě úrovně (obě musí platit):
//  1) MODUL: první úroveň src-tauri/src (soubor `x.rs` nebo složka `x/`) a první úroveň src/ (složka nebo
//     soubor) je pokrytý, když některá mapa (jakýkoli backtickový token v docs/codemap/*.md, i README)
//     jmenuje cestu uvnitř něj — i holé `složka/*`.
//  2) SOUBOR: každý produkční zdroj (.rs .ts .tsx .mjs; ne testy, fixtures, mod.rs / index.ts) musí být
//     jmenován výslovně — přesná cesta, `{a,b}`, nebo vzor s literálním kusem (`en-*.ts`, `Appearance*.tsx`).
//     Holé `složka/*` nebo `složka/*.rs` na to NESTAČÍ (jediný řádek by „pokryl“ cokoli, co se do složky
//     přidá). Tohle je pojistka, že nový soubor dostane řádek v mapě.
// Jak se token páruje se souborem: od kořene repa, od `src-tauri/src/` nebo od `src/` (`x.rs` ≙ `x/mod.rs`,
// `x.ts` ≙ `x/index.ts`). Mapy píšou i zkráceně (`oauth.rs` v accounts.md) — zkrácený token platí jen
// v mapě, která ten modul jinde jmenuje celou cestou, a jen když token nic nepojmenuje přesně
// (jinak by `timer.rs` kryl i `commands/timer.rs`).
import { expandBraces, isPathToken } from './codemap-lint.mjs'

// Výjimky — klíč je modul (`src/x`, `src-tauri/src/x[.rs]`) nebo soubor, hodnota důvod. Zbytečná výjimka
// (cíl už pokrytý nebo neexistuje) je chyba, ať seznam nehnije.
export const COVERAGE_ALLOW = {}

const ROOTS = ['', 'src-tauri/src/', 'src/']
const RUST = 'src-tauri/src/'
const ROOT_DIRS = new Set(['src', 'src-tauri', 'src-tauri/src'])
const BARE_WILD = /(^|\/)[*]+([.][A-Za-z0-9]+)?$/

/** Modul (id) souboru, nebo null, když se do pokrytí nepočítá. Id: `src-tauri/src/x[.rs]`, `src/x[.ext]`. */
export function unitOf(file) {
  const root = file.startsWith(RUST) ? RUST : file.startsWith('src/') ? 'src/' : null
  if (!root) return null
  if (root === 'src/' && (/[.]test[.]mjs$/.test(file) || /[.]d[.]ts$/.test(file))) return null
  return root + file.slice(root.length).split('/')[0]
}

/** Produkční zdroj, který musí být v mapě jmenován výslovně (úroveň 2)? */
export function needsNaming(file) {
  if (unitOf(file) === null || !/[.](rs|ts|tsx|mjs)$/.test(file)) return false
  if (/(^|\/)(mod[.]rs|index[.]tsx?|tests?[.]rs|testkit[.]rs)$|_tests[.]rs$|[.]test[.]mjs$|[.]d[.]ts$/.test(file)) return false
  return !file.startsWith(RUST + 'fixtures/')
}

function globRe(p) {
  const B = String.fromCharCode(92)
  let re = ''
  for (let i = 0; i < p.length; i++) {
    const c = p[i]
    if (c === '*') {
      if (p[i + 1] === '*') { re += '.*'; i++ } else re += '[^/]*'
    } else re += c.replace(/[.+?^${}()|[\]]/g, (x) => B + x)
  }
  return re
}

/** Backtickové cesty z jedné mapy (rozbalené `{a,b}`, bez `./` a koncového `/`). */
export function pathTokens(md) {
  const out = new Set()
  for (const m of md.matchAll(/`([^`]+)`/g)) {
    const t = m[1].trim()
    if (!isPathToken(t)) continue
    for (const q of expandBraces(t)) {
      const clean = q.replace(/^\.\//, '').replace(/\/+$/, '')
      if (!ROOT_DIRS.has(clean)) out.add(clean) // `src/` samo o sobě nejmenuje žádný modul
    }
  }
  return [...out]
}

/** Varianty cesty: `x.rs` ≙ `x/mod.rs`, `x.ts(x)` ≙ `x/index.ts(x)`. */
function variants(p) {
  if (p.endsWith('.rs')) return [p, p.slice(0, -3) + '/mod.rs']
  if (/[.]tsx?$/.test(p)) return [p, p.replace(/[.]tsx?$/, (e) => '/index' + e)]
  return [p]
}

/** Zkompilovaný token: rooted = shoda od kořene / src-tauri/src / src, suffix = zkrácený zápis, dirs = složka. */
export function compileToken(tok) {
  const vs = variants(tok)
  const isDir = !/[.][A-Za-z0-9]+$/.test(tok) && !tok.includes('*')
  return {
    tok,
    bare: BARE_WILD.test(tok),
    rooted: ROOTS.flatMap((r) => vs.map((v) => new RegExp('^' + globRe(r + v) + '$'))),
    suffix: vs.map((v) => new RegExp('(^|/)' + globRe(v) + '$')),
    dirs: isDir ? ROOTS.map((r) => r + tok + '/') : [],
  }
}

const rootedHit = (c, file, named) =>
  !(named && c.bare) && (c.rooted.some((re) => re.test(file)) || (!named && c.dirs.some((d) => file.startsWith(d))))
const suffixHit = (c, file) => !c.bare && c.suffix.some((re) => re.test(file))

/** Pokrývá token `tok` soubor `file` (jen od kořene; `named` = ne holá `složka/*`, ne složka)? */
export function tokenCovers(tok, file, named = false) {
  return rootedHit(compileToken(tok), file, named)
}

/**
 * Mezery v pokrytí. files = soubory repa, mds = texty všech docs/codemap/*.md, allow = výjimky.
 * → { missing: moduly bez mapy, unnamed: soubory bez řádku v mapě, staleAllow: zbytečné výjimky }
 */
export function coverageGaps(files, mds, allow = COVERAGE_ALLOW) {
  const maps = mds.map((md) => pathTokens(md).map(compileToken))
  // zkrácený zápis (`oauth.rs`) smí kryt soubor jen tehdy, když token od kořene nic nepřesně nepojmenuje —
  // jinak by `timer.rs` (= src-tauri/src/timer.rs) „kryl“ i commands/timer.rs
  for (const cs of maps) for (const c of cs) c.exact = files.some((f) => rootedHit(c, f, true))
  const byUnit = new Map()
  for (const f of files) {
    const u = unitOf(f)
    if (!u) continue
    if (!byUnit.has(u)) byUnit.set(u, [])
    byUnit.get(u).push(f)
  }
  // moduly, které mapa jmenuje celou cestou (jakkoli, i `složka/*`) — podmínka pro zkrácené zápisy v téže mapě
  const unitsIn = maps.map((cs) => {
    const set = new Set()
    for (const [u, fs_] of byUnit) if (fs_.some((f) => cs.some((c) => rootedHit(c, f, false)))) set.add(u)
    return set
  })
  const hit = new Set()
  const missing = []
  for (const u of byUnit.keys()) (unitsIn.some((s) => s.has(u)) ? hit.add(u) : missing.push(u))

  const unnamed = []
  const named = new Set()
  for (const f of files.filter(needsNaming)) {
    const u = unitOf(f)
    const ok = maps.some((cs, i) => cs.some((c) => rootedHit(c, f, true) || (!c.exact && unitsIn[i].has(u) && suffixHit(c, f))))
    if (ok) named.add(f)
    else unnamed.push(f)
  }
  const staleAllow = Object.keys(allow).filter((k) => (byUnit.has(k) ? hit.has(k) : !files.includes(k) || named.has(k)))
  const free = (k) => !(k in allow)
  return { missing: missing.filter(free).sort(), unnamed: unnamed.filter(free).sort(), staleAllow }
}
