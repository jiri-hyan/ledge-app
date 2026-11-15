// Čisté pomocníky pro brany.mjs (tabulkový test: brany-lib.test.mjs).
//  - razítko `.brany-ok`: hash stromu po plném PASS; beze změny stromu se plné brány nespouštějí znovu
//  - opakování jen padlých (flaky) testů: jména z výstupu `cargo test` → příkaz `cargo test -- --exact …`

/** Jména padlých testů z výstupu `cargo test` (řádky `test a::b ... FAILED`), unikátně, v pořadí výskytu. */
export function parseFailedTests(text) {
  const out = []
  for (const m of text.matchAll(/^test (\S+) \.\.\. FAILED\s*$/gm)) if (!out.includes(m[1])) out.push(m[1])
  return out
}

/**
 * Smí se opakovat? Jen když padly pojmenované testy (≤ max), nic se nesestavovalo špatně (`error[`/`error:`)
 * a počet jmen sedí na počet „N failed“ — jinak by šlo o regresi, kterou nechceme maskovat.
 */
export function canRetry(text, names, max = 5) {
  if (!names.length || names.length > max) return false
  // `error: test failed, to rerun…` / `error: N targets failed` píše cargo i u obyčejně padlého testu; jiné `error` = chyba sestavení
  if (text.split(/\r?\n/).some((l) => /^error(\[|:)/.test(l) && !/^error: (test failed|\d+ targets? failed)/.test(l))) return false
  const failedTotal = [...text.matchAll(/(\d+) failed/g)].reduce((a, m) => a + Number(m[1]), 0)
  return failedTotal === names.length
}

/** Příkaz pro opakování jen padlých testů (jména jsou z `cargo test`, přesto jen bezpečné znaky). */
export function retryArgs(names) {
  const safe = names.filter((n) => /^[\w:]+$/.test(n))
  return safe.length === names.length ? ['--exact', ...safe] : null
}

/** Řádek výsledku brány `test` po opakování. `second` = true když prošlo na 2. pokus. */
export function flakyLine(names, second, secs) {
  return second
    ? `FLAKY test   prošlo až na 2. pokus (${secs} s) — NENÍ to čistý průchod: ${names.join(', ')}`
    : `FAIL  test   padlo i na 2. pokus (není flaky): ${names.join(', ')}`
}

/** Obsah razítka. */
export function stampText(treeHash, when = new Date()) {
  return `${treeHash}\n${when.toISOString()}\n`
}

/** Platí razítko pro tento strom? */
export function stampMatches(text, treeHash) {
  if (!text || !treeHash) return false
  return text.split(/\r?\n/)[0].trim() === treeHash
}

/** Plný běh = všech pět bran, žádný výběr ani --changed. */
export function isFullRun(names, all) {
  return names.length === all.length && all.every((n) => names.includes(n))
}

/** Brána `build` volá `tsc --noEmit && vite build`; pokud v tomtéž běhu běžela brána `tsc`, stačí `vite build`. */
export function buildCommand(names, queued) {
  const inner = names.includes('tsc') ? 'corepack pnpm exec vite build' : 'corepack pnpm build'
  return queued(inner)
}

/** `--since <ref>` / `--since=<ref>` z argumentů (null = není; '' = chybí hodnota) a `--allow-empty` (J3). */
export function parseSince(args) {
  const i = args.findIndex((a) => a === '--since' || a.startsWith('--since='))
  if (i < 0) return { since: null, allowEmpty: args.includes('--allow-empty') }
  const a = args[i]
  const val = a.includes('=') ? a.slice(a.indexOf('=') + 1) : args[i + 1] ?? ''
  return { since: val.startsWith('--') ? '' : val, allowEmpty: args.includes('--allow-empty') }
}

/** `--changed` bez jediného souboru nic neověřil → nesmí tvrdit „vše prošlo“ (J3). Vrací varování, nebo null. */
export function emptyChangedWarning(fileCount, allowEmpty) {
  if (fileCount > 0 || allowEmpty) return null
  return 'VAROVÁNÍ: --changed našel 0 změněných souborů — NIC SE NEOVĚŘILO. Použij --since <ref> (např. --since HEAD~1 po mergi), plný běh `node scripts/brany.mjs`, nebo --allow-empty, když je prázdná změna záměr.'
}

/** Linker „soubor je zamčený / používá jiný proces“ (LNK1104 apod.) — dva agenti linkují stejnou test binárku (J-linker). */
export function isLinkLockError(text) {
  return /LNK1104|LNK1201|error: linking with `link\.exe` failed[\s\S]{0,400}(cannot open file|being used by another process)|os error 32/.test(text)
}

/** Obsah zámku `.brany-cargo.lock` = `<pid>\n<čas ISO>\n<cwd>`; vrací pid, nebo null při nesmyslu. */
export function lockOwnerPid(text) {
  const n = Number(String(text).split(/\r?\n/)[0])
  return Number.isInteger(n) && n > 0 ? n : null
}

/** Zámek je opuštěný, když vlastník neběží, nebo je starší než maxMs (zabitý stroj / reuse pid). */
export function lockIsStale(text, alive, nowMs, maxMs = 45 * 60_000) {
  const pid = lockOwnerPid(text)
  if (pid === null) return true
  const when = Date.parse(String(text).split(/\r?\n/)[1] ?? '')
  if (Number.isFinite(when) && nowMs - when > maxMs) return true
  return !alive
}
