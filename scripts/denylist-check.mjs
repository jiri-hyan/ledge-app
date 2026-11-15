// Kontrola zakázaných slov (scripts/private/denylist.txt) před vydáním do veřejného repa — jediná implementace;
// publish.ps1 ji volá nad rozbaleným klonem (--dir), brána `denylist` v brany.mjs nad pracovním stromem (--tree).
// Výstup NIKDY nevypisuje vzor ani nalezený text (ať se slovo nedostane do logů a reportů): jen `soubor:řádek ~ vzor #<index>`,
// kde index je pořadí vzoru mezi platnými řádky denylistu (0 = první); shoda v samotném jménu souboru je ve výstupu zamaskovaná `***`.
//
//   node scripts/denylist-check.mjs                    → obsah i jména souborů stromu HEAD (kromě export-ignore)
//   node scripts/denylist-check.mjs --tree             → totéž nad pracovním stromem (sledované + nové neignorované soubory)
//   node scripts/denylist-check.mjs --dir <složka>     → rozbalený klon (to, co jde ven; vše kromě .git)
//   node scripts/denylist-check.mjs --since <tag|auto> → navíc předměty commitů tag..HEAD (mimo scripts/private a .github),
//                                                        bez commitů z scripts/private/notes-skip.txt; auto = poslední tag v*
//   node scripts/denylist-check.mjs --exe <soubor>     → sestavená binárka: řetězce ASCII i UTF-16LE (po blocích, ne celý soubor v paměti) proti denylistu
//                                                        + obecný vzor cesty `X:\Users\<jméno>\` (jméno mimo veřejné atrapy); výstup jen offset, kódování a vzor.
//                                                        Bez denylistu běží jen obecný vzor. Tohle volá publish.ps1 nad exe před nahráním; do bran nepatří (exe tam není).
//   --denylist <soubor> · --root <repo>               → jiný denylist / kořen repa (testy)
// Exit 0 čisto (nebo bez denylistu → „přeskočeno“, např. veřejný klon), 1 zásahy, 2 chyba (vadný regulární výraz, git).
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// Přípony, jejichž obsah se nečte (binární obrázky); jméno se kontroluje vždy.
// Známé omezení: obsah těchto typů se neprohledává (a GIF/WebM/MP4/WOFF se čtou jako text, takže zachytí jen nekomprimované
// ASCII řetězce); jména souborů se kontrolují u KAŽDÉHO typu. Metadata obrázků/videa tedy ověř očima / exiftool.
export const BINARY_EXT = ['.png', '.ico', '.icns', '.jpg', '.webp', '.bmp']

/** Text denylistu → pole { index, re } (řádky = regulární výrazy bez ohledu na velikost písmen, `#` = komentář). Vadný výraz hodí chybu s indexem. */
export function parsePatterns(text) {
  const out = []
  for (const raw of text.split(/\r?\n/)) {
    const l = raw.trim()
    if (!l || l.startsWith('#')) continue
    const index = out.length
    try { out.push({ index, re: new RegExp(l, 'i') }) } catch { throw new Error(`vzor #${index} není platný regulární výraz`) }
  }
  return out
}

/** notes-skip.txt → množina plných SHA (řádek `<sha> # komentář`). */
export function parseSkip(text) {
  return new Set(text.split(/\r?\n/).map((l) => l.split('#')[0].trim()).filter(Boolean))
}

const mask = (s, re) => s.replace(new RegExp(re.source, 'ig'), '***')
const isBinaryName = (rel) => BINARY_EXT.includes(path.extname(rel).toLowerCase())

/** Zásahy v jednom souboru: jméno (každý vzor) + obsah (první řádek každého vzoru). → [{ where, idx }] */
export function scanFile(rel, content, patterns) {
  const hits = []
  for (const p of patterns) {
    if (p.re.test(rel)) hits.push({ where: `${patterns.reduce((s, q) => mask(s, q.re), rel)} (jméno souboru)`, idx: p.index })
    if (content == null || isBinaryName(rel)) continue
    const lines = content.split('\n')
    for (let i = 0; i < lines.length; i++) {
      if (p.re.test(lines[i])) { hits.push({ where: `${patterns.reduce((s, q) => mask(s, q.re), rel)}:${i + 1}`, idx: p.index }); break }
    }
  }
  return hits
}

/** Předměty commitů ({sha, subject}) proti vzorům, bez SHA ze skip množiny. */
export function scanSubjects(commits, patterns, skip = new Set()) {
  const hits = []
  for (const c of commits) {
    if (skip.has(c.sha)) continue
    for (const p of patterns) if (p.re.test(c.subject)) hits.push({ where: `commit ${c.sha.slice(0, 8)} (předmět)`, idx: p.index })
  }
  return hits
}

export const formatHit = (h) => `ZAKÁZÁNO: ${h.where} ~ vzor #${h.idx}`

/** Souhrn podle prvního dílu cesty: { docs: 2, src: 1, … } (commity = „commity“). */
export function summarize(hits) {
  const by = {}
  for (const h of hits) {
    const k = h.where.startsWith('commit ') ? 'commity' : h.where.split(/[/:( ]/)[0] || '?'
    by[k] = (by[k] ?? 0) + 1
  }
  return by
}

const git = (root, args, input) => execFileSync('git', ['-c', 'core.quotepath=off', ...args], { cwd: root, input, maxBuffer: 1 << 28, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] })

/** Cesty s atributem export-ignore — soubor samotný, nebo některá jeho složka (`check-attr` na cestě uvnitř složky vzor `dir/` nepozná, proto se ptá na každou složku zvlášť). */
function exportIgnored(root, files) {
  if (!files.length) return new Set()
  const dirs = new Set()
  for (const n of files) { const p = n.split('/'); for (let i = 1; i < p.length; i++) dirs.add(p.slice(0, i).join('/') + '/') }
  const ask = (list) => {
    const out = git(root, ['check-attr', 'export-ignore', '--stdin', '-z'], list.join('\0') + '\0').toString('utf8').split('\0')
    const set = new Set()
    for (let i = 0; i + 2 < out.length; i += 3) if (out[i + 2] === 'set') set.add(out[i])
    return set
  }
  const ignDirs = ask([...dirs])
  const ignFiles = ask(files)
  return new Set(files.filter((n) => ignFiles.has(n) || [...ignDirs].some((d) => n.startsWith(d))))
}

/** Soubory k prohledání: [{ rel, read() }]. Šablony `.github/verejne-repo/*` jdou ven pod `.github/` (jako v publish.ps1). */
export function collect(root, mode, dir) {
  const files = []
  if (mode === 'dir') {
    const walk = (d) => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        if (e.name === '.git') continue
        const p = path.join(d, e.name)
        if (e.isDirectory()) walk(p)
        else files.push({ rel: path.relative(dir, p).split(path.sep).join('/'), read: () => fs.readFileSync(p, 'utf8') })
      }
    }
    walk(dir)
    return files
  }
  const listing = mode === 'tree'
    ? git(root, ['ls-files', '-z', '--cached', '--others', '--exclude-standard'])
    : git(root, ['ls-tree', '-r', '-z', '--name-only', 'HEAD'])
  const names = listing.toString('utf8').split('\0').filter(Boolean)
  const uniq = [...new Set(names)].filter((n) => mode !== 'tree' || fs.existsSync(path.join(root, n)))
  const ign = exportIgnored(root, uniq)
  const tpl = '.github/verejne-repo/'
  let blobs = null
  if (mode === 'head') { // jedno `cat-file --batch` místo procesu na soubor
    const keep = uniq.filter((n) => !isBinaryName(n))
    const buf = git(root, ['cat-file', '--batch'], keep.map((n) => `HEAD:${n}`).join('\n') + '\n')
    blobs = new Map()
    let pos = 0
    for (const n of keep) {
      const nl = buf.indexOf(10, pos)
      const len = Number(buf.subarray(pos, nl).toString('utf8').split(' ')[2])
      blobs.set(n, buf.subarray(nl + 1, nl + 1 + len).toString('utf8'))
      pos = nl + 1 + len + 1
    }
  }
  const read = (n) => (blobs ? blobs.get(n) ?? null : isBinaryName(n) ? null : fs.readFileSync(path.join(root, n), 'utf8'))
  for (const n of uniq) {
    if (n.startsWith(tpl)) files.push({ rel: `.github/${n.slice(tpl.length)}`, read: () => read(n) })
    else if (!ign.has(n)) files.push({ rel: n, read: () => read(n) })
  }
  return files
}

/** Commity tag..HEAD (jen cesty, které jdou ven): [{ sha, subject }]. `tag` null = nic; tag, který lokálně není, = posledních 30 commitů (jako publish.ps1 dřív). */
export function commitsSince(root, tag) {
  if (!tag) return []
  let range = [`${tag}..HEAD`]
  try { git(root, ['rev-parse', '-q', '--verify', `refs/tags/${tag}`]) } catch { range = ['-n', '30'] }
  const out = git(root, ['log', '--no-decorate', '--format=%H %s', ...range, '--', '.', ':(exclude)scripts/private', ':(exclude).github']).toString('utf8')
  return out.split(/\r?\n/).filter(Boolean).map((l) => ({ sha: l.slice(0, 40), subject: l.slice(41) }))
}

// ---- binárka (--exe) ----
const EXE_BLOCK = 4 << 20 // velikost bloku
const EXE_OVERLAP = 1 << 12 // přesah, aby nález na hranici bloků nezmizel (a nebyl dvakrát)
// Obecný vzor: cesta k domovské složce uživatele. Veřejné atrapy a zástupné tvary (%USERNAME%, <uživatel>…) se nehlásí.
export const USERS_PATH_RE = /[A-Z]:[\\/]Users[\\/]([^\\/\s\0"'<>|*?:]{1,64})[\\/]/gi
export const SAFE_USERS = new Set(['demo', 'public', 'default', 'default user', 'all users', 'example', 'user', 'username'])
export const userPathIsSafe = (name) => SAFE_USERS.has(name.toLowerCase()) || /^[%$<{[]/.test(name)

/** Zásahy v jednom bloku bajtů: [{ offset, enc, name, idx? }] (offset = base + pozice v bloku; `limit` = hlásí se jen nálezy s offsetem < limit, zbytek patří dalšímu bloku). */
export function scanBlock(buf, base, patterns, limit = Infinity) {
  const hits = []
  const encs = [['ascii', 0], ['utf16le', 0], ['utf16le', 1]]
  for (const [enc, shift] of encs) {
    const view = shift ? buf.subarray(shift) : buf
    const text = enc === 'ascii' ? view.toString('latin1') : view.subarray(0, view.length & ~1).toString('utf16le')
    const at = (i) => base + shift + (enc === 'ascii' ? i : i * 2)
    for (const p of patterns) {
      const re = new RegExp(p.re.source, 'ig')
      for (let m; (m = re.exec(text)); ) { if (m[0] === '') { re.lastIndex++; continue } const o = at(m.index); if (o < limit) hits.push({ offset: o, enc, name: `vzor #${p.index}` }) }
    }
    USERS_PATH_RE.lastIndex = 0
    for (let m; (m = USERS_PATH_RE.exec(text)); ) {
      const o = at(m.index)
      if (o < limit && !userPathIsSafe(m[1])) hits.push({ offset: o, enc, name: 'obecný: cesta uživatele' })
    }
  }
  return hits
}

/** Binárku projde po blocích; zásahy seřazené podle offsetu. */
export function scanExe(file, patterns) {
  const fd = fs.openSync(file, 'r')
  const hits = []
  try {
    const size = fs.fstatSync(fd).size
    const buf = Buffer.alloc(EXE_BLOCK + EXE_OVERLAP)
    for (let pos = 0; pos < size; pos += EXE_BLOCK) {
      const n = fs.readSync(fd, buf, 0, buf.length, pos)
      hits.push(...scanBlock(buf.subarray(0, n), pos, patterns, pos + EXE_BLOCK))
    }
  } finally { fs.closeSync(fd) }
  return hits.sort((a, b) => a.offset - b.offset)
}

/** Souhrn zásahů v binárce: { 'ascii · vzor #0': 3, … } */
export function summarizeExe(hits) {
  const by = {}
  for (const h of hits) { const k = `${h.enc} · ${h.name}`; by[k] = (by[k] ?? 0) + 1 }
  return by
}

function mainExe(file, denyFile) {
  if (!fs.existsSync(file)) { console.error(`denylist: soubor ${file} neexistuje`); return 2 }
  let patterns = []
  try { if (fs.existsSync(denyFile)) patterns = parsePatterns(fs.readFileSync(denyFile, 'utf8')) } catch (e) { console.error(`denylist: ${e.message}`); return 2 }
  const hits = scanExe(file, patterns)
  console.log(`denylist: exe ${path.basename(file)}, ${patterns.length} vzorů + obecný vzor cesty uživatele (ASCII + UTF-16LE)`)
  for (const h of hits.slice(0, 50)) console.log(`ZAKÁZÁNO: exe @${h.offset} (${h.enc}) ~ ${h.name}`)
  if (hits.length > 50) console.log(`… a dalších ${hits.length - 50}`)
  if (hits.length) console.log(`ZÁSAHY: ${hits.length} (${Object.entries(summarizeExe(hits)).map(([k, v]) => `${k}: ${v}`).join(', ')})`)
  return hits.length ? 1 : 0
}

function main(argv) {
  const opt = (n) => { const i = argv.indexOf(n); return i >= 0 ? argv[i + 1] : null }
  const root = path.resolve(opt('--root') ?? path.join(path.dirname(fileURLToPath(import.meta.url)), '..'))
  const denyFile = opt('--denylist') ?? path.join(root, 'scripts', 'private', 'denylist.txt')
  const exe = opt('--exe')
  if (exe) return mainExe(path.resolve(exe), denyFile)
  if (!fs.existsSync(denyFile)) { console.log('denylist: bez scripts/private/denylist.txt — přeskočeno'); return 0 }
  let patterns
  try { patterns = parsePatterns(fs.readFileSync(denyFile, 'utf8')) } catch (e) { console.error(`denylist: ${e.message}`); return 2 }
  const dir = opt('--dir')
  const mode = dir ? 'dir' : argv.includes('--tree') ? 'tree' : 'head'
  let hits = []
  try {
    const files = collect(root, mode, dir ? path.resolve(dir) : null)
    for (const f of files) hits.push(...scanFile(f.rel, f.read(), patterns))
    let since = opt('--since')
    if (since === 'auto') { try { since = git(root, ['describe', '--tags', '--abbrev=0', '--match', 'v*']).toString('utf8').trim() } catch { since = null } }
    if (since) {
      const skipFile = path.join(root, 'scripts', 'private', 'notes-skip.txt')
      hits.push(...scanSubjects(commitsSince(root, since), patterns, fs.existsSync(skipFile) ? parseSkip(fs.readFileSync(skipFile, 'utf8')) : new Set()))
    }
    hits = hits.filter((h, i) => hits.findIndex((x) => x.where === h.where && x.idx === h.idx) === i)
    console.log(`denylist: ${patterns.length} vzorů, ${files.length} souborů (${mode})`)
  } catch (e) { console.error(`denylist: chyba — ${String(e.message).split(/\r?\n/)[0]}`); return 2 }
  for (const h of hits) console.log(formatHit(h))
  if (hits.length) console.log(`ZÁSAHY: ${hits.length} (${Object.entries(summarize(hits)).map(([k, v]) => `${k}: ${v}`).join(', ')})`)
  return hits.length ? 1 : 0
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) process.exit(main(process.argv.slice(2)))
