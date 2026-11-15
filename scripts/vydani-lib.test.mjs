// Pojistky vydání — tabulkově.
//   node --experimental-strip-types --test scripts/vydani-lib.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import { gateCheck, notesCheck, extractVersions, versionsAgree, isDowngrade, versionFromSetupName } from './vydani-lib.mjs'

test('gateCheck: razítko vs strom HEAD', () => {
  const T = 'a'.repeat(40)
  for (const [stamp, tree, ok] of [
    [`${T}\n2026-11-13T10:00:00Z\n`, T, true],
    [`${T}\r\nx\r\n`, T, true],
    [`${'b'.repeat(40)}\nx\n`, T, false],
    [null, T, false],
    ['', T, false],
    [`${T}\nx\n`, null, false],
  ]) assert.equal(gateCheck(stamp, tree).ok, ok, JSON.stringify([stamp, tree]))
})

test('notesCheck: bez souboru, prázdný i špatná verze = Fail', () => {
  for (const [v, t, ok] of [['0.16.0', '- **Česky / English:** x', true], ['0.16.0', null, false], ['0.16.0', '  \n', false], ['0.16', 'x', false], ['../x', 'x', false]])
    assert.equal(notesCheck(v, t).ok, ok, `${v} ${t}`)
})

test('isDowngrade / versionFromSetupName', () => {
  for (const [i, c, d] of [['0.15.2', '0.2.0', true], ['0.15.2', '0.16.0', false], ['0.15.2', '0.15.2', false], ['0.2.0', '0.10.0', false], [null, '0.2.0', false], ['0.15.2', null, false]])
    assert.equal(isDowngrade(i, c), d, `${i} ${c}`)
  assert.equal(versionFromSetupName('Ledge_0.16.0_x64-setup.exe'), '0.16.0')
  assert.equal(versionFromSetupName('setup.exe'), null)
})

test('extractVersions + versionsAgree: syntetické vstupy', () => {
  const src = {
    tauriConf: '{"version":"1.2.3"}',
    packageJson: '{"name":"x","version":"1.2.3"}',
    cargoToml: '[package]\nname="ledge"\nversion = "1.2.3"\n[dependencies]\nfoo = { version = "9" }',
    cargoLock: '[[package]]\nname = "other"\nversion = "9.9.9"\n\n[[package]]\nname = "ledge"\nversion = "1.2.3"\n',
  }
  assert.equal(versionsAgree(extractVersions(src)), true)
  assert.equal(versionsAgree(extractVersions({ ...src, cargoLock: src.cargoLock.replace('"1.2.3"', '"1.2.4"') })), false)
  assert.equal(versionsAgree(extractVersions({ ...src, cargoLock: '' })), false)
})

test('zástupná verze je shodná na všech 4 místech repa', () => {
  const rd = (p) => fs.readFileSync(new URL(`../${p}`, import.meta.url), 'utf8')
  const m = extractVersions({ tauriConf: rd('src-tauri/tauri.conf.json'), packageJson: rd('package.json'), cargoToml: rd('src-tauri/Cargo.toml'), cargoLock: rd('src-tauri/Cargo.lock') })
  assert.equal(versionsAgree(m), true, JSON.stringify(m))
})
