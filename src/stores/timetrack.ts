/**
 * Pinia-Store für den App-Zustand des Hauptfensters.
 *
 * Der Store spiegelt nur, was das Backend meldet. Einzige lokale Eigenleistung
 * ist der Sekundenzähler zwischen zwei Backend-Meldungen, damit die Uhr flüssig
 * läuft, ohne jede Sekunde über die IPC-Brücke zu gehen.
 */

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import * as api from '../api/backend.ts'
import { setLocale } from '../i18n.ts'
import type { AppSettings, Project, ProjectTotal, Snapshot, TimeEntry } from '../api/backend.ts'

export const useTimetrackStore = defineStore('timetrack', () => {
  const snapshot = ref<Snapshot | null>(null)
  const projects = ref<Project[]>([])
  const totals = ref<ProjectTotal[]>([])
  const entries = ref<TimeEntry[]>([])
  const settings = ref<AppSettings | null>(null)
  const error = ref<string | null>(null)

  /** Sekunden seit der letzten Backend-Meldung, lokal weitergezählt. */
  const drift = ref(0)
  let unlisten: UnlistenFn | null = null
  let timer: number | null = null

  const status = computed(() => snapshot.value?.status ?? 'idle')
  const isRunning = computed(() => status.value === 'running')
  const isPaused = computed(() => status.value === 'paused')

  const elapsedSeconds = computed(() => {
    if (!snapshot.value) return 0
    return snapshot.value.elapsed_seconds + (isRunning.value ? drift.value : 0)
  })

  const todaySeconds = computed(() => {
    if (!snapshot.value) return 0
    // Laufende Sitzung mitzählen, damit die Tagessumme nicht springt.
    return snapshot.value.today_seconds + elapsedSeconds.value
  })

  const activeProjects = computed(() => projects.value.filter((project) => project.active))

  function applySnapshot(next: Snapshot) {
    snapshot.value = next
    drift.value = 0
  }

  async function guard<T>(action: () => Promise<T>): Promise<T | null> {
    try {
      error.value = null
      return await action()
    } catch (cause) {
      error.value = String(cause)
      return null
    }
  }

  async function refresh() {
    await guard(async () => {
      applySnapshot(await api.getState())
      projects.value = await api.getProjects(false)
      totals.value = await api.dayTotals()
      settings.value = await api.getSettings()
      // Sprache folgt den Einstellungen - auch in Overlay und Menüleisten-Fenster.
      setLocale(settings.value.language)
    })
  }

  async function loadEntries(from: string, to: string, projectId: number | null = null) {
    await guard(async () => {
      entries.value = await api.listEntries(from, to, projectId)
    })
  }

  /** Startet Event-Abo und lokalen Sekundentakt. */
  async function init() {
    await refresh()
    unlisten = await listen<Snapshot>(api.EVENT_STATE, (event) => {
      applySnapshot(event.payload)
      void guard(async () => {
        totals.value = await api.dayTotals()
      })
    })
    timer = window.setInterval(() => {
      if (isRunning.value) drift.value += 1
    }, 1000)
  }

  function dispose() {
    unlisten?.()
    unlisten = null
    if (timer !== null) window.clearInterval(timer)
    timer = null
  }

  const start = (projectId: number) =>
    guard(async () => applySnapshot(await api.startTracking(projectId)))
  const stop = () => guard(async () => applySnapshot(await api.stopTracking()))
  const pause = () => guard(async () => applySnapshot(await api.pauseTracking()))
  const resume = () => guard(async () => applySnapshot(await api.resumeTracking()))

  const assignSlot = (slot: number, projectId: number | null) =>
    guard(async () => applySnapshot(await api.setSlot(slot, projectId)))

  const addProject = (name: string, color: string) =>
    guard(async () => {
      await api.createProject(name, color)
      await refresh()
    })

  const editProject = (id: number, name: string, color: string, active: boolean) =>
    guard(async () => {
      await api.updateProject(id, name, color, active)
      await refresh()
    })

  const removeProject = (id: number) =>
    guard(async () => {
      const deleted = await api.deleteProject(id)
      await refresh()
      return deleted
    })

  /**
   * Einstellung speichern. Lehnt das Backend ab (z. B. belegter Hotkey), wird
   * der tatsächliche Stand nachgeladen - sonst zeigte die Oberfläche einen Wert
   * an, der nie gesetzt wurde.
   */
  async function changeSetting(key: string, value: string) {
    try {
      error.value = null
      settings.value = await api.setSetting(key, value)
      setLocale(settings.value.language)
    } catch (cause) {
      error.value = String(cause)
      try {
        settings.value = await api.getSettings()
      } catch {
        // Stand bleibt wie er war.
      }
    }
  }

  return {
    snapshot,
    projects,
    activeProjects,
    totals,
    entries,
    settings,
    error,
    status,
    isRunning,
    isPaused,
    elapsedSeconds,
    todaySeconds,
    init,
    dispose,
    refresh,
    loadEntries,
    start,
    stop,
    pause,
    resume,
    assignSlot,
    addProject,
    editProject,
    removeProject,
    changeSetting,
  }
})
