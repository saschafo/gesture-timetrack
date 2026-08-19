/**
 * Aktualisierung über GitHub Releases.
 *
 * Bewusst **nur auf Knopfdruck**: Eine Update-Prüfung ist der einzige Moment,
 * in dem diese Anwendung von sich aus ins Netz geht. Eine Hintergrundprüfung
 * würde das Versprechen „läuft vollständig offline" aufweichen, ohne dass der
 * Nutzer es merkt. Er löst sie deshalb selbst aus - und erfährt vorher, was
 * dabei passiert.
 */

import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'

export interface UpdateInfo {
  version: string
  notes: string | null
  date: string | null
}

let pending: Update | null = null

/** Sucht nach einer neueren Fassung. `null` heißt: alles aktuell. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  pending = await check()
  if (!pending) return null
  return {
    version: pending.version,
    notes: pending.body ?? null,
    date: pending.date ?? null,
  }
}

/**
 * Lädt die gefundene Fassung, installiert sie und startet neu.
 * `onProgress` bekommt den Fortschritt in Prozent, sofern die Größe bekannt ist.
 */
export async function installUpdate(onProgress?: (percent: number) => void): Promise<void> {
  if (!pending) throw new Error('Kein Update vorgemerkt.')

  let total = 0
  let loaded = 0
  await pending.downloadAndInstall((event) => {
    if (event.event === 'Started') {
      total = event.data.contentLength ?? 0
    } else if (event.event === 'Progress') {
      loaded += event.data.chunkLength
      if (total > 0) onProgress?.(Math.min(100, Math.round((loaded / total) * 100)))
    }
  })

  // Der Neustart übernimmt die installierte Fassung.
  await relaunch()
}
