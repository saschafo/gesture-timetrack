/**
 * MediaPipe-Anbindung (Tasks Vision, WASM).
 *
 * Modell und WASM-Laufzeit liegen als lokale Dateien im Bundle
 * (`public/models`, `public/mediapipe/wasm`, siehe scripts/fetch-assets.mjs).
 * Die Erkennung selbst lädt damit nichts nach.
 *
 * Woher das Bild kommt, ist hier bewusst nicht bekannt - das entscheidet die
 * `FrameSource` (eingebaute Webcam oder Netzwerk-Kamera).
 */

import {
  FilesetResolver,
  HandLandmarker,
  type ImageSource,
  type Landmark,
  type NormalizedLandmark,
} from '@mediapipe/tasks-vision'

const WASM_PATH = '/mediapipe/wasm'
const MODEL_PATH = '/models/hand_landmarker.task'

export interface Detection {
  /** Bildkoordinaten - für die Anzeige und für Richtungsfragen. */
  landmarks: NormalizedLandmark[] | null
  /**
   * Metrische Weltkoordinaten derselben Hand. Frei von Perspektive und vom
   * Seitenverhältnis des Bildes und damit die bessere Grundlage, um die Form
   * der Hand zu beurteilen.
   */
  world: Landmark[] | null
}

export class HandTracker {
  private landmarker: HandLandmarker | null = null
  private lastTimestamp = -1

  /** Lädt Modell und Laufzeit. Bewusst einmal pro Sitzung, nicht pro Auslösung. */
  async init(): Promise<void> {
    if (this.landmarker) return
    const fileset = await FilesetResolver.forVisionTasks(WASM_PATH)
    this.landmarker = await HandLandmarker.createFromOptions(fileset, {
      baseOptions: { modelAssetPath: MODEL_PATH, delegate: 'GPU' },
      runningMode: 'VIDEO',
      numHands: 1,
      // Bewusst niedrig: Ob eine Geste gilt, entscheidet die eigene
      // Klassifikation samt Schwellwert - hier soll die Hand erst einmal
      // überhaupt gefunden werden, auch bei mäßigem Licht.
      minHandDetectionConfidence: 0.5,
      minHandPresenceConfidence: 0.5,
      minTrackingConfidence: 0.5,
    })
  }

  /** Einzelbild auswerten. Gibt `null` zurück, solange keine Hand sichtbar ist. */
  detect(image: ImageSource, timestampMs: number): Detection {
    if (!this.landmarker) return { landmarks: null, world: null }

    // MediaPipe verlangt streng monoton steigende Zeitstempel.
    const timestamp = timestampMs <= this.lastTimestamp ? this.lastTimestamp + 1 : timestampMs
    this.lastTimestamp = timestamp

    const result = this.landmarker.detectForVideo(image, timestamp)
    const landmarks = result.landmarks?.[0]
    if (!landmarks || landmarks.length < 21) return { landmarks: null, world: null }

    const world = result.worldLandmarks?.[0]
    return { landmarks, world: world && world.length >= 21 ? world : null }
  }

  /** Setzt den Zeitstempel zurück, z. B. beim Wechsel der Bildquelle. */
  resetClock(): void {
    this.lastTimestamp = -1
  }

  close(): void {
    this.landmarker?.close()
    this.landmarker = null
  }
}
