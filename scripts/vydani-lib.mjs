// Pojistky vydání — čisté funkce (volá je scripts/vydani-check.mjs z publish.ps1 / install-local.ps1; test vydani-lib.test.mjs).
import { stampMatches } from './brany-lib.mjs'

/** L213: razítko `.brany-ok` (1. řádek = hash stromu po plném PASS) musí odpovídat stromu HEAD. */
export function gateCheck(stampTextOrNull, headTree) {
  if (!headTree) return { ok: false, msg: 'nejde zjistit strom HEAD (git rev-parse HEAD^{tree})' }
  if (!stampTextOrNull) return { ok: false, msg: 'chybí razítko .brany-ok — plné brány (node scripts/brany.mjs) nad tímto stromem neprošly' }
  if (!stampMatches(stampTextOrNull, headTree)) return { ok: false, msg: `razítko .brany-ok neodpovídá stromu HEAD (${headTree.slice(0, 10)}) — spusť plné brány (node scripts/brany.mjs) nad finálním stromem` }
  return { ok: true, msg: `brány: razítko odpovídá stromu ${headTree.slice(0, 10)}` }
}

/** L214: poznámky vydání `docs/releases/<verze>.md` musí existovat a nesmí být prázdné. */
export function notesCheck(version, text) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) return { ok: false, msg: `verze '${version}' není x.y.z` }
  if (text == null) return { ok: false, msg: `chybí docs/releases/${version}.md — napiš poznámky vydání (odrážky „- **Česky / English:** …“), bez nich se nevydává` }
  if (!text.trim()) return { ok: false, msg: `docs/releases/${version}.md je prázdný` }
  return { ok: true, msg: `poznámky: docs/releases/${version}.md` }
}

/** Verze ze zdrojů: tauri.conf.json / package.json (klíč "version"), Cargo.toml ([package]), Cargo.lock (balíček ledge). */
export function extractVersions({ tauriConf, packageJson, cargoToml, cargoLock }) {
  const json = (t) => JSON.parse(t).version ?? null
  const pkg = /^\[package\]\s*$[\s\S]*?^version\s*=\s*"([^"]*)"/m.exec(cargoToml)
  const lock = /^name = "ledge"\r?\nversion = "([^"]*)"/m.exec(cargoLock)
  return { 'tauri.conf.json': json(tauriConf), 'package.json': json(packageJson), 'Cargo.toml': pkg ? pkg[1] : null, 'Cargo.lock': lock ? lock[1] : null }
}

/** L217: všechna místa se zástupnou verzí musí být nalezená a shodná. */
export function versionsAgree(map) {
  const vals = Object.values(map)
  return vals.every((v) => typeof v === 'string' && v) && new Set(vals).size === 1
}

/** L217: je instalátor starší než nainstalovaná verze? (jen varování, neblokuje) */
export function isDowngrade(installed, candidate) {
  const p = (s) => (/^\d+\.\d+\.\d+/.exec(String(s ?? '').trim()) ?? [''])[0].split('.').map(Number)
  const a = p(installed)
  const b = p(candidate)
  if (a.length !== 3 || b.length !== 3) return false
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] > b[i]
  return false
}

/** Verze z názvu instalátoru `Ledge_0.16.0_x64-setup.exe`. */
export function versionFromSetupName(name) {
  const m = /_(\d+\.\d+\.\d+)_/.exec(name)
  return m ? m[1] : null
}
