<script setup lang="ts">
/**
 * Formularzeile für einen Zeiteintrag - dieselbe Maske fürs Bearbeiten und fürs
 * Nachtragen vergessener Zeiten.
 *
 * Geprüft wird im Backend (`state::plan_entry`); hier steht nur, was der Nutzer
 * eintippt. So gibt es genau eine Stelle, die über Plausibilität entscheidet.
 */
import { ref, watch } from 'vue'

import type { EntryInput, Project, TimeEntry } from '../api/backend'
import { t } from '../i18n'
import Icon from './Icon.vue'

const props = defineProps<{
  projects: Project[]
  /** Vorhandener Eintrag beim Bearbeiten, sonst leer. */
  entry?: TimeEntry | null
  /** Projekt, das bei einem neuen Eintrag vorgeschlagen wird. */
  defaultProjectId?: number | null
  busy?: boolean
  /** Felder untereinander - für den Dialog. */
  vertical?: boolean
}>()

const emit = defineEmits<{
  save: [input: EntryInput]
  cancel: []
}>()

const projectId = ref<number | null>(null)
const start = ref('')
const end = ref('')
const pauseMinutes = ref(0)

/** „2026-08-19 09:37:00" -> „2026-08-19T09:37" für das Eingabefeld. */
function toInput(ts: string | null): string {
  if (!ts) return ''
  return ts.slice(0, 16).replace(' ', 'T')
}

function localNow(offsetMinutes = 0): string {
  const value = new Date(Date.now() + offsetMinutes * 60_000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}T${pad(
    value.getHours(),
  )}:${pad(value.getMinutes())}`
}

watch(
  () => props.entry,
  (entry) => {
    if (entry) {
      projectId.value = entry.project_id
      start.value = toInput(entry.start_ts)
      end.value = toInput(entry.end_ts)
      pauseMinutes.value = Math.round(entry.pause_duration_seconds / 60)
    } else {
      // Vorschlag für einen neuen Eintrag: die vergangene Stunde.
      projectId.value = props.defaultProjectId ?? props.projects[0]?.id ?? null
      start.value = localNow(-60)
      end.value = localNow()
      pauseMinutes.value = 0
    }
  },
  { immediate: true },
)

function save() {
  if (!projectId.value || !start.value || !end.value) return
  emit('save', {
    project_id: projectId.value,
    start: start.value,
    end: end.value,
    pause_minutes: Number(pauseMinutes.value) || 0,
  })
}
</script>

<template>
  <div class="editor" :class="{ vertical }">
    <label class="project">
      <span class="muted">{{ t('entry.project') }}</span>
      <select v-model="projectId">
        <option v-for="project in projects" :key="project.id" :value="project.id">
          {{ project.name }}
        </option>
      </select>
    </label>

    <div class="times">
      <label>
        <span class="muted">{{ t('entry.from') }}</span>
        <input v-model="start" type="datetime-local" />
      </label>
      <label>
        <span class="muted">{{ t('entry.to') }}</span>
        <input v-model="end" type="datetime-local" />
      </label>
      <label>
        <span class="muted">{{ t('entry.pause') }}</span>
        <input v-model="pauseMinutes" type="number" min="0" step="5" class="pause" />
      </label>
    </div>

    <div class="actions">
      <button :disabled="busy" @click="emit('cancel')">{{ t('common.cancel') }}</button>
      <button class="primary" :disabled="busy || !projectId" @click="save">
        <Icon name="check" :size="15" /> {{ t('common.save') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.editor {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 10px;
}

.editor.vertical {
  flex-direction: column;
  align-items: stretch;
}

.times {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.project select {
  width: 100%;
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.editor.vertical .actions {
  border-top: 1px solid var(--border);
  padding-top: 14px;
}

.actions button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 11.5px;
}

select {
  min-width: 160px;
}

.pause {
  width: 80px;
}
</style>
