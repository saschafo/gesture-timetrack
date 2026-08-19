// Holt alle Assets, die die Gestenerkennung zur Laufzeit braucht, EINMALIG
// ins Repo bzw. in den Bundle-Ordner. Zur Laufzeit ist damit kein Netzwerk-
// zugriff mehr nötig - das ist die Grundlage des Offline-Versprechens.
//
//   node scripts/fetch-assets.mjs
//
// Läuft automatisch als postinstall-Hook.

import { createWriteStream } from 'node:fs'
import { cp, mkdir, stat } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

const MODEL_URL =
  'https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/1/hand_landmarker.task'
const MODEL_PATH = join(root, 'public', 'models', 'hand_landmarker.task')
const WASM_SRC = join(root, 'node_modules', '@mediapipe', 'tasks-vision', 'wasm')
const WASM_DEST = join(root, 'public', 'mediapipe', 'wasm')

async function exists(path) {
  try {
    const s = await stat(path)
    return s.isDirectory() || s.size > 0
  } catch {
    return false
  }
}

async function fetchModel() {
  if (await exists(MODEL_PATH)) {
    console.log('[assets] Modell bereits vorhanden, überspringe Download.')
    return
  }
  console.log('[assets] Lade MediaPipe-Handmodell (~7,5 MB) ...')
  const res = await fetch(MODEL_URL)
  if (!res.ok) throw new Error(`Download fehlgeschlagen: HTTP ${res.status}`)
  await mkdir(dirname(MODEL_PATH), { recursive: true })
  await pipeline(Readable.fromWeb(res.body), createWriteStream(MODEL_PATH))
  console.log('[assets] Modell gespeichert:', MODEL_PATH)
}

async function copyWasm() {
  if (!(await exists(WASM_SRC))) {
    console.warn('[assets] WASM-Quelle fehlt - erst "npm install" ausführen.')
    return
  }
  await mkdir(dirname(WASM_DEST), { recursive: true })
  await cp(WASM_SRC, WASM_DEST, { recursive: true })
  console.log('[assets] WASM-Laufzeit kopiert:', WASM_DEST)
}

await fetchModel()
await copyWasm()
