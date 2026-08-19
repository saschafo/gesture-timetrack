<script setup lang="ts">
/** Einstellungen: Hotkey, Konfidenz-Schwelle, Erkennungsdauer, Ton. */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import CollapsibleCard from './CollapsibleCard.vue'
import Icon from './Icon.vue'
import { getVersion } from '@tauri-apps/api/app'

import { EVENT_HOTKEY_FIRED, openOverlay, testCameraUrl } from '../api/backend'
import { checkForUpdate, installUpdate } from '../api/updates'
import {
  DEFAULT_HOTKEY,
  HOTKEY_PRESETS,
  formatHotkey,
  isMacPlatform,
  shortcutFromEvent,
} from '../api/hotkey'
import { t } from '../i18n'
import { useTimetrackStore } from '../stores/timetrack'

const store = useTimetrackStore()

const hotkey = ref('')
const recording = ref(false)
const cameraUrl = ref('')
/** Sichtbare Bestätigung, dass der Hotkey wirklich beim Programm ankommt. */
const fired = ref(false)
const isMac = isMacPlatform()

let firedHandle: number | null = null
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  unlisten = await listen(EVENT_HOTKEY_FIRED, () => {
    fired.value = true
    if (firedHandle !== null) window.clearTimeout(firedHandle)
    firedHandle = window.setTimeout(() => (fired.value = false), 2500)
  })
})

onBeforeUnmount(() => {
  unlisten?.()
  if (firedHandle !== null) window.clearTimeout(firedHandle)
})

const presets = computed(() =>
  HOTKEY_PRESETS.filter((preset) => preset !== store.settings?.hotkey),
)

const isNetworkCamera = computed(() => store.settings?.camera_source === 'network')

watch(
  () => store.settings?.camera_url,
  (value) => {
    if (value !== undefined && document.activeElement?.id !== 'camera-url') {
      cameraUrl.value = value
    }
  },
  { immediate: true },
)

function switchCamera(value: string) {
  void store.changeSetting('camera_source', value === 'network' ? 'network' : 'builtin')
}

function saveCameraUrl() {
  void store.changeSetting('camera_url', cameraUrl.value.trim())
}

const probe = ref<{ ok: boolean; text: string } | null>(null)
const probing = ref(false)

// --- Aktualisierung ---

const updateState = ref<'idle' | 'checking' | 'available' | 'installing' | 'current' | 'failed'>(
  'idle',
)
const updateText = ref('')
const updatePercent = ref(0)
/** Laufende Version - für die Meldung „ist aktuell". */
const version = ref('')

onMounted(async () => {
  version.value = await getVersion().catch(() => '')
})

async function lookForUpdate() {
  updateState.value = 'checking'
  try {
    const found = await checkForUpdate()
    if (found) {
      updateState.value = 'available'
      updateText.value = t('update.available', { version: found.version })
    } else {
      updateState.value = 'current'
      updateText.value = t('update.current', { version: version.value })
    }
  } catch (cause) {
    updateState.value = 'failed'
    updateText.value = t('update.failed', { error: String(cause) })
  }
}

async function applyUpdate() {
  updateState.value = 'installing'
  updatePercent.value = 0
  try {
    await installUpdate((percent) => (updatePercent.value = percent))
  } catch (cause) {
    updateState.value = 'failed'
    updateText.value = t('update.failed', { error: String(cause) })
  }
}

/** Einmaliger Verbindungstest, damit man die Adresse ohne Geste prüfen kann. */
async function testConnection() {
  probing.value = true
  probe.value = null
  try {
    probe.value = { ok: true, text: await testCameraUrl(cameraUrl.value.trim()) }
  } catch (cause) {
    probe.value = { ok: false, text: String(cause) }
  } finally {
    probing.value = false
  }
}

/** Kurzfassung für die eingeklappte Kopfzeile. */
const summary = computed(() => {
  const camera = isNetworkCamera.value
    ? t('settings.summaryNetwork')
    : t('settings.summaryBuiltin')
  return `${formatHotkey(store.settings?.hotkey)} · ${camera}`
})

watch(
  () => store.settings?.hotkey,
  (value) => {
    if (value && !recording.value) hotkey.value = value
  },
  { immediate: true },
)

const threshold = computed({
  get: () => store.settings?.confidence_threshold ?? 0.85,
  set: (value: number) => void store.changeSetting('confidence_threshold', String(value)),
})

/**
 * Das Erkennungsfenster hängt an der Bildquelle: eine Netzwerk-Kamera liefert
 * später, deshalb hat sie einen eigenen, längeren Wert. Beide liegen in den
 * Einstellungen, nicht im Code.
 */
const timeout = computed({
  get: () =>
    isNetworkCamera.value
      ? (store.settings?.overlay_timeout_network_ms ?? 4000)
      : (store.settings?.overlay_timeout_ms ?? 3000),
  set: (value: number) =>
    void store.changeSetting(
      isNetworkCamera.value ? 'overlay_timeout_network_ms' : 'overlay_timeout_ms',
      String(value),
    ),
})

const sound = computed({
  get: () => store.settings?.sound_cue ?? true,
  set: (value: boolean) => void store.changeSetting('sound_cue', value ? '1' : '0'),
})

/** Nimmt die nächste Tastenkombination auf und speichert sie. */
function captureHotkey(event: KeyboardEvent) {
  event.preventDefault()
  const shortcut = shortcutFromEvent(event)
  if (!shortcut) return

  recording.value = false
  void setHotkey(shortcut)
}

async function setHotkey(shortcut: string) {
  await store.changeSetting('hotkey', shortcut)
  // Bei Ablehnung gilt weiter der alte Wert aus dem Backend.
  hotkey.value = store.settings?.hotkey ?? shortcut
}
</script>

<template>
  <CollapsibleCard
    :title="t('settings.title')"
    :hint="t('settings.hint')"
    storage-key="settings"
    icon="sliders"
    :default-open="false"
    :summary="summary"
  >

    <div class="setting">
      <label>
        <Icon name="keyboard" :size="15" />
        {{ t('settings.hotkey') }}
        <span v-if="fired" class="fired">{{ t('settings.hotkeyFired') }}</span>
      </label>
      <div class="row">
        <input
          class="mono grow"
          :class="{ recording }"
          :value="recording ? '' : formatHotkey(hotkey)"
          readonly
          :placeholder="t('settings.hotkeyPlaceholder')"
          @focus="recording = true"
          @blur="recording = false"
          @keydown="captureHotkey"
        />
        <button @click="openOverlay()">
          <Icon name="record" :size="13" filled /> {{ t('settings.testNow') }}
        </button>
      </div>

      <div class="row presets">
        <span class="muted">{{ t('settings.presets') }}</span>
        <button v-for="preset in presets" :key="preset" class="ghost mono" @click="setHotkey(preset)">
          {{ formatHotkey(preset) }}
        </button>
        <button
          v-if="store.settings?.hotkey !== DEFAULT_HOTKEY"
          class="ghost"
          @click="setHotkey(DEFAULT_HOTKEY)"
        >
          {{ t('settings.standard') }}
        </button>
      </div>

      <p v-if="store.settings?.hotkey_error" class="warn">
        {{ store.settings.hotkey_error }}
      </p>
      <p class="note">
        {{
          t('settings.hotkeyNote', { macHint: isMac ? t('settings.hotkeyMacHint') : '' })
        }}
      </p>
    </div>

    <div class="setting">
      <label><Icon name="camera" :size="15" /> {{ t('settings.camera') }}</label>
      <select
        :value="store.settings?.camera_source ?? 'builtin'"
        @change="switchCamera(($event.target as HTMLSelectElement).value)"
      >
        <option value="builtin">{{ t('settings.cameraBuiltin') }}</option>
        <option value="network">{{ t('settings.cameraNetwork') }}</option>
      </select>

      <template v-if="isNetworkCamera">
        <input
          id="camera-url"
          v-model="cameraUrl"
          class="url mono"
          placeholder="http://192.168.1.20:4747/video"
          @keyup.enter="saveCameraUrl"
          @blur="saveCameraUrl"
        />
        <div class="row test">
          <button :disabled="probing || !cameraUrl.trim()" @click="testConnection">
            <Icon name="wifi" :size="14" />
            {{ probing ? t('settings.cameraTesting') : t('settings.cameraTest') }}
          </button>
          <span v-if="probe" :class="probe.ok ? 'ok' : 'warn'">{{ probe.text }}</span>
        </div>
        <p class="privacy">{{ t('settings.cameraPrivacy') }}</p>
        <p class="note">{{ t('settings.cameraNote') }}</p>
      </template>
      <p v-else class="note">{{ t('settings.cameraBuiltinNote') }}</p>
    </div>

    <div class="setting">
      <label>
        {{ t('settings.threshold') }}
        <span class="mono">{{ Math.round(threshold * 100) }} %</span>
      </label>
      <input v-model.number="threshold" type="range" min="0.5" max="0.99" step="0.01" />
      <p class="note">{{ t('settings.thresholdNote') }}</p>
    </div>

    <div class="setting">
      <label>
        {{ t('settings.window') }}
        <span class="mono">{{ (timeout / 1000).toFixed(1) }} s</span>
      </label>
      <input v-model.number="timeout" type="range" min="1500" max="6000" step="500" />
      <p class="note">
        {{
          t('settings.windowNote', {
            networkHint: isNetworkCamera ? t('settings.windowNetworkHint') : '',
          })
        }}
      </p>
    </div>

    <div class="setting">
      <label class="row inline">
        <input v-model="sound" type="checkbox" />
        {{ t('settings.sound') }}
      </label>
    </div>

    <div class="setting">
      <label>{{ t('update.title') }}</label>
      <div class="row">
        <button :disabled="updateState === 'checking' || updateState === 'installing'" @click="lookForUpdate">
          <Icon name="download" :size="14" />
          {{ updateState === 'checking' ? t('update.checking') : t('update.check') }}
        </button>
        <button v-if="updateState === 'available'" class="primary" @click="applyUpdate">
          {{ t('update.install') }}
        </button>
        <span v-if="updateState === 'installing'" class="muted">
          {{ t('update.installing', { percent: updatePercent }) }}
        </span>
      </div>

      <p v-if="updateText" :class="updateState === 'failed' ? 'warn' : 'note'">
        {{ updateText }}
      </p>
      <p class="note">{{ t('update.note') }}</p>
    </div>
  </CollapsibleCard>
</template>

<style scoped>
.setting {
  padding: 12px 0;
  border-top: 1px solid var(--border);
}

.setting:first-of-type {
  border-top: none;
  padding-top: 0;
}

label {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-weight: 550;
  font-size: 13px;
  margin-bottom: 6px;
}

label.inline {
  font-weight: 400;
  margin: 0;
}

select,
.url {
  width: 100%;
}

.grow {
  flex: 1;
  min-width: 0;
}

input.recording {
  border-color: var(--accent);
}

.presets {
  margin-top: 8px;
  flex-wrap: wrap;
  gap: 6px;
  font-size: 12px;
}

.fired {
  color: var(--success);
  font-weight: 600;
  font-size: 12px;
}

.warn {
  margin: 8px 0 0;
  color: var(--warning);
  font-size: 12px;
}

.url {
  margin-top: 8px;
  font-size: 12.5px;
}

.test {
  margin-top: 8px;
  align-items: flex-start;
  font-size: 12px;
}

.test .ok {
  color: var(--success);
}

.test .warn {
  color: var(--warning);
  margin: 0;
}

.test button,
.row button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.privacy {
  margin: 8px 0 0;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--text);
  font-size: 12px;
  line-height: 1.45;
}

input[type='range'] {
  width: 100%;
  padding: 0;
  border: none;
  background: transparent;
  accent-color: var(--accent);
}

.note {
  margin: 6px 0 0;
  color: var(--muted);
  font-size: 12px;
}
</style>
