// Nimmt ein Fenster der laufenden Anwendung auf und legt es unter
// docs/screenshots/ ab.
//
//   npm run shot hauptfenster
//   npm run shot overlay -- --window Gestenerkennung
//
// Aufgenommen wird gezielt **ein Fenster**, nicht ein Bildschirmausschnitt:
// So landet garantiert nichts anderes im Bild. Nur macOS.

import { execFileSync } from 'node:child_process'
import { mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const args = process.argv.slice(2)
const name = args.find((value) => !value.startsWith('--')) ?? 'hauptfenster'
const titleIndex = args.indexOf('--window')
const title = titleIndex >= 0 ? args[titleIndex + 1] : 'Gesture TimeTrack'
/** Breite im Bild - genug für eine README, klein genug fürs Repository. */
const WIDTH = 1600

if (process.platform !== 'darwin') {
  console.error('Dieses Skript nutzt macOS-Werkzeuge (screencapture, swift).')
  process.exit(1)
}

let windowId
try {
  windowId = execFileSync('swift', [join(root, 'scripts', 'window-id.swift'), title], {
    encoding: 'utf8',
  }).trim()
} catch {
  console.error(`Kein Fenster „${title}" gefunden. Läuft die Anwendung und ist das Fenster offen?`)
  process.exit(2)
}

const target = join(root, 'docs', 'screenshots', `${name}.png`)
mkdirSync(dirname(target), { recursive: true })

// -o lässt den Fensterschatten weg, -x den Auslöseton.
execFileSync('screencapture', ['-x', '-o', '-l', windowId, target])
execFileSync('sips', ['-Z', String(WIDTH), target], { stdio: 'ignore' })

console.log(`[shot] ${target}`)
