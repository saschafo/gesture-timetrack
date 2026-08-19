/**
 * Bildquellen für die Gestenerkennung.
 *
 * Die Erkennung selbst kennt nur noch `FrameSource` und bekommt pro Aufruf ein
 * fertiges Bild. Dadurch ist sie unabhängig davon, ob das Bild aus der
 * eingebauten Webcam (`<video>` + getUserMedia) oder aus einer Netzwerk-Kamera
 * im WLAN kommt (Handy-App mit MJPEG-Stream).
 *
 * Standard bleibt die eingebaute Webcam - der Netzwerkweg ist rein optional und
 * muss vom Nutzer bewusst eingerichtet werden.
 */

import { invoke } from '@tauri-apps/api/core'
import type { ImageSource } from '@mediapipe/tasks-vision'

import type { AppSettings } from '../api/backend.ts'

export type CameraSource = 'builtin' | 'network'

export interface FrameSource {
  readonly kind: CameraSource
  /** Spiegeln bei der Anzeige? Nur beim Blick in die eigene Webcam sinnvoll. */
  readonly mirrored: boolean
  start(): Promise<void>
  stop(): void
  /** Aktuelles Bild oder `null`, solange noch keines vorliegt. */
  frame(): ImageSource | null
  /**
   * Echte Bildmaße der Quelle. Gebraucht, um die erkannten Handpunkte
   * deckungsgleich über die formatfüllend angezeigte Vorschau zu zeichnen.
   */
  frameSize(): { width: number; height: number } | null
  /** Klartext-Fehler der Quelle, falls vorhanden. */
  lastError(): Promise<string | null>
}

/** Eingebaute bzw. per USB angeschlossene Kamera. */
export class WebcamSource implements FrameSource {
  readonly kind = 'builtin' as const
  readonly mirrored = true
  private stream: MediaStream | null = null
  private readonly video: HTMLVideoElement

  constructor(video: HTMLVideoElement) {
    this.video = video
  }

  async start(): Promise<void> {
    if (this.stream) return
    this.stream = await navigator.mediaDevices.getUserMedia({
      video: { width: { ideal: 640 }, height: { ideal: 480 }, facingMode: 'user' },
      audio: false,
    })
    this.video.srcObject = this.stream
    await this.video.play()
  }

  /** Beendet den Stream - danach erlischt auch die Kamera-LED. */
  stop(): void {
    this.stream?.getTracks().forEach((track) => track.stop())
    this.stream = null
    this.video.srcObject = null
  }

  frame(): ImageSource | null {
    return this.stream && this.video.readyState >= 2 ? this.video : null
  }

  frameSize(): { width: number; height: number } | null {
    return this.video.videoWidth
      ? { width: this.video.videoWidth, height: this.video.videoHeight }
      : null
  }

  async lastError(): Promise<string | null> {
    return null
  }
}

/**
 * Netzwerk-Kamera (MJPEG-Stream oder Einzelbild-Adresse).
 *
 * Die Bilddaten holt das Rust-Backend und reicht sie als Rohdaten herein; ein
 * direkt eingebundener fremder Stream wäre für WebGL eine Cross-Origin-Quelle
 * und damit für MediaPipe unbrauchbar (siehe `src-tauri/src/camera.rs`).
 */
export class NetworkCameraSource implements FrameSource {
  readonly kind = 'network' as const
  readonly mirrored = false

  private bitmap: ImageBitmap | null = null
  private pending = false
  private active = false
  private readonly canvas: HTMLCanvasElement

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas
  }

  async start(): Promise<void> {
    this.active = true
    void this.pull()
  }

  stop(): void {
    this.active = false
    this.bitmap?.close()
    this.bitmap = null
    const context = this.canvas.getContext('2d')
    context?.clearRect(0, 0, this.canvas.width, this.canvas.height)
  }

  frame(): ImageSource | null {
    // Nachschub anfordern, ohne den Erkennungstakt zu blockieren.
    void this.pull()
    return this.bitmap
  }

  frameSize(): { width: number; height: number } | null {
    return this.bitmap ? { width: this.bitmap.width, height: this.bitmap.height } : null
  }

  async lastError(): Promise<string | null> {
    return (await invoke<string | null>('camera_error')) ?? null
  }

  /** Holt genau ein Bild; parallele Aufrufe werden verworfen. */
  private async pull(): Promise<void> {
    if (!this.active || this.pending) return
    this.pending = true
    try {
      const data = await invoke<ArrayBuffer>('camera_frame')
      if (!this.active || !data || data.byteLength === 0) return

      const bitmap = await createImageBitmap(new Blob([data], { type: 'image/jpeg' }))
      if (!this.active) {
        bitmap.close()
        return
      }
      this.bitmap?.close()
      this.bitmap = bitmap
      this.draw(bitmap)
    } catch {
      // Einzelne kaputte Bilder sind bei MJPEG normal - einfach überspringen.
    } finally {
      this.pending = false
    }
  }

  /** Zeigt das Bild formatfüllend an (entspricht `object-fit: cover`). */
  private draw(bitmap: ImageBitmap): void {
    const context = this.canvas.getContext('2d')
    if (!context) return
    const box = coverBox(bitmap.width, bitmap.height, this.canvas.width, this.canvas.height)
    context.drawImage(bitmap, box.x, box.y, box.width, box.height)
  }
}

/**
 * Zuschnitt eines Bildes, das eine Fläche formatfüllend ausfüllt - dieselbe
 * Rechnung wie `object-fit: cover`, nur eben zum Nachrechnen für die
 * Punkteebene.
 */
export function coverBox(
  sourceWidth: number,
  sourceHeight: number,
  targetWidth: number,
  targetHeight: number,
): { x: number; y: number; width: number; height: number } {
  if (!sourceWidth || !sourceHeight) {
    return { x: 0, y: 0, width: targetWidth, height: targetHeight }
  }
  const scale = Math.max(targetWidth / sourceWidth, targetHeight / sourceHeight)
  const width = sourceWidth * scale
  const height = sourceHeight * scale
  return { x: (targetWidth - width) / 2, y: (targetHeight - height) / 2, width, height }
}

export function cameraSourceOf(settings: AppSettings): CameraSource {
  return settings.camera_source === 'network' && settings.camera_url.trim()
    ? 'network'
    : 'builtin'
}

/**
 * Zeitfenster der Erkennung. Netzwerk-Kameras bekommen mehr Zeit, weil ein
 * MJPEG-Stream über WLAN spürbar später ankommt als die lokale Webcam. Beide
 * Werte stehen in den Einstellungen und sind nicht fest verdrahtet.
 */
export function detectionTimeoutMs(settings: AppSettings): number {
  return cameraSourceOf(settings) === 'network'
    ? settings.overlay_timeout_network_ms
    : settings.overlay_timeout_ms
}

export function createFrameSource(
  settings: AppSettings,
  video: HTMLVideoElement,
  canvas: HTMLCanvasElement,
): FrameSource {
  return cameraSourceOf(settings) === 'network'
    ? new NetworkCameraSource(canvas)
    : new WebcamSource(video)
}
