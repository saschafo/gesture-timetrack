<script setup lang="ts">
/**
 * Live-Vorschau im Hauptfenster - zum Ausrichten der Kamera und zum Prüfen, ob
 * eine Geste sauber erkannt wird.
 *
 * Bewusst nur auf Knopfdruck: Solange die Vorschau läuft, ist die Kamera an,
 * und das soll sichtbar bleiben. Gebucht wird hier nichts - die Vorschau zeigt
 * nur, was die Erkennung sieht.
 */
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'

import GestureTrainingPanel from './GestureTrainingPanel.vue'
import Icon from './Icon.vue'
import {
  clearGestureTraining,
  createCustomGesture,
  customGestureActions,
  deleteCustomGesture,
  getCustomGestures,
  getGestureTraining,
  getProjects,
  getSettings,
  recordGestureSamples,
  setCameraPreview,
  setSetting,
  updateCustomGesture,
  type ActionOption,
  type CustomGesture,
  type GestureTraining,
  type Project,
} from '../api/backend'
import {
  coverBox,
  createFrameSource,
  cameraSourceOf,
  type FrameSource,
} from '../gesture/frame-source'
import {
  parseCustomLabel,
  type FingerScores,
  type Point,
  type ThumbSignal,
} from '../gesture/gesture-classifier'
import { FEATURE_VERSION, featureVector } from '../gesture/features'
import { createRecognizer, type Recognizer } from '../gesture/recognizer'
import { t } from '../i18n'
import { HandTracker } from '../gesture/mediapipe-hands'

const video = ref<HTMLVideoElement | null>(null)
const display = ref<HTMLCanvasElement | null>(null)
const points = ref<HTMLCanvasElement | null>(null)

const running = ref(false)
const starting = ref(false)
const error = ref<string | null>(null)
const isNetwork = ref(false)
const mirrored = ref(true)
const gesture = ref<string | null>(null)
const action = ref<string | null>(null)
const confidence = ref(0)
const threshold = ref(0.85)
const waiting = ref(true)
/** Messwerte je Finger - zeigt, woran eine Geste hängt. */
const fingers = ref<FingerScores | null>(null)
const thumb = ref<ThumbSignal | null>(null)

const tracker = shallowRef(new HandTracker())
const source = shallowRef<FrameSource | null>(null)
const recognizer = shallowRef<Recognizer>(createRecognizer({ use_training: false }))
let frameHandle: number | null = null

// --- Einlernen ---

/** Countdown, damit die Hand vor der Aufnahme in Position ist. */
const COUNTDOWN_STEPS = 3
const COUNTDOWN_MS = 700
/** Aufnahmedauer und Obergrenze an gespeicherten Bildern. */
const CAPTURE_MS = 1600
const MAX_SAMPLES = 40

const training = ref<GestureTraining | null>(null)
const useTraining = ref(false)
const recordingGesture = ref<string | null>(null)
const recordingStatus = ref<string | null>(null)
const customGestures = ref<CustomGesture[]>([])
const actionOptions = ref<ActionOption[]>([])
const projects = ref<Project[]>([])
let capturing = false
let captured: number[][] = []

const counts = computed<Record<string, number>>(() =>
  Object.fromEntries(training.value?.counts ?? []),
)

const trainingComplete = computed(() => training.value?.complete ?? false)

async function loadTraining() {
  try {
    training.value = await getGestureTraining()
  } catch {
    training.value = null
  }
}

async function loadCustom() {
  try {
    customGestures.value = await getCustomGestures()
    projects.value = await getProjects(true)
  } catch (cause) {
    error.value = String(cause)
  }
}

/** Klartextname einer Kennung - für die Meldungen während der Aufnahme. */
function labelName(label: string): string {
  const id = parseCustomLabel(label)
  if (id !== null) {
    return (
      customGestures.value.find((gesture) => gesture.id === id)?.name ??
      t('training.ownFallbackName')
    )
  }
  return t(`gesture.${label}`)
}

function refreshRecognizer() {
  recognizer.value = createRecognizer(
    { use_training: useTraining.value, confidence_threshold: threshold.value },
    training.value ? { version: training.value.version, samples: training.value.samples } : null,
  )
}

onMounted(async () => {
  await Promise.all([loadTraining(), loadCustom()])
  try {
    actionOptions.value = await customGestureActions()
    useTraining.value = (await getSettings()).use_training
  } catch {
    useTraining.value = false
  }
  refreshRecognizer()
})

const sleep = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms))

/** Nimmt eine Geste auf: Countdown, dann Merkmale sammeln. */
async function recordGesture(label: string) {
  if (!running.value || recordingGesture.value) return
  error.value = null
  recordingGesture.value = label
  captured = []
  const name = labelName(label)

  try {
    for (let step = COUNTDOWN_STEPS; step > 0; step--) {
      recordingStatus.value = t('training.countdown', { name, step })
      await sleep(COUNTDOWN_MS)
    }

    recordingStatus.value = t('training.hold', { name })
    capturing = true
    await sleep(CAPTURE_MS)
    capturing = false

    if (captured.length < 5) {
      error.value = t('training.tooFew')
      return
    }
    training.value = await recordGestureSamples(
      label,
      FEATURE_VERSION,
      captured.slice(0, MAX_SAMPLES),
    )
    recordingStatus.value = t('training.saved', { name, count: captured.length })
    refreshRecognizer()
    window.setTimeout(() => (recordingStatus.value = null), 2500)
  } catch (cause) {
    error.value = String(cause)
  } finally {
    capturing = false
    captured = []
    recordingGesture.value = null
  }
}

async function clearTraining(label?: string) {
  try {
    training.value = await clearGestureTraining(label)
    if (!training.value.complete) useTraining.value = false
    refreshRecognizer()
  } catch (cause) {
    error.value = String(cause)
  }
}

async function addCustom(name: string, action: string, projectId: number | null) {
  try {
    customGestures.value = await createCustomGesture(name, action, projectId)
  } catch (cause) {
    error.value = String(cause)
  }
}

async function editCustom(id: number, name: string, action: string, projectId: number | null) {
  try {
    customGestures.value = await updateCustomGesture(id, name, action, projectId)
  } catch (cause) {
    error.value = String(cause)
  }
}

async function removeCustom(id: number) {
  try {
    customGestures.value = await deleteCustomGesture(id)
    await loadTraining()
    refreshRecognizer()
  } catch (cause) {
    error.value = String(cause)
  }
}

async function toggleTraining(value: boolean) {
  useTraining.value = value
  try {
    await setSetting('use_training', value ? '1' : '0')
  } catch (cause) {
    error.value = String(cause)
    useTraining.value = !value
  }
  refreshRecognizer()
}

function drawPoints(landmarks: Point[] | null) {
  const element = points.value
  const context = element?.getContext('2d')
  if (!element || !context) return
  context.clearRect(0, 0, element.width, element.height)
  if (!landmarks) return

  const size = source.value?.frameSize()
  const box = coverBox(size?.width ?? 0, size?.height ?? 0, element.width, element.height)
  context.fillStyle = 'rgba(139, 133, 255, 0.95)'
  for (const point of landmarks) {
    const relative = mirrored.value ? 1 - point.x : point.x
    context.beginPath()
    context.arc(box.x + relative * box.width, box.y + point.y * box.height, 3, 0, Math.PI * 2)
    context.fill()
  }
}

function loop() {
  frameHandle = requestAnimationFrame(loop)
  const image = source.value?.frame()
  waiting.value = !image
  if (!image) return

  const { landmarks, world } = tracker.value.detect(image, performance.now())
  drawPoints(landmarks as Point[] | null)

  if (!landmarks) {
    gesture.value = null
    action.value = null
    confidence.value = 0
    fingers.value = null
    thumb.value = null
    return
  }
  if (capturing) captured.push(featureVector(landmarks as Point[], world as Point[] | null))

  const result = recognizer.value.classify(landmarks as Point[], world as Point[] | null)
  gesture.value = result.label ? labelName(result.label) : null
  action.value = result.gesture ? t(`gestureAction.${result.gesture}`) : null
  confidence.value = result.confidence
  fingers.value = result.fingers
  thumb.value = result.thumb
}

async function start() {
  if (running.value || starting.value) return
  starting.value = true
  error.value = null
  try {
    const settings = await getSettings()
    threshold.value = settings.confidence_threshold
    isNetwork.value = cameraSourceOf(settings) === 'network'
    useTraining.value = settings.use_training
    await Promise.all([loadTraining(), loadCustom()])
    refreshRecognizer()

    if (!video.value || !display.value) throw new Error(t('preview.notReady'))
    // Das Backend hält die Netzwerkverbindung offen, solange die Vorschau läuft.
    if (isNetwork.value) await setCameraPreview(true)

    const frameSource = createFrameSource(settings, video.value, display.value)
    mirrored.value = frameSource.mirrored
    await tracker.value.init()
    tracker.value.resetClock()
    await frameSource.start()
    source.value = frameSource

    running.value = true
    loop()

    // Nach kurzer Zeit ohne Bild den Grund nachfragen.
    window.setTimeout(async () => {
      if (running.value && waiting.value) {
        error.value = (await frameSource.lastError()) ?? t('preview.noImage')
      }
    }, 3000)
  } catch (cause) {
    const text = String(cause)
    error.value =
      text.includes('NotAllowed') || text.includes('Permission')
        ? t('preview.noAccess')
        : text
    await stop()
  } finally {
    starting.value = false
  }
}

async function stop() {
  if (frameHandle !== null) cancelAnimationFrame(frameHandle)
  frameHandle = null
  source.value?.stop()
  source.value = null
  running.value = false
  gesture.value = null
  action.value = null
  confidence.value = 0
  fingers.value = null
  thumb.value = null
  drawPoints(null)
  if (isNetwork.value) await setCameraPreview(false)
}

onBeforeUnmount(() => {
  void stop()
  tracker.value.close()
})
</script>

<template>
  <div class="preview">
    <!-- Zwei Spalten, solange Platz ist: links das Bild, rechts das Einlernen.
         Untereinander wäre die Karte auf breiten Fenstern unnötig hoch. -->
    <div class="split">
      <div class="live">
        <div class="stage" :class="{ live: running }">
      <video
        v-show="running && !isNetwork"
        ref="video"
        class="layer"
        :class="{ mirrored }"
        playsinline
        muted
      ></video>
      <canvas
        v-show="running && isNetwork"
        ref="display"
        class="layer"
        width="320"
        height="240"
      ></canvas>
      <canvas ref="points" class="layer" width="320" height="240"></canvas>

      <p v-if="!running" class="placeholder muted">
        <Icon name="camera" :size="18" />
        {{ t('preview.off') }}
      </p>
      <p v-else-if="waiting" class="placeholder muted">{{ t('preview.waiting') }}</p>
          <span v-if="running && isNetwork" class="tag">WLAN</span>
        </div>

        <div class="row controls">
          <button v-if="!running" class="primary" :disabled="starting" @click="start">
        <Icon name="camera" :size="15" />
        {{ starting ? t('preview.starting') : t('preview.start') }}
      </button>
          <button v-else @click="stop">
            <Icon name="stop" :size="15" /> {{ t('preview.stop') }}
          </button>

          <div v-if="running" class="reading">
        <span class="mode muted">{{
          recognizer.mode === 'trained'
            ? t('preview.modeTraining')
            : recognizer.mode === 'hybrid'
              ? t('preview.modeHybrid')
              : t('preview.modeRules')
        }}</span>
        <span v-if="gesture" class="mono">
          {{ gesture }} → {{ action }}
          <strong :class="{ ok: confidence >= threshold }">
            {{ Math.round(confidence * 100) }} %
          </strong>
        </span>
        <span v-else class="muted">{{ t('preview.noHand') }}</span>
        <div class="bar">
          <span :style="{ width: `${Math.round(confidence * 100)}%` }"></span>
          <i :style="{ left: `${Math.round(threshold * 100)}%` }"></i>
        </div>
      </div>
    </div>

        <table v-if="running && fingers" class="values mono">
      <tbody>
        <tr>
          <td>{{ t('preview.fingers') }}</td>
          <td v-for="(value, finger) in fingers" :key="finger">
            {{ String(finger)[0].toUpperCase() }}
            <b :class="{ on: value > 0.5 }">{{ Math.round(value * 100) }}</b>
          </td>
          <td v-if="thumb">
            {{ t('preview.thumb') }}
            <b :class="{ on: thumb.up > 0.5 }">↑{{ Math.round(thumb.up * 100) }}</b>
          </td>
        </tr>
      </tbody>
    </table>

      </div>

      <div class="train">
        <GestureTrainingPanel
      :counts="counts"
      :custom="customGestures"
      :actions="actionOptions"
      :projects="projects"
      :complete="trainingComplete"
      :use-training="useTraining"
      :ready="running"
      :active="recordingGesture"
      :status="recordingStatus"
      @record="recordGesture"
      @clear="clearTraining"
      @toggle="toggleTraining"
          @create-custom="addCustom"
          @update-custom="editCustom"
          @delete-custom="removeCustom"
        />
      </div>
    </div>

    <p v-if="error" class="warn">{{ error }}</p>
    <p v-else class="note muted">{{ t('preview.note') }}</p>
  </div>
</template>

<style scoped>
.preview {
  /* Maßgeblich ist die Breite der Karte, nicht die des Fensters: die Vorschau
     sitzt in einer Spalte, die schmaler sein kann als das Fenster. */
  container-type: inline-size;
}

.split {
  display: grid;
  grid-template-columns: minmax(260px, 340px) minmax(0, 1fr);
  gap: 20px;
  align-items: start;
}

.train {
  border-left: 1px solid var(--border);
  padding-left: 20px;
}

@container (max-width: 720px) {
  .split {
    grid-template-columns: 1fr;
  }

  .train {
    border-left: none;
    border-top: 1px solid var(--border);
    padding: 14px 0 0;
  }
}

/* Ausweg für Webviews ohne Container-Abfragen. */
@supports not (container-type: inline-size) {
  @media (max-width: 1100px) {
    .split {
      grid-template-columns: 1fr;
    }

    .train {
      border-left: none;
      border-top: 1px solid var(--border);
      padding: 14px 0 0;
    }
  }
}

.stage {
  position: relative;
  width: 100%;
  aspect-ratio: 4 / 3;
  max-height: 260px;
  border-radius: 10px;
  overflow: hidden;
  background: var(--surface-2);
  border: 1px solid var(--border);
}

.stage.live {
  background: #05070c;
}

.layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.layer.mirrored {
  transform: scaleX(-1);
}

.placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  margin: 0;
  font-size: 12.5px;
}

.controls button {
  display: inline-flex;
  align-items: center;
  gap: 7px;
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

.controls {
  margin-top: 12px;
  align-items: flex-start;
}

.reading {
  flex: 1;
  min-width: 0;
  font-size: 12.5px;
}

.mode {
  display: block;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.reading strong {
  color: var(--muted);
}

.reading strong.ok {
  color: var(--success);
}

.bar {
  position: relative;
  height: 5px;
  margin-top: 5px;
  border-radius: 999px;
  background: var(--surface-2);
}

.bar span {
  display: block;
  height: 100%;
  border-radius: 999px;
  background: var(--accent);
  transition: width 0.1s linear;
}

.bar i {
  position: absolute;
  top: -2px;
  width: 2px;
  height: 9px;
  background: var(--text);
  opacity: 0.55;
}

.values {
  margin-top: 10px;
  width: 100%;
  font-size: 11.5px;
  color: var(--muted);
}

.values td {
  padding-right: 10px;
  white-space: nowrap;
}

.values b {
  color: var(--muted);
  font-weight: 600;
}

.values b.on {
  color: var(--accent);
}

.note,
.warn {
  margin: 10px 0 0;
  font-size: 12px;
}

.warn {
  color: var(--warning);
}
</style>
