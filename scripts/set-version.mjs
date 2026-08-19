// Setzt die Versionsnummer an allen drei Stellen, an denen sie stehen muss:
// package.json, src-tauri/tauri.conf.json und src-tauri/Cargo.toml.
//
//   node scripts/set-version.mjs 0.2.0
//
// Getrennte Dateien, eine Wahrheit: Laufen die Nummern auseinander, trägt der
// Installer eine andere Version als die Anwendung meldet - und ein Update
// erkennt womöglich gar nicht, dass es eines ist.

import { readFile, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const version = process.argv[2]

if (!/^\d+\.\d+\.\d+$/.test(version ?? '')) {
  console.error('Bitte eine Version im Format 1.2.3 angeben, z. B.:')
  console.error('  node scripts/set-version.mjs 0.2.0')
  process.exit(1)
}

async function patchJson(relative, apply) {
  const path = join(root, relative)
  const data = JSON.parse(await readFile(path, 'utf8'))
  apply(data)
  await writeFile(path, `${JSON.stringify(data, null, 2)}\n`)
  console.log(`[version] ${relative}`)
}

/** In Cargo.toml nur die Version des Pakets selbst ändern, nicht die der Abhängigkeiten. */
async function patchCargo() {
  const path = join(root, 'src-tauri', 'Cargo.toml')
  const text = await readFile(path, 'utf8')
  const pattern = /(\[package\][\s\S]*?\nversion = ")[^"]+(")/
  // Auf den Treffer prüfen, nicht auf eine Änderung: Bei gleicher Version wäre
  // der Text identisch, und das ist kein Fehler.
  if (!pattern.test(text)) throw new Error('Version in Cargo.toml nicht gefunden')
  await writeFile(path, text.replace(pattern, `$1${version}$2`))
  console.log('[version] src-tauri/Cargo.toml')
}

await patchJson('package.json', (data) => {
  data.version = version
})
await patchJson('src-tauri/tauri.conf.json', (data) => {
  data.version = version
})
await patchCargo()

console.log(`[version] auf ${version} gesetzt. Cargo.lock zieht beim nächsten Build nach.`)
