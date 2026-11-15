// Remap cest v release buildu — čisté funkce tabulkově (vymyšlené cesty, nic ze skutečného prostředí).
//   node --experimental-strip-types --test scripts/remap-flags.test.mjs
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { remapPairs, withRemap } from './remap-flags.mjs'

const dirs = { home: String.raw`C:\Users\demo`, cargoHome: String.raw`C:\Users\demo\.cargo`, rustupHome: String.raw`C:\Users\demo\.rustup`, targetDir: String.raw`C:\Users\demo\w\notch\src-tauri\target`, root: 'C:/Users/demo/w/notch/' }

test('remapPairs: obě podoby lomítek, obecné předpony před konkrétními, bez koncového lomítka, bez duplicit', () => {
  const p = remapPairs(dirs)
  assert.deepEqual(p.map((x) => x[1]), ['/home', '/home', '/rustup', '/rustup', '/cargo', '/cargo', '/target', '/target', '/ledge'])  // root zadaný jen s „/“ = jedna podoba
  assert.equal(p[1][0], 'C:/Users/demo')
  assert.equal(p.at(-1)[0], 'C:/Users/demo/w/notch')
  assert.deepEqual(remapPairs({ home: '/x', cargoHome: '/x/c' }).map((x) => x[1]), ['/home', '/cargo'], 'chybějící složky se vynechají')
})

test('withRemap: RUSTFLAGS se doplní za stávající; encoded má přednost; mezera v cestě → encoded', () => {
  const pairs = [['C:/a', '/a']]
  assert.equal(withRemap({}, pairs).RUSTFLAGS, '--remap-path-prefix=C:/a=/a')
  assert.equal(withRemap({ RUSTFLAGS: '-C target-cpu=native' }, pairs).RUSTFLAGS, '-C target-cpu=native --remap-path-prefix=C:/a=/a')
  assert.equal(withRemap({ CARGO_ENCODED_RUSTFLAGS: '-Cx' }, pairs).CARGO_ENCODED_RUSTFLAGS, '-Cx\x1f--remap-path-prefix=C:/a=/a')
  const sp = withRemap({ RUSTFLAGS: '-Cy' }, [['C:/a b', '/a']])
  assert.equal(sp.CARGO_ENCODED_RUSTFLAGS, '-Cy\x1f--remap-path-prefix=C:/a b=/a')
  assert.equal(sp.RUSTFLAGS, '-Cy', 'původní RUSTFLAGS zůstává (cargo ho při encoded ignoruje)')
})
