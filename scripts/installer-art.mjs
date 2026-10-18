// Obrázky NSIS instalátoru: src-tauri/installer/{sidebar,header}.svg → .bmp (24 bit, bez alfy, bez RLE).
// NSIS (Modern UI 2) bere jen BMP; alfa kanál by ukázal jako černou. Žádný prohlížeč ani balíček navíc:
// SVG podklady (náhled A, design-mockups/ledge-instalacka) jsou jen plné barvy a jednoduché tvary, takže
// je vykreslí malý rasterizér níž — `rect` (i se zaoblením `rx` a obrysem `stroke`), `circle`, `g` s
// děděnou výplní a `shape-rendering="crispEdges"`. Cokoli jiného v SVG = chyba (raději spadnout než tiše
// vynechat tvar). Vykresluje se přesně v rozměru SVG (1×), hrany mimo mřížku vyhlazuje 8×8 vzorků na pixel.
//
// Použití: node scripts/installer-art.mjs          → přepíše .bmp vedle .svg
//          node scripts/installer-art.mjs --check  → jen ověří, že commitnuté .bmp odpovídají .svg (exit 1 = ne)
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const dir = path.join(root, 'src-tauri', 'installer')
/** [soubor, šířka, výška] — rozměry, které NSIS čeká (neškáluje je) */
const ART = [
  ['sidebar', 164, 314],
  ['header', 150, 57]
]
const SS = 8

const attrs = (s) => Object.fromEntries([...s.matchAll(/([\w:-]+)="([^"]*)"/g)].map((m) => [m[1], m[2]]))
const num = (a, k, d = 0) => (a[k] === undefined ? d : Number(a[k]))

function hex(c) {
  const m = /^#([0-9a-f]{6})$/i.exec(c ?? '')
  if (!m) throw new Error(`barva „${c}“: čekám #rrggbb`)
  const v = parseInt(m[1], 16)
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255]
}

/** Uvnitř zaobleného obdélníku? (souřadnice v pixelech SVG) */
function inRoundRect(px, py, x, y, w, h, r) {
  if (w <= 0 || h <= 0 || px < x || py < y || px >= x + w || py >= y + h) return false
  r = Math.max(0, Math.min(r, w / 2, h / 2))
  if (r === 0) return true
  const cx = px < x + r ? x + r : px > x + w - r ? x + w - r : px
  const cy = py < y + r ? y + r : py > y + h - r ? y + h - r : py
  return (px - cx) ** 2 + (py - cy) ** 2 <= r * r
}

/** SVG → seznam tvarů {test(px,py), color, crisp, box} v pořadí kreslení. */
export function parseSvg(text) {
  const svg = /<svg\b([^>]*)>/.exec(text)
  if (!svg) throw new Error('není <svg>')
  const sa = attrs(svg[1])
  const shapes = []
  const stack = [{ fill: undefined, crisp: false }]
  for (const m of text.matchAll(/<(\/?)(\w+)\b([^>]*?)(\/?)>/g)) {
    const [, close, tag, rest, self] = m
    const a = attrs(rest)
    const ctx = stack[stack.length - 1]
    if (tag === 'svg') continue
    if (tag === 'g') {
      if (close) stack.pop()
      else if (!self) stack.push({ fill: a.fill ?? ctx.fill, crisp: a['shape-rendering'] === 'crispEdges' || ctx.crisp })
      continue
    }
    const crisp = a['shape-rendering'] === 'crispEdges' || ctx.crisp
    const fill = a.fill ?? ctx.fill
    if (tag === 'rect') {
      const [x, y, w, h] = [num(a, 'x'), num(a, 'y'), num(a, 'width'), num(a, 'height')]
      const r = num(a, 'rx', num(a, 'ry'))
      if (fill && fill !== 'none') shapes.push({ color: hex(fill), crisp, test: (px, py) => inRoundRect(px, py, x, y, w, h, r) })
      if (a.stroke && a.stroke !== 'none') {
        const s = num(a, 'stroke-width', 1) / 2
        shapes.push({
          color: hex(a.stroke),
          crisp,
          test: (px, py) => inRoundRect(px, py, x - s, y - s, w + 2 * s, h + 2 * s, r + s) && !inRoundRect(px, py, x + s, y + s, w - 2 * s, h - 2 * s, Math.max(0, r - s))
        })
      }
    } else if (tag === 'circle') {
      const [cx, cy, r] = [num(a, 'cx'), num(a, 'cy'), num(a, 'r')]
      if (a.stroke && a.stroke !== 'none') throw new Error('circle se stroke rasterizér neumí')
      shapes.push({ color: hex(fill), crisp, test: (px, py) => (px - cx) ** 2 + (py - cy) ** 2 <= r * r })
    } else {
      throw new Error(`prvek <${tag}> rasterizér neumí — zjednoduš SVG nebo doplň skript`)
    }
  }
  return { width: num(sa, 'width'), height: num(sa, 'height'), shapes }
}

/** Vykreslí tvary do RGB pole (řádky shora dolů). */
export function rasterize({ width, height, shapes }) {
  const px = new Float64Array(width * height * 3)
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const o = (y * width + x) * 3
      for (const s of shapes) {
        let cov
        if (s.crisp) cov = s.test(x + 0.5, y + 0.5) ? 1 : 0
        else {
          let hit = 0
          for (let j = 0; j < SS; j++) for (let i = 0; i < SS; i++) if (s.test(x + (i + 0.5) / SS, y + (j + 0.5) / SS)) hit++
          cov = hit / (SS * SS)
        }
        if (cov === 0) continue
        for (let c = 0; c < 3; c++) px[o + c] = px[o + c] * (1 - cov) + s.color[c] * cov
      }
    }
  }
  return Uint8Array.from(px, (v) => Math.round(v))
}

/** RGB (shora dolů) → BMP: BITMAPINFOHEADER, 24 bit, BI_RGB (bez komprese), řádky zdola, zarovnání na 4 B. */
export function bmp(width, height, rgb) {
  const stride = Math.ceil((width * 3) / 4) * 4
  const size = 54 + stride * height
  const b = Buffer.alloc(size)
  b.write('BM', 0, 'ascii')
  b.writeUInt32LE(size, 2)
  b.writeUInt32LE(54, 10)
  b.writeUInt32LE(40, 14)
  b.writeInt32LE(width, 18)
  b.writeInt32LE(height, 22) // kladná výška = řádky zdola nahoru (nejširší podpora)
  b.writeUInt16LE(1, 26)
  b.writeUInt16LE(24, 28)
  b.writeUInt32LE(0, 30) // BI_RGB
  b.writeUInt32LE(stride * height, 34)
  b.writeInt32LE(2835, 38) // 72 DPI
  b.writeInt32LE(2835, 42)
  for (let y = 0; y < height; y++) {
    const row = 54 + (height - 1 - y) * stride
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 3
      b[row + x * 3] = rgb[i + 2]
      b[row + x * 3 + 1] = rgb[i + 1]
      b[row + x * 3 + 2] = rgb[i]
    }
  }
  return b
}

function main() {
  const check = process.argv.includes('--check')
  let bad = 0
  for (const [name, w, h] of ART) {
    const svg = parseSvg(fs.readFileSync(path.join(dir, `${name}.svg`), 'utf8'))
    if (svg.width !== w || svg.height !== h) throw new Error(`${name}.svg má ${svg.width}×${svg.height}, NSIS čeká ${w}×${h}`)
    const out = bmp(w, h, rasterize(svg))
    const file = path.join(dir, `${name}.bmp`)
    if (check) {
      const same = fs.existsSync(file) && fs.readFileSync(file).equals(out)
      if (!same) bad++
      console.log(`${same ? 'ok  ' : 'JINÉ'} ${path.relative(root, file)} (${w}×${h}, ${out.length} B)`)
    } else {
      fs.writeFileSync(file, out)
      console.log(`zapsáno ${path.relative(root, file)} (${w}×${h}, 24 bit, ${out.length} B)`)
    }
  }
  if (bad) {
    console.error('BMP neodpovídá SVG — spusť node scripts/installer-art.mjs a commitni výsledek')
    process.exit(1)
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main()
