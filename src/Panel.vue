<script setup lang="ts">
/**
 * Fenster unter dem Symbol in der Menüleiste.
 *
 * Absicht: die häufigen Handgriffe ohne Hauptfenster - laufende Zeit sehen,
 * starten, pausieren, stoppen, Projekt wechseln. Alles andere (Auswertung,
 * Training, Einstellungen) bleibt dem Hauptfenster.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { closePanel, formatDuration, openMainWindow, openOverlay, resizePanel } from './api/backend'
import { projectButtonStyle } from './api/colors'
import { formatHotkey } from './api/hotkey'
import { t } from './i18n'
import Icon from './components/Icon.vue'
import { useTimetrackStore } from './stores/timetrack'

const store = useTimetrackStore()
const content = ref<HTMLElement | null>(null)

const projectName = computed(
  () => store.snapshot?.project_name ?? slotProject.value?.project_name ?? t('status.noProject'),
)

const slotProject = computed(() => {
  const slot = store.snapshot?.active_slot
  return store.snapshot?.slots.find((entry) => entry.slot === slot) ?? null
})

const projectColor = computed(
  () => store.snapshot?.project_color ?? slotProject.value?.project_color ?? 'var(--border)',
)

const actionStyle = computed(() => projectButtonStyle(projectColor.value) ?? undefined)

/** Slot-Nummer eines Projekts, für die Kurzinfo in der Liste. */
function slotOf(projectId: number) {
  return store.snapshot?.slots.find((slot) => slot.project_id === projectId)?.slot ?? null
}

const isCurrent = (projectId: number) => store.snapshot?.project_id === projectId

async function start() {
  const projectId = slotProject.value?.project_id
  if (projectId) await store.start(projectId)
}

/** Nach dem Wechsel schließen - das Fenster hat seinen Zweck dann erfüllt. */
async function switchTo(projectId: number) {
  await store.start(projectId)
  void closePanel()
}

/** Die Höhe folgt dem Inhalt, damit die Projektliste nicht abgeschnitten wird. */
function fitHeight() {
  const height = content.value?.scrollHeight
  if (height) void resizePanel(height + 2)
}

onMounted(async () => {
  await store.init()
  fitHeight()
})

watch(() => [store.activeProjects.length, store.status], fitHeight)

onBeforeUnmount(() => store.dispose())
</script>

<template>
  <div ref="content" class="panel">
    <div class="head" :style="{ '--project': projectColor }">
      <span class="dot"></span>
      <div class="titles">
        <strong>{{ projectName }}</strong>
        <span class="muted">{{ store.snapshot?.status_label }}</span>
      </div>
      <span class="clock mono">{{ formatDuration(store.elapsedSeconds) }}</span>
    </div>

    <div class="actions">
      <button
        v-if="store.status === 'idle'"
        class="primary"
        :style="slotProject?.project_id ? actionStyle : undefined"
        :disabled="!slotProject?.project_id"
        @click="start"
      >
        <Icon name="play" :size="15" /> {{ t('projects.start') }}
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

    <p class="label muted">
      {{ store.status === 'idle' ? t('panel.startProject') : t('panel.switchProject') }}
    </p>
    <ul class="projects">
      <li v-for="project in store.activeProjects" :key="project.id">
        <button class="row-button" :disabled="isCurrent(project.id)" @click="switchTo(project.id)">
          <span class="dot small" :style="{ background: project.color }"></span>
          <span class="name">{{ project.name }}</span>
          <span v-if="slotOf(project.id)" class="slot muted">
            {{ t('panel.slot', { slot: slotOf(project.id) ?? 0 }) }}
          </span>
          <span v-if="isCurrent(project.id)" class="muted">
            {{ store.isPaused ? t('projects.paused') : t('projects.running') }}
          </span>
          <Icon v-else name="chevron" :size="14" class="muted" />
        </button>
      </li>
      <li v-if="!store.activeProjects.length" class="empty muted">
        {{ t('panel.noProjects') }}
      </li>
    </ul>

    <div class="foot">
      <span class="muted">
        {{ t('common.today') }} {{ formatDuration(store.todaySeconds) }}
      </span>
      <span class="grow"></span>
      <button class="ghost" :title="t('panel.captureGesture')" @click="openOverlay()">
        <Icon name="hand" :size="15" />
      </button>
      <button class="ghost" :title="t('panel.openWindow')" @click="openMainWindow()">
        <Icon name="external" :size="15" />
      </button>
    </div>

    <p v-if="store.settings?.hotkey_active === false" class="hint warn">
      {{ t('panel.hotkeyInactive') }}
    </p>
    <p v-else class="hint muted">
      {{ t('panel.gesture') }} <kbd>{{ formatHotkey(store.settings?.hotkey) }}</kbd>
    </p>
  </div>
</template>

<style scoped>
.panel {
  padding: 12px 14px 12px;
  background: var(--surface);
}

.head {
  display: flex;
  align-items: center;
  gap: 9px;
}

.titles {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  line-height: 1.3;
}

.titles strong {
  font-size: 13.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.titles .muted {
  font-size: 11.5px;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--project);
  flex: none;
}

.dot.small {
  width: 8px;
  height: 8px;
}

.clock {
  font-size: 15px;
  font-weight: 600;
}

.actions {
  display: flex;
  gap: 6px;
  margin: 12px 0 4px;
}

.actions button {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 8px;
  font-size: 12.5px;
}

.label {
  margin: 12px 0 4px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.projects {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 260px;
  overflow-y: auto;
}

.row-button {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 8px;
  border: none;
  border-radius: 8px;
  background: none;
  font-size: 12.5px;
  text-align: left;
}

.row-button:hover:not(:disabled) {
  background: var(--surface-2);
}

.row-button:disabled {
  opacity: 1;
}

.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.slot {
  font-size: 11px;
}

.empty {
  padding: 8px;
  font-size: 12px;
}

.foot {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 10px;
  padding-top: 9px;
  border-top: 1px solid var(--border);
  font-size: 12px;
}

.foot button {
  padding: 4px 6px;
}

.grow {
  flex: 1;
}

.hint {
  margin: 7px 0 0;
  font-size: 11px;
}

.hint.warn {
  color: var(--warning);
}
</style>
