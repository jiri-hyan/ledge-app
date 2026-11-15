// CLI pro publish.ps1 / install-local.ps1 (logika je v vydani-lib.mjs, testovaná).
//   node scripts/vydani-check.mjs gates            → .brany-ok odpovídá stromu HEAD (exit 1 = ne)
//   node scripts/vydani-check.mjs notes <verze>    → existuje docs/releases/<verze>.md (exit 1 = ne)
//   node scripts/vydani-check.mjs downgrade <nainstalovaná> <instalátor.exe>   → varování na stdout, exit vždy 0
import fs from 'node:fs'
import path from 'node:path'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { gateCheck, notesCheck, isDowngrade, versionFromSetupName } from './vydani-lib.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const read = (p) => (fs.existsSync(p) ? fs.readFileSync(p, 'utf8') : null)
const [cmd, a, b] = process.argv.slice(2)
let r
if (cmd === 'gates') {
  let tree = null
  try { tree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { cwd: root, encoding: 'utf8', windowsHide: true }).trim() } catch { /* tree zůstane null */ }
  r = gateCheck(read(path.join(root, '.brany-ok')), tree)
} else if (cmd === 'notes') {
  r = notesCheck(a ?? '', read(path.join(root, 'docs', 'releases', `${a}.md`)))
} else if (cmd === 'downgrade') {
  const cand = versionFromSetupName(path.basename(b ?? ''))
  r = isDowngrade(a, cand)
    ? { ok: true, msg: `POZOR: instalátor ${cand} je starší než nainstalovaná ${a} — downgrade (zástupná verze 0.2.0 z lokálního buildu?); updater pak znovu nabídne ${a}` }
    : { ok: true, msg: '' }
} else { console.error('použití: vydani-check.mjs gates | notes <verze> | downgrade <nainstalovaná> <setup.exe>'); process.exit(2) }
if (r.msg) console.log(r.msg)
process.exit(r.ok ? 0 : 1)
