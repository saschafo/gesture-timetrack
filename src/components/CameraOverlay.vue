<script setup lang="ts">
/**
 * Kamera-Overlay: das einzige Stück Code, das überhaupt ein Kamerabild sieht.
 *
 * Ablauf einer Auslösung: Hotkey -> Backend zeigt das Fenster und sendet
 * `overlay:open` -> hier startet die eingestellte Bildquelle, es wird für die
 * konfigurierte Zeitspanne nach einer Geste gesucht -> Ergebnis ans Backend ->
 * Bildquelle aus, Fenster zu. Es wird zu keinem Zeitpunkt ein Bild gespeichert.
 */
import { onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import GestureIndicator from './GestureIndicator.vue'
import type { IndicatorState } from './indicator-state'
import { HandTracker } from '../gesture/mediapipe-hands'
import {
  coverBox,
  createFrameSource,
  detectionTimeoutMs,
  type FrameSource,
} from '../gesture/frame-source'
import {
  GestureStabilizer,
  parseCustomLabel,
  type Point,
} from '../gesture/gesture-classifier'
import { createRecognizer, type Recognizer } from '../gesture/recognizer'
import { setLocale, t } from '../i18n'
import {
  EVENT_OVERLAY_CLOSE,
  EVENT_OVERLAY_OPEN,
  applyCustomGesture,
  applyGesture,
  closeOverlay,
  getGestureTraining,
  getSettings,
} from '../api/backend'

/** Wie lange das Ergebnis (grüner/roter Rahmen) sichtbar bleibt. */
const RESULT_MS = 900

const video = ref<HTMLVideoElement | null>(null)
/** Anzeigefläche der Netzwerk-Kamera. */
const display = ref<HTMLCanvasElement | null>(null)
/** Ebene für die erkannten Handpunkte. */
const canvas = ref<HTMLCanvasElement | null>(null)

const state = ref<IndicatorState>('starting')
const message = ref('')
const confidence = ref(0)
const remaining = ref(1)
const mirrored = ref(true)
const isNetwork = ref(false)

const tracker = shallowRef(new HandTracker())
const source = shallowRef<FrameSource | null>(null)
const recognizer = shallowRef<Recognizer>(createRecognizer({ use_training: false }))
let stabilizer = new GestureStabilizer(0.85)
let frameHandle: number | null = null
let closeHandle: number | null = null
/** Ende des Erkennungsfensters; wird erst gesetzt, wenn Bilder ankommen. */
let deadline = 0
/** Frist, bis zu der das erste Bild da sein muss. */
let firstFrameDeadline = 0
let timeoutMs = 3000
let soundCue = true
let busy = false
const unlisteners: UnlistenFn[] = []

/** Kurzer Ton als Bestätigung - erzeugt statt geladen, damit offline nichts fehlt. */
function beep(ok: boolean) {
  if (!soundCue) return
  try {
    const context = new AudioContext()
    const oscillator = context.createOscillator()
    const gain = context.createGain()
    oscillator.frequency.value = ok ? 880 : 320
    oscillator.type = 'sine'
    gain.gain.setValueAtTime(0.0001, context.currentTime)
    gain.gain.exponentialRampToValueAtTime(0.08, context.currentTime + 0.01)
    gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 0.16)
    oscillator.connect(gain).connect(context.destination)
    oscillator.start()
    oscillator.stop(context.currentTime + 0.18)
    oscillator.onended = () => void context.close()
  } catch {
    // Ton ist reines Extra - Fehler hier dürfen die Erfassung nicht stören.
  }
}

function drawLandmarks(points: Point[] | null) {
  const element = canvas.value
  const context = element?.getContext('2d')
  if (!element || !context) return
  context.clearRect(0, 0, element.width, element.height)
  if (!points) return

  // Die Punkte liegen in Bildkoordinaten der Quelle, die Vorschau ist aber
  // formatfüllend beschnitten - ohne denselben Zuschnitt sitzen die Punkte
  // neben den Fingern.
  const size = source.value?.frameSize()
  const box = coverBox(size?.width ?? 0, size?.height ?? 0, element.width, element.height)

  context.fillStyle = 'rgba(139, 133, 255, 0.9)'
  for (const point of points) {
    // Gespiegelt nur bei der eigenen Webcam - dort zeigt die Vorschau ebenfalls
    // ein Spiegelbild.
    const relative = mirrored.value ? 1 - point.x : point.x
    context.beginPath()
    context.arc(box.x + relative * box.width, box.y + point.y * box.height, 2.4, 0, Math.PI * 2)
    context.fill()
  }
}

function stopLoop() {
  if (frameHandle !== null) cancelAnimationFrame(frameHandle)
  frameHandle = null
}

function finish(accepted: boolean, text: string) {
  state.value = accepted ? 'accepted' : 'rejected'
  message.value = text
  beep(accepted)
  stopLoop()
  drawLandmarks(null)
  if (closeHandle !== null) window.clearTimeout(closeHandle)
  closeHandle = window.setTimeout(() => void closeOverlay(), RESULT_MS)
}

/** Meldet, warum kein Bild ankommt - bei der Netzwerk-Kamera oft die Adresse. */
async function reportNoFrames() {
  const detail = await source.value?.lastError()
  finish(false, detail ?? t('overlay.noImage'))
}

function loop() {
  frameHandle = requestAnimationFrame(loop)
  if (busy || !source.value) return

  const now = performance.now()
  const image = source.value.frame()

  if (!image) {
    // Noch kein Bild: erst die Verbindungsfrist prüfen, nicht schon zählen.
    if (now >= firstFrameDeadline) {
      busy = true
      void reportNoFrames().finally(() => (busy = false))
    }
    return
  }

  if (deadline === 0) {
    // Erstes Bild ist da - ab jetzt läuft das Erkennungsfenster.
    deadline = now + timeoutMs
    state.value = 'searching'
    message.value = t('overlay.showGesture')
  }
  remaining.value = Math.max(0, (deadline - now) / timeoutMs)

  const { landmarks, world } = tracker.value.detect(image, now)
  drawLandmarks(landmarks as Point[] | null)

  if (landmarks) {
    const result = recognizer.value.classify(landmarks as Point[], world as Point[] | null)
    confidence.value = result.confidence
    const confirmed = stabilizer.push(result)
    if (confirmed) {
      busy = true
      // Eigene Gesten kennt nur das Backend - dort liegt ihre Aktion.
      const customId = parseCustomLabel(confirmed.label)
      const request =
        customId !== null
          ? applyCustomGesture(customId, confirmed.confidence)
          : applyGesture(confirmed.gesture!, confirmed.confidence)

      void request
        .then((outcome) => {
          const name = outcome.gesture_label
          const action = outcome.gesture
            ? t(`gestureAction.${outcome.gesture}`)
            : outcome.message
          finish(
            outcome.accepted,
            outcome.accepted ? `${name} → ${action}` : `${name}: ${outcome.message}`,
          )
        })
        .catch((error) => finish(false, String(error)))
      return
    }
  } else {
    confidence.value = 0
    stabilizer.reset()
  }

  if (now >= deadline) {
    finish(false, t('overlay.noGesture'))
  }
}

async function begin() {
  if (busy) return
  busy = true
  state.value = 'starting'
  confidence.value = 0
  remaining.value = 1
  deadline = 0

  try {
    const settings = await getSettings()
    // Sprache übernehmen: das Overlay hat keinen Store, der das täte.
    setLocale(settings.language)
    timeoutMs = detectionTimeoutMs(settings)
    soundCue = settings.sound_cue
    stabilizer = new GestureStabilizer(settings.confidence_threshold)

    // Immer laden: eigene Gesten wirken auch ohne vollständiges Training, nur
    // eben nachrangig zu den Grundgesten.
    const training = await getGestureTraining().catch(() => null)
    recognizer.value = createRecognizer(
      settings,
      training ? { version: training.version, samples: training.samples } : null,
    )

    if (!video.value || !display.value) throw new Error(t('overlay.notReady'))
    const frameSource = createFrameSource(settings, video.value, display.value)
    source.value = frameSource
    isNetwork.value = frameSource.kind === 'network'
    mirrored.value = frameSource.mirrored
    message.value = isNetwork.value
      ? t('overlay.networkConnecting')
      : t('overlay.cameraStarting')

    await tracker.value.init()
    tracker.value.resetClock()
    await frameSource.start()

    // Beim Netzwerkweg darf der Verbindungsaufbau das Erkennungsfenster nicht
    // aufbrauchen: die Zeit läuft erst ab dem ersten Bild.
    firstFrameDeadline = performance.now() + timeoutMs
    busy = false
    stopLoop()
    loop()
  } catch (error) {
    busy = false
    const text = String(error)
    finish(
      false,
      text.includes('NotAllowed') || text.includes('Permission')
        ? t('overlay.noAccess')
        : t('overlay.cameraUnavailable'),
    )
  }
}

function end() {
  stopLoop()
  if (closeHandle !== null) window.clearTimeout(closeHandle)
  closeHandle = null
  stabilizer.reset()
  busy = false
  deadline = 0
  source.value?.stop()
  source.value = null
  drawLandmarks(null)
  state.value = 'starting'
  message.value = t('overlay.cameraOff')
  confidence.value = 0
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') void closeOverlay()
}

onMounted(async () => {
  unlisteners.push(await listen(EVENT_OVERLAY_OPEN, () => void begin()))
  unlisteners.push(await listen(EVENT_OVERLAY_CLOSE, () => end()))
  // Esc bricht ab, ohne etwas zu buchen.
  window.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  unlisteners.forEach((unlisten) => unlisten())
  end()
  tracker.value.close()
})
</script>

<template>
  <div class="overlay">
    <div class="viewport">
      <video
        v-show="!isNetwork"
        ref="video"
        class="layer"
        :class="{ mirrored }"
        playsinline
        muted
      ></video>
      <canvas v-show="isNetwork" ref="display" class="layer" width="200" height="200"></canvas>
      <canvas ref="canvas" class="layer" width="200" height="200"></canvas>
      <span v-if="isNetwork" class="tag">WLAN</span>
      <GestureIndicator
        :state="state"
        :message="message"
        :confidence="confidence"
        :remaining="remaining"
      />
    </div>
  </div>
</template>

<style scoped>
.overlay {
  width: 100vw;
  height: 100vh;
  padding: 10px;
  background: #0d0f16;
  border-radius: 12px;
}

.viewport {
  position: relative;
  width: 200px;
  height: 200px;
  border-radius: 10px;
  overflow: hidden;
  background: #05070c;
}

.layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

/* Gespiegelt, weil sich Menschen im Spiegelbild besser justieren. */
.layer.mirrored {
  transform: scaleX(-1);
}

.tag {
  position: absolute;
  top: 6px;
  left: 6px;
  font-size: 9.5px;
  font-weight: 700;
  letter-spacing: 0.6px;
  color: #c7d2fe;
  background: rgba(13, 15, 22, 0.75);
  border-radius: 5px;
  padding: 1px 5px;
}
</style>
