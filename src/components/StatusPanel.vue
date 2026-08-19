<script setup lang="ts">
/** Aktueller Erfassungsstand samt manueller Steuerung (Tastatur-Fallback). */
import { computed } from 'vue'

import { formatDuration } from '../api/backend'
import { projectButtonStyle } from '../api/colors'
import { formatHotkey } from '../api/hotkey'
import { GESTURE_SYMBOLS, LEGEND_GROUPS } from '../gesture/symbols'
import { t } from '../i18n'
import { useTimetrackStore } from '../stores/timetrack'
import Icon from './Icon.vue'

const store = useTimetrackStore()

const statusClass = computed(() => store.status)

/** Slot, den eine Start-Geste verwenden würde. */
const slotProject = computed(() => {
  const slot = store.snapshot?.active_slot
  return store.snapshot?.slots.find((entry) => entry.slot === slot) ?? null
})

/**
 * Solange nichts läuft, zeigt die Karte das vorgemerkte Projekt des aktiven
 * Slots - sonst sähe der Kopf für jedes Projekt gleich aus, obwohl eine
 * Start-Geste sehr wohl ein bestimmtes Projekt erfassen würde.
 */
const projectName = computed(
  () =>
    store.snapshot?.project_name ?? slotProject.value?.project_name ?? t('status.noProject'),
)

const projectColor = computed(
  () => store.snapshot?.project_color ?? slotProject.value?.project_color ?? 'var(--border)',
)

const hasProject = computed(
  () => Boolean(store.snapshot?.project_id ?? slotProject.value?.project_id),
)

/** Zusatz zum Status, damit „gestoppt" nicht mit „läuft" verwechselt wird. */
const slotHint = computed(() => {
  if (store.status !== 'idle') return null
  const slot = slotProject.value
  return slot?.project_id ? t('status.slotPending', { slot: slot.slot }) : null
})

/** Hauptschaltfläche in Projektfarbe - Textfarbe wird mitgerechnet. */
const actionStyle = computed(() => projectButtonStyle(projectColor.value) ?? undefined)

function startFromSlot() {
  const projectId = slotProject.value?.project_id
  if (projectId) void store.start(projectId)
}
</script>

<template>
  <section class="card status" :style="{ '--project': projectColor }">
    <span class="accent"></span>
    <div class="row spread">
      <div class="row">
        <span class="dot" :class="{ pale: !hasProject }"></span>
        <div>
          <h2>{{ projectName }}</h2>
          <p class="hint" style="margin: 0">
            <span class="badge" :class="statusClass">{{ store.snapshot?.status_label }}</span>
            <span v-if="store.snapshot?.pause_seconds" class="muted">
              · {{ t('status.break', { time: formatDuration(store.snapshot.pause_seconds) }) }}
            </span>
            <span v-else-if="slotHint" class="muted">· {{ slotHint }}</span>
          </p>
        </div>
      </div>
      <div class="times">
        <div class="clock mono">{{ formatDuration(store.elapsedSeconds) }}</div>
        <div class="muted">
          {{ t('status.today', { time: formatDuration(store.todaySeconds) }) }}
        </div>
      </div>
    </div>

    <div class="lower">
      <div class="controls">
        <button
          v-if="store.status === 'idle'"
          class="primary"
          :style="slotProject?.project_id ? actionStyle : undefined"
          :disabled="!slotProject?.project_id"
          @click="startFromSlot"
        >
          <Icon name="play" :size="15" />
          {{ t('status.start', { name: slotProject?.project_name ?? t('status.noSlot') }) }}
        </button>
        <button v-if="store.isRunning" @click="store.pause()">
          <Icon name="pause" :size="15" /> {{ t('status.pause') }}
        </button>
        <button v-if="store.isPaused" class="primary" :style="actionStyle" @click="store.resume()">
          <Icon name="play" :size="15" /> {{ t('status.resume') }}
        </button>
        <button v-if="store.status !== 'idle'" @click="store.stop()">
          <Icon name="stop" :size="15" /> {{ t('status.stop') }}
        </button>
      </div>

      <!-- Gestenübersicht als Gegensatzpaare: Start/Stopp, Pause/Weiter,
           Slot 1/Slot 2. Sie ist reine Nachschlage-Information und braucht keine
           eigene Karte - hier füllt sie den freien Raum neben der Uhr. -->
      <div class="legend">
        <div class="pairs" :title="t('gestures.hint')">
          <div v-for="(group, index) in LEGEND_GROUPS" :key="index" class="pair">
            <span
              v-for="entry in group"
              :key="entry.action"
              class="item"
              :title="t(`gesture.${entry.gesture}`)"
            >
              <span class="symbol">{{ GESTURE_SYMBOLS[entry.gesture] }}</span>
              {{ t(entry.action) }}
            </span>
          </div>
        </div>

        <p v-if="store.settings?.hotkey_active === false" class="warn">
          {{ t('status.hotkeyInactive') }}
        </p>
        <p v-else class="muted hotkey">
          <Icon name="keyboard" :size="14" />
          {{ t('status.hotkey') }} <kbd>{{ formatHotkey(store.settings?.hotkey) }}</kbd>
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.status {
  position: relative;
  overflow: hidden;
}

.status h2 {
  margin: 0;
  font-size: 17px;
}

/* Farbstreifen am Rand: die Projektfarbe ist so auch aus dem Augenwinkel
   erkennbar, nicht nur am kleinen Punkt. */
.accent {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  background: var(--project);
}

.dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex: none;
  background: var(--project);
}

.dot.pale {
  opacity: 0.35;
}

.times {
  text-align: right;
}

.clock {
  font-size: 30px;
  font-weight: 600;
  letter-spacing: -0.5px;
}

.badge {
  display: inline-block;
  border-radius: 999px;
  padding: 1px 9px;
  font-size: 11.5px;
  font-weight: 600;
  background: var(--surface-2);
  color: var(--muted);
}

.badge.running {
  background: color-mix(in srgb, var(--success) 16%, transparent);
  color: var(--success);
}

.badge.paused {
  background: color-mix(in srgb, var(--warning) 18%, transparent);
  color: var(--warning);
}

.lower {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 16px;
  margin-top: 18px;
}

.controls {
  display: flex;
  gap: 10px;
}

.controls button {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}

.legend {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
}

.pairs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

/* Ein Paar je Kästchen - die Zusammengehörigkeit ist so ohne Worte sichtbar. */
.pair {
  display: flex;
  gap: 12px;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface-2);
  font-size: 12px;
}

.item {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  white-space: nowrap;
}

.symbol {
  font-size: 14px;
  display: inline-block;
}

.warn {
  margin: 0;
  color: var(--warning);
  font-size: 12.5px;
}

.hotkey {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-size: 12.5px;
}

@media (max-width: 900px) {
  .legend {
    align-items: flex-start;
  }
}
</style>
