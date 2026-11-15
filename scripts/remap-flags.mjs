// Release build bez absolutních cest uživatele v binárce: do RUSTFLAGS přidá `--remap-path-prefix` pro domovskou složku cargo,
// rustup, kořen repa, cílovou složku a profil uživatele (panic lokace závislostí `…\.cargo\registry\src\…` jinak nesou jméno
// účtu — v ledge.exe 806×). Proč proměnná prostředí, ne `.cargo/config.toml`: config nezná proměnné (cesty jsou na každém
// stroji jiné) a `rustflags` v něm platí pro všechny profily, takže by změna přebuildovala i debug/test cache sdíleného
// CARGO_TARGET_DIR; `trim-paths` je nestabilní (nightly `-Ztrim-paths`) a per-profile rustflags také. Přes proměnnou se mění
// jen build, který ji dostane (cargo má rustflags v hashi jednotky → release s remapem a release bez něj se nemíchají).
// `file!()` vlastního kódu zůstává relativní (`src\…`) — cargo ho předává rustc relativně k pracovnímu adresáři.
//
//   node scripts/remap-flags.mjs --run -- <příkaz…>   → spustí příkaz s doplněným RUSTFLAGS (release.ps1, QA build)
//   node scripts/remap-flags.mjs                      → vypíše výsledné RUSTFLAGS (bez spuštění)
// Kontrola výsledku: `node scripts/denylist-check.mjs --exe <ledge.exe>` (publish.ps1 ji dělá sám).
import { spawnSync } from 'node:child_process'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))

/** Seznam [předpona, náhrada] od obecných ke konkrétním (rustc při shodě více předpon bere POSLEDNÍ). */
export function remapPairs({ root, cargoHome, rustupHome, targetDir, home }) {
  const out = []
  const add = (from, to) => {
    if (!from) return
    const clean = String(from).replace(/[\\/]+$/, '')
    if (!clean) return
    for (const v of new Set([clean, clean.replace(/\\/g, '/')])) if (!out.some(([f]) => f === v)) out.push([v, to])
  }
  add(home, '/home')
  add(rustupHome, '/rustup')
  add(cargoHome, '/cargo')
  add(targetDir, '/target')
  add(root, '/ledge')
  return out
}

/** Doplní remap do RUSTFLAGS / CARGO_ENCODED_RUSTFLAGS (podle toho, co už je v prostředí; encoded má přednost, odděluje \x1f). */
export function withRemap(env, pairs) {
  const flags = pairs.map(([f, t]) => `--remap-path-prefix=${f}=${t}`)
  const out = { ...env }
  if (out.CARGO_ENCODED_RUSTFLAGS) out.CARGO_ENCODED_RUSTFLAGS += '\x1f' + flags.join('\x1f')
  else if (flags.some((f) => f.includes(' '))) out.CARGO_ENCODED_RUSTFLAGS = [out.RUSTFLAGS, ...flags].filter(Boolean).join('\x1f') // mezera v cestě: RUSTFLAGS by ji rozdělil
  else out.RUSTFLAGS = [out.RUSTFLAGS, ...flags].filter(Boolean).join(' ')
  return out
}

/** Pár z aktuálního prostředí a repa. */
export function currentPairs(env = process.env) {
  const home = env.USERPROFILE || os.homedir()
  const root = path.resolve(here, '..')
  const targetDir = env.CARGO_TARGET_DIR ? path.resolve(root, env.CARGO_TARGET_DIR) : path.join(root, 'src-tauri', 'target')
  return remapPairs({
    root, home, targetDir,
    cargoHome: env.CARGO_HOME || path.join(home, '.cargo'),
    rustupHome: env.RUSTUP_HOME || path.join(home, '.rustup'),
  })
}

function main(argv) {
  const env = withRemap(process.env, currentPairs())
  const run = argv.indexOf('--run')
  if (run < 0) { console.log(env.CARGO_ENCODED_RUSTFLAGS ? env.CARGO_ENCODED_RUSTFLAGS.split('\x1f').join(' ') : env.RUSTFLAGS); return 0 }
  const cmd = argv.slice(argv.indexOf('--', run) + 1)
  if (!cmd.length) { console.error('remap-flags: chybí příkaz za --'); return 2 }
  const r = spawnSync(cmd[0], cmd.slice(1), { env, stdio: 'inherit', shell: process.platform === 'win32', windowsHide: true, cwd: process.cwd() })
  return r.status ?? 1
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) process.exit(main(process.argv.slice(2)))
