// Výběr bran podle změněných souborů (pro `node scripts/brany.mjs --changed`). Čistá funkce bez I/O —
// tabulkový test: scripts/brany-select.test.mjs. Proč: plný běh (test, clippy, build) trvá minuty a worker
// po změně jedné dokumentace nebo jednoho .tsx nepotřebuje cargo.
//
//   src/**, *.css, *.ts(x), *.mjs, *.html (UI)        → tsc ui
//   src-tauri/** (Rust)                                → test clippy
//   UI i Rust, nebo package.json / Cargo.* / tauri.conf.json / lockfile → test clippy tsc ui (vše kromě build)
//   docs/**, *.md                                      → ui (test hniloby codemap čte docs i kód)
//   cokoli jiného (.github, LICENSE, *.ps1…)           → samo o sobě žádná brána
// Docs spolu s jiným druhem změny jen přidají `ui`. Pořadí bran drží brany.mjs (test, clippy, tsc, ui).

const ORDER = ['test', 'clippy', 'tsc', 'ui']
const WIDE = /^(package\.json|pnpm-lock\.yaml|src-tauri\/(Cargo\.(toml|lock)|tauri\.conf\.json))$/

/** Druh změny jednoho souboru: 'wide' | 'rust' | 'ui' | 'docs' | 'other'. */
export function kindOf(file) {
  const f = file.split(String.fromCharCode(92)).join('/') // zpětná lomítka z Windows
  if (WIDE.test(f)) return 'wide'
  if (f.startsWith('src-tauri/')) return 'rust'
  if (f.startsWith('docs/') || f.endsWith('.md')) return 'docs'
  if (f.startsWith('src/') || /\.(css|tsx?|mjs|html)$/.test(f)) return 'ui'
  return 'other'
}

/** files → { gates: string[], why: string } */
export function selectGates(files) {
  const kinds = new Set(files.map(kindOf))
  const n = (k) => files.filter((f) => kindOf(f) === k).length
  if (!files.length) return { gates: [], why: 'žádné změněné soubory' }
  let want
  let why
  if (kinds.has('wide') || (kinds.has('rust') && kinds.has('ui'))) {
    want = ['test', 'clippy', 'tsc', 'ui']
    why = kinds.has('wide') ? 'změněn package.json / Cargo.* / tauri.conf.json → vše kromě build' : 'změněn UI i Rust → vše kromě build'
  } else if (kinds.has('rust')) {
    want = ['test', 'clippy']
    why = `změněno jen src-tauri/** (${n('rust')} souborů)`
  } else if (kinds.has('ui')) {
    want = ['tsc', 'ui']
    why = `změněno jen UI — src/**, ts, css, mjs (${n('ui')} souborů)`
  } else if (kinds.has('docs')) {
    want = ['ui']
    why = `změněny jen docs / *.md (${n('docs')} souborů) → ui kvůli testu codemap`
  } else {
    return { gates: [], why: 'změněno jen mimo kód a docs (.github, skripty .ps1…) — žádná brána' }
  }
  if (kinds.has('docs') && !want.includes('ui')) {
    want.push('ui')
    why += ' + docs → ui'
  }
  return { gates: ORDER.filter((g) => want.includes(g)), why }
}
