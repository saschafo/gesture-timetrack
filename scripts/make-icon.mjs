// Erzeugt das Quell-Icon (1024x1024 PNG) ohne externe Bildbibliothek.
// Motiv: abgerundetes Quadrat, darauf eine stilisierte offene Hand (Start-Geste)
// mit Zifferblatt-Ring. Danach: npx tauri icon src-tauri/icons/source.png
import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const S = 1024
const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const px = new Uint8Array(S * S * 4)

const set = (x, y, [r, g, b], a = 255) => {
  const i = (y * S + x) * 4
  const inv = 1 - a / 255
  px[i] = px[i] * inv + r * (a / 255)
  px[i + 1] = px[i + 1] * inv + g * (a / 255)
  px[i + 2] = px[i + 2] * inv + b * (a / 255)
  px[i + 3] = Math.max(px[i + 3], a)
}

const INDIGO = [79, 70, 229]
const INDIGO_DARK = [55, 48, 163]
const WHITE = [255, 255, 255]

// Hintergrund: abgerundetes Quadrat mit vertikalem Verlauf
const radius = 200
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    const cx = Math.min(Math.max(x, radius), S - radius)
    const cy = Math.min(Math.max(y, radius), S - radius)
    const d = Math.hypot(x - cx, y - cy)
    if (d > radius) continue
    const t = y / S
    const color = INDIGO.map((c, i) => c + (INDIGO_DARK[i] - c) * t)
    set(x, y, color, d > radius - 1.5 ? 160 : 255)
  }
}

// Zifferblatt-Ring
const cx = S / 2
const cy = S / 2
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    const d = Math.hypot(x - cx, y - cy)
    if (d > 372 && d < 392) set(x, y, WHITE, 90)
  }
}

// Finger (vier Balken) + Daumen: offene Hand als Start-Geste
const bar = (x0, y0, w, h, r) => {
  for (let y = y0; y < y0 + h; y++) {
    for (let x = x0; x < x0 + w; x++) {
      const ix = Math.min(Math.max(x, x0 + r), x0 + w - r)
      const iy = Math.min(Math.max(y, y0 + r), y0 + h - r)
      if (Math.hypot(x - ix, y - iy) > r) continue
      set(x, y, WHITE)
    }
  }
}

const fingerTops = [300, 250, 262, 300]
fingerTops.forEach((top, i) => {
  bar(352 + i * 82, top, 58, 700 - top + 40, 29)
})
bar(268, 470, 58, 270, 29) // Daumen
bar(330, 640, 360, 160, 60) // Handfläche

// PNG schreiben (RGBA, keine Filter)
const raw = Buffer.alloc((S * 4 + 1) * S)
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0
  Buffer.from(px.buffer, y * S * 4, S * 4).copy(raw, y * (S * 4 + 1) + 1)
}
const chunk = (type, data) => {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data])
  const crcTable = chunk.table ?? (chunk.table = Array.from({ length: 256 }, (_, n) => {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    return c >>> 0
  }))
  let crc = 0xffffffff
  for (const byte of body) crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8)
  const crcBuf = Buffer.alloc(4)
  crcBuf.writeUInt32BE((crc ^ 0xffffffff) >>> 0)
  return Buffer.concat([len, body, crcBuf])
}
const ihdr = Buffer.alloc(13)
ihdr.writeUInt32BE(S, 0)
ihdr.writeUInt32BE(S, 4)
ihdr[8] = 8
ihdr[9] = 6
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
])
const out = join(root, 'src-tauri', 'icons', 'source.png')
mkdirSync(dirname(out), { recursive: true })
writeFileSync(out, png)
console.log('[icon] geschrieben:', out)
