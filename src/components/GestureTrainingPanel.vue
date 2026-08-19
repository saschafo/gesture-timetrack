<script setup lang="ts">
/**
 * Bedienteil zum Einlernen der Gesten. Die Aufnahme selbst macht die
 * Kamera-Vorschau, weil dort die Bilder laufen - hier wird nur ausgelöst und
 * angezeigt.
 */
import { computed, ref } from 'vue'

import type { ActionOption, CustomGesture, Project } from '../api/backend'
import { customLabel } from '../gesture/gesture-classifier'
import { GESTURE_ORDER, GESTURE_SYMBOLS } from '../gesture/symbols'
import { MIN_SAMPLES } from '../gesture/trained-classifier'
import { t } from '../i18n'
import Icon from './Icon.vue'

const props = defineProps<{
  /** Aufnahmen je Kennung (Grundgeste oder `custom:<id>`). */
  counts: Record<string, number>
  /** Selbst definierte Gesten. */
  custom: CustomGesture[]
  /** Auswahlliste der möglichen Aktionen. */
  actions: ActionOption[]
  projects: Project[]
  /** Sind alle Gesten ausreichend eingelernt? */
  complete: boolean
  useTraining: boolean
  /** Läuft die Kamera-Vorschau? Ohne Bild keine Aufnahme. */
  ready: boolean
  /** Kennung der Geste, die gerade aufgenommen wird. */
  active: string | null
  /** Hinweistext während der Aufnahme (Countdown o. ä.). */
  status: string | null
}>()

const emit = defineEmits<{
  record: [label: string]
  clear: [label?: string]
  toggle: [value: boolean]
  createCustom: [name: string, action: string, projectId: number | null]
  updateCustom: [id: number, name: string, action: string, projectId: number | null]
  deleteCustom: [id: number]
}>()

// --- Eigene Gesten ---

const newName = ref('')
const newAction = ref('stop')
const newProject = ref<number | null>(null)

const needsProject = (action: string) =>
  props.actions.find((option) => option.key === action)?.needs_project ?? false

const actionLabel = (action: string) =>
  props.actions.find((option) => option.key === action)?.label ?? action

function addCustom() {
  const name = newName.value.trim()
  if (!name) return
  emit('createCustom', name, newAction.value, needsProject(newAction.value) ? newProject.value : null)
  newName.value = ''
}

function changeAction(gesture: CustomGesture, action: string) {
  emit('updateCustom', gesture.id, gesture.name, action, needsProject(action) ? gesture.project_id : null)
}

function changeProject(gesture: CustomGesture, projectId: number | null) {
  emit('updateCustom', gesture.id, gesture.name, gesture.action, projectId)
}

const total = computed(() => Object.values(props.counts).reduce((sum, value) => sum + value, 0))
</script>

<template>
  <div class="training">
    <p class="note muted">{{ t('training.intro') }}</p>

    <ul>
      <li v-for="gesture in GESTURE_ORDER" :key="gesture" :class="{ busy: active === gesture }">
        <span class="symbol">{{ GESTURE_SYMBOLS[gesture] }}</span>
        <span class="name">
          {{ t(`gesture.${gesture}`) }}
          <span class="muted">{{ t(`gestureAction.${gesture}`) }}</span>
        </span>

        <span class="count mono" :class="{ ok: (counts[gesture] ?? 0) >= MIN_SAMPLES }">
          {{ counts[gesture] ?? 0 }}/{{ MIN_SAMPLES }}
        </span>
        <button :disabled="!ready || active !== null" @click="emit('record', gesture)">
          <Icon name="record" :size="12" filled />
          {{ (counts[gesture] ?? 0) > 0 ? t('training.recordAgain') : t('training.record') }}
        </button>
        <button
          class="ghost"
          :disabled="!counts[gesture] || active !== null"
          :title="t('training.deleteSamples')"
          @click="emit('clear', gesture)"
        >
          <Icon name="trash" :size="14" />
        </button>
      </li>
    </ul>

    <div class="own">
      <h3><Icon name="hand" :size="15" class="lead" /> {{ t('training.ownTitle') }}</h3>
      <p class="note muted">{{ t('training.ownHint') }}</p>

      <ul>
        <li v-for="gesture in custom" :key="gesture.id" :class="{ busy: active === `custom:${gesture.id}` }">
          <span class="symbol">✋</span>
          <span class="name">
            {{ gesture.name }}
            <span class="muted">{{ actionLabel(gesture.action) }}</span>
          </span>

          <select
            class="action"
            :value="gesture.action"
            @change="changeAction(gesture, ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="option in actions" :key="option.key" :value="option.key">
              {{ option.label }}
            </option>
          </select>
          <select
            v-if="needsProject(gesture.action)"
            class="action"
            :value="gesture.project_id ?? ''"
            @change="
              changeProject(
                gesture,
                ($event.target as HTMLSelectElement).value
                  ? Number(($event.target as HTMLSelectElement).value)
                  : null,
              )
            "
          >
            <option value="">{{ t('training.chooseProject') }}</option>
            <option v-for="project in projects" :key="project.id" :value="project.id">
              {{ project.name }}
            </option>
          </select>

          <span
            class="count mono"
            :class="{ ok: (counts[customLabel(gesture.id)] ?? 0) >= MIN_SAMPLES }"
          >
            {{ counts[customLabel(gesture.id)] ?? 0 }}/{{ MIN_SAMPLES }}
          </span>
          <button
            :disabled="!ready || active !== null"
            @click="emit('record', customLabel(gesture.id))"
          >
            <Icon name="record" :size="12" filled />
            {{
              (counts[customLabel(gesture.id)] ?? 0) > 0
                ? t('training.recordAgain')
                : t('training.record')
            }}
          </button>
          <button
            class="ghost"
            :title="t('training.ownDelete')"
            :disabled="active !== null"
            @click="emit('deleteCustom', gesture.id)"
          >
            <Icon name="trash" :size="14" />
          </button>
        </li>
      </ul>

      <div class="row add">
        <input v-model="newName" :placeholder="t('training.ownName')" @keyup.enter="addCustom" />
        <select v-model="newAction">
          <option v-for="option in actions" :key="option.key" :value="option.key">
            {{ option.label }}
          </option>
        </select>
        <select v-if="needsProject(newAction)" v-model="newProject">
          <option :value="null">{{ t('training.chooseProject') }}</option>
          <option v-for="project in projects" :key="project.id" :value="project.id">
            {{ project.name }}
          </option>
        </select>
        <button class="primary" :disabled="!newName.trim()" @click="addCustom">
          <Icon name="plus" :size="14" /> {{ t('training.ownAdd') }}
        </button>
      </div>
    </div>

    <p v-if="status" class="status">{{ status }}</p>
    <p v-else-if="!ready" class="note muted">{{ t('training.needPreview') }}</p>

    <div class="row footer">
      <label class="row inline" :class="{ disabled: !complete }">
        <input
          type="checkbox"
          :checked="useTraining"
          :disabled="!complete"
          @change="emit('toggle', ($event.target as HTMLInputElement).checked)"
        />
        {{ t('training.use') }}
      </label>
      <span v-if="!complete" class="muted">{{ t('training.needComplete') }}</span>
      <span class="grow"></span>
      <button v-if="total" class="ghost danger" :disabled="active !== null" @click="emit('clear')">
        <Icon name="x" :size="13" /> {{ t('training.reset') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.training {
  /* Abgrenzung übernimmt die Vorschau-Karte, je nach Anordnung Linie links
     oder oben. */
  min-width: 0;
}

ul {
  list-style: none;
  margin: 10px 0;
  padding: 0;
}

li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 0;
}

li.busy {
  background: var(--accent-soft);
  border-radius: 8px;
  padding: 5px 6px;
  margin: 0 -6px;
}

.symbol {
  width: 22px;
  text-align: center;
  font-size: 16px;
}

.name {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  font-size: 13px;
  line-height: 1.25;
}

.name .muted {
  font-size: 11.5px;
}

.count {
  font-size: 11.5px;
  color: var(--muted);
}

.count.ok {
  color: var(--success);
}

.own {
  border-top: 1px solid var(--border);
  margin-top: 12px;
  padding-top: 12px;
}

.own h3 {
  margin: 0 0 4px;
  font-size: 13.5px;
  font-weight: 650;
}

.action {
  font-size: 12px;
  max-width: 150px;
}

.add {
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 4px;
}

.add input {
  flex: 1;
  min-width: 140px;
}

.status {
  margin: 0 0 10px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--accent);
}

.note {
  margin: 0 0 4px;
  font-size: 12px;
}

.footer {
  flex-wrap: wrap;
  gap: 6px;
  font-size: 12.5px;
}

label.inline {
  margin: 0;
  font-weight: 400;
  gap: 6px;
}

label.disabled {
  opacity: 0.55;
}

.grow {
  flex: 1;
}

.danger {
  color: var(--danger);
}

.lead {
  color: var(--accent);
}

button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>
