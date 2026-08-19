/**
 * Typisierte Hülle um die Tauri-Commands. Alle Zustandsänderungen laufen über
 * das Rust-Backend - das Frontend hält keine eigene Wahrheit über laufende
 * Zeiten.
 */

import { invoke } from '@tauri-apps/api/core'
import type { Gesture } from '../gesture/gesture-classifier.ts'

export type TrackerStatus = 'idle' | 'running' | 'paused'

export interface SlotInfo {
  slot: number
  project_id: number | null
  project_name: string | null
  project_color: string | null
}

export interface Snapshot {
  status: TrackerStatus
  status_label: string
  project_id: number | null
  project_name: string | null
  project_color: string | null
  active_slot: number | null
  elapsed_seconds: number
  pause_seconds: number
  today_seconds: number
  slots: SlotInfo[]
}

export interface Project {
  id: number
  name: string
  color: string
  active: boolean
  created_at: string
}

export interface ProjectTotal {
  project_id: number | null
  project_name: string
  color: string
  seconds: number
}

export interface TimeEntry {
  id: number
  project_id: number | null
  project_name: string
  start_ts: string
  end_ts: string | null
  pause_duration_seconds: number
  duration_seconds: number
  gesture_triggered: boolean
}

export interface AppSettings {
  hotkey: string
  /** Ist der Hotkey tatsächlich registriert? */
  hotkey_active: boolean
  /** Grund, falls die Registrierung fehlgeschlagen ist. */
  hotkey_error: string | null
  confidence_threshold: number
  overlay_timeout_ms: number
  /** Eigenes Zeitfenster für Netzwerk-Kameras (mehr Latenz über WLAN). */
  overlay_timeout_network_ms: number
  /** 'builtin' = eingebaute Webcam, 'network' = Kamera im WLAN. */
  camera_source: string
  camera_url: string
  sound_cue: boolean
  slot_1_project_id: number | null
  slot_2_project_id: number | null
  active_slot: number
  /** Gelten die eingelernten Gesten? */
  use_training: boolean
  /** Sprache der Oberfläche: 'de' oder 'en'. */
  language: string
}

export interface GestureTraining {
  version: number
  samples: Array<{ gesture: string; features: number[] }>
  /** Anzahl Aufnahmen je Geste. */
  counts: Array<[string, number]>
  /** Sind alle Gesten ausreichend eingelernt? */
  complete: boolean
}

export interface CustomGesture {
  id: number
  name: string
  /** Aktionsschlüssel, siehe customGestureActions(). */
  action: string
  project_id: number | null
  project_name: string | null
}

export interface ActionOption {
  key: string
  label: string
  /** Braucht diese Aktion die Angabe eines Projekts? */
  needs_project: boolean
}

export interface GestureOutcome {
  accepted: boolean
  /** Grundgeste - bei einer eigenen Geste `null`. */
  gesture: Gesture | null
  gesture_label: string
  message: string
  snapshot: Snapshot
}

export const EVENT_STATE = 'tracker:state'
export const EVENT_OVERLAY_OPEN = 'overlay:open'
export const EVENT_OVERLAY_CLOSE = 'overlay:close'
export const EVENT_HOTKEY_FIRED = 'hotkey:fired'

export const getState = () => invoke<Snapshot>('get_state')
export const applyGesture = (gesture: Gesture, confidence: number) =>
  invoke<GestureOutcome>('apply_gesture', { gesture, confidence })

export const startTracking = (projectId: number) =>
  invoke<Snapshot>('start_tracking', { projectId })
export const stopTracking = () => invoke<Snapshot>('stop_tracking')
export const pauseTracking = () => invoke<Snapshot>('pause_tracking')
export const resumeTracking = () => invoke<Snapshot>('resume_tracking')
export const setSlot = (slot: number, projectId: number | null) =>
  invoke<Snapshot>('set_slot', { slot, projectId })

export const getProjects = (onlyActive = false) =>
  invoke<Project[]>('get_projects', { onlyActive })
export const createProject = (name: string, color: string) =>
  invoke<Project>('create_project', { name, color })
export const updateProject = (id: number, name: string, color: string, active: boolean) =>
  invoke<void>('update_project', { id, name, color, active })
export const deleteProject = (id: number) => invoke<boolean>('delete_project', { id })

export const dayTotals = (day?: string) => invoke<ProjectTotal[]>('day_totals', { day })
export const listEntries = (from: string, to: string, projectId: number | null = null) =>
  invoke<TimeEntry[]>('list_entries', { from, to, projectId })

/** Eingaben für einen von Hand erfassten oder geänderten Eintrag. */
export interface EntryInput {
  project_id: number
  /** „2026-08-19T09:37" - so liefert es ein datetime-local-Feld. */
  start: string
  end: string
  pause_minutes: number
}

export const createEntry = (input: EntryInput) => invoke<number>('create_entry', { input })
export const updateEntry = (id: number, input: EntryInput) =>
  invoke<void>('update_entry', { id, input })
export const deleteEntry = (id: number) => invoke<void>('delete_entry', { id })
export const exportCsv = (
  path: string,
  from: string,
  to: string,
  projectId: number | null = null,
) => invoke<number>('export_csv', { path, from, to, projectId })

export const getSettings = () => invoke<AppSettings>('get_settings')
export const setSetting = (key: string, value: string) =>
  invoke<AppSettings>('set_setting', { key, value })

export const getCustomGestures = () => invoke<CustomGesture[]>('get_custom_gestures')
export const customGestureActions = () => invoke<ActionOption[]>('custom_gesture_actions')
export const createCustomGesture = (name: string, action: string, projectId: number | null) =>
  invoke<CustomGesture[]>('create_custom_gesture', { name, action, projectId })
export const updateCustomGesture = (
  id: number,
  name: string,
  action: string,
  projectId: number | null,
) => invoke<CustomGesture[]>('update_custom_gesture', { id, name, action, projectId })
export const deleteCustomGesture = (id: number) =>
  invoke<CustomGesture[]>('delete_custom_gesture', { id })
export const applyCustomGesture = (id: number, confidence: number) =>
  invoke<GestureOutcome>('apply_custom_gesture', { id, confidence })

export const getGestureTraining = () => invoke<GestureTraining>('get_gesture_training')
export const recordGestureSamples = (gesture: string, version: number, samples: number[][]) =>
  invoke<GestureTraining>('record_gesture_samples', { gesture, version, samples })
export const clearGestureTraining = (gesture?: string) =>
  invoke<GestureTraining>('clear_gesture_training', { gesture: gesture ?? null })

export const testCameraUrl = (url: string) => invoke<string>('test_camera_url', { url })
export const setCameraPreview = (active: boolean) =>
  invoke<void>('set_camera_preview', { active })

export const closeOverlay = () => invoke<void>('close_overlay')
export const openOverlay = () => invoke<void>('open_overlay')
export const openMainWindow = () => invoke<void>('open_main_window')
export const closePanel = () => invoke<void>('close_panel')
export const resizePanel = (height: number) => invoke<void>('resize_panel', { height })

/** hh:mm:ss für Anzeige. */
export function formatDuration(seconds: number): string {
  const value = Math.max(0, Math.floor(seconds))
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(Math.floor(value / 3600))}:${pad(Math.floor((value % 3600) / 60))}:${pad(value % 60)}`
}

/** Dezimalstunden, wie sie in Rechnungen stehen. */
export function formatHours(seconds: number): string {
  return (seconds / 3600).toFixed(2).replace('.', ',')
}

export function today(): string {
  const now = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}
