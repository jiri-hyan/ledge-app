// perf-lib.ps1: klasifikace procesů (jádro Ledge × rozšíření × cizí) nad zachycenou tabulkou procesů.
//   node --experimental-strip-types --test scripts/perf-lib.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import path from 'node:path'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const lib = path.join(path.dirname(fileURLToPath(import.meta.url)), 'perf-lib.ps1')
const ps = (script) =>
  execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-Command', `. '${lib}'; ${script}`], { encoding: 'utf8', windowsHide: true })

const W = 'msedgewebview2.exe'
const rows = [
  { ProcessId: 10, ParentProcessId: 1, Name: 'ledge.exe', CommandLine: '"C:\\L\\ledge.exe"', CreationDate: 100 },
  { ProcessId: 11, ParentProcessId: 10, Name: W, CommandLine: `${W} --embedded-browser-webview`, CreationDate: 101 },
  { ProcessId: 12, ParentProcessId: 11, Name: W, CommandLine: `${W} --type=renderer`, CreationDate: 102 },
  { ProcessId: 20, ParentProcessId: 10, Name: 'ledge.exe', CommandLine: '"C:\\L\\ledge.exe" --ext wall', CreationDate: 110 },
  { ProcessId: 21, ParentProcessId: 20, Name: W, CommandLine: `${W} --embedded-browser-webview`, CreationDate: 111 },
  { ProcessId: 22, ParentProcessId: 21, Name: W, CommandLine: `${W} --type=gpu-process`, CreationDate: 112 },
  { ProcessId: 30, ParentProcessId: 20, Name: 'ledge.exe', CommandLine: 'ledge.exe --ext=browser', CreationDate: 120 },
  // cizí: starší než „rodič“ (recyklované PID rodiče) a nesouvisející proces
  { ProcessId: 40, ParentProcessId: 10, Name: 'cizi.exe', CommandLine: 'cizi.exe', CreationDate: 50 },
  { ProcessId: 41, ParentProcessId: 999, Name: 'notepad.exe', CommandLine: 'notepad.exe', CreationDate: 200 },
]

function groups(rs) {
  const json = JSON.stringify(rs).replace(/'/g, "''")
  const out = ps(`$g = Get-LedgeGroups ('${json}' | ConvertFrom-Json); $g.Keys | Sort-Object | ForEach-Object { "$_=$($g[$_])" }`)
  return Object.fromEntries(out.trim().split(/\r?\n/).filter(Boolean).map((l) => l.split('=')))
}

test('Get-LedgeGroups: jádro bez ext, ext s celým stromem, cizí mimo', () => {
  assert.deepEqual(groups(rows), {
    10: 'core', 11: 'core', 12: 'core',
    20: 'ext:wall', 21: 'ext:wall', 22: 'ext:wall',
    30: 'ext:browser',
  })
})

test('Get-LedgeGroups: bez ledge.exe nic', () => {
  assert.deepEqual(groups(rows.filter((r) => r.Name !== 'ledge.exe')), {})
})

test('Get-ExtId: tabulka', () => {
  const t = [['ledge.exe --ext wall', 'wall'], ['ledge.exe --ext=wall --diag', 'wall'], ['"x\\ledge.exe" --ext "browser"', 'browser'], ['ledge.exe', ''], ['ledge.exe --extra x', ''], ['', '']]
  for (const [cmd, want] of t) {
    assert.equal(ps(`Get-ExtId '${cmd.replace(/'/g, "''")}'`).trim(), want, cmd)
  }
})
