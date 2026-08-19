// Trägt das GitHub-Repository an den Stellen ein, an denen es gebraucht wird:
// bislang die Update-Adresse in src-tauri/tauri.conf.json.
//
//   node scripts/set-repo.mjs konto/gesture-timetrack

import { readFile, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const repo = process.argv[2]

if (!/^[\w.-]+\/[\w.-]+$/.test(repo ?? '')) {
  console.error('Bitte Konto und Repository angeben, z. B.:')
  console.error('  node scripts/set-repo.mjs sascha/gesture-timetrack')
  process.exit(1)
}

const path = join(root, 'src-tauri', 'tauri.conf.json')
const config = JSON.parse(await readFile(path, 'utf8'))

config.plugins.updater.endpoints = [
  `https://github.com/${repo}/releases/latest/download/latest.json`,
]

await writeFile(path, `${JSON.stringify(config, null, 2)}\n`)
console.log(`[repo] Update-Adresse gesetzt: ${config.plugins.updater.endpoints[0]}`)
