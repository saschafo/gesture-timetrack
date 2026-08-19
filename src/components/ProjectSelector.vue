<script setup lang="ts">
/**
 * Projektverwaltung und Zuordnung der beiden Gesten-Slots.
 * Slot 1 = ein Finger, Slot 2 = zwei Finger.
 */
import { ref } from 'vue'

import type { Project } from '../api/backend'
import { projectButtonStyle } from '../api/colors'
import { t } from '../i18n'
import Icon from './Icon.vue'
import { useTimetrackStore } from '../stores/timetrack'

const store = useTimetrackStore()

const PALETTE = ['#4f46e5', '#0891b2', '#16a34a', '#d97706', '#db2777', '#7c3aed']

const newName = ref('')
const newColor = ref(PALETTE[0])
const editing = ref<number | null>(null)
const editName = ref('')
const editColor = ref('#4f46e5')

async function add() {
  const name = newName.value.trim()
  if (!name) return
  await store.addProject(name, newColor.value)
  newName.value = ''
  newColor.value = PALETTE[store.projects.length % PALETTE.length]
}

function beginEdit(project: Project) {
  editing.value = project.id
  editName.value = project.name
  editColor.value = project.color
}

async function saveEdit(project: Project) {
  await store.editProject(project.id, editName.value, editColor.value, project.active)
  editing.value = null
}

/** Läuft oder pausiert die Erfassung für dieses Projekt? */
function isCurrent(project: Project) {
  return store.snapshot?.project_id === project.id
}

/**
 * Wechseln ist auch aus einer Pause heraus möglich: der laufende Eintrag wird
 * abgeschlossen, das neue Projekt beginnt sofort.
 */
function startOrSwitch(project: Project) {
  void store.start(project.id)
}

function toggleActive(project: Project) {
  void store.editProject(project.id, project.name, project.color, !project.active)
}

async function remove(project: Project) {
  const deleted = await store.removeProject(project.id)
  if (deleted === false) {
    // Projekte mit Zeiteinträgen bleiben als inaktiv erhalten.
    store.error = t('projects.keptInactive', { name: project.name })
  }
}
</script>

<template>
  <section class="card">
    <h2><Icon name="folder" :size="17" class="lead" /> {{ t('projects.title') }}</h2>
    <p class="hint">{{ t('projects.hint') }}</p>
    <p class="hint slots-hint">{{ t('gestures.switchHint') }}</p>

    <div class="slots">
      <label v-for="slot in store.snapshot?.slots ?? []" :key="slot.slot" class="slot">
        <span class="slot-name">
          <span
            class="dot"
            :class="{ pale: !slot.project_id }"
            :style="{ background: slot.project_color ?? 'var(--border)' }"
          ></span>
          <strong>{{ t('projects.slot', { slot: slot.slot }) }}</strong>
          <span class="muted">
            {{ slot.slot === 1 ? t('projects.oneFinger') : t('projects.twoFingers') }}
          </span>
        </span>
        <select
          :value="slot.project_id ?? ''"
          @change="
            store.assignSlot(
              slot.slot,
              ($event.target as HTMLSelectElement).value
                ? Number(($event.target as HTMLSelectElement).value)
                : null,
            )
          "
        >
          <option value="">{{ t('projects.unassigned') }}</option>
          <option v-for="project in store.activeProjects" :key="project.id" :value="project.id">
            {{ project.name }}
          </option>
        </select>
      </label>
    </div>

    <ul class="projects">
      <li v-for="project in store.projects" :key="project.id" :class="{ inactive: !project.active }">
        <template v-if="editing === project.id">
          <input v-model="editName" class="grow" @keyup.enter="saveEdit(project)" />
          <input v-model="editColor" type="color" />
          <button class="primary" @click="saveEdit(project)">{{ t('common.save') }}</button>
          <button class="ghost" @click="editing = null">{{ t('common.cancel') }}</button>
        </template>
        <template v-else>
          <span class="dot" :style="{ background: project.color }"></span>
          <span class="name">
            {{ project.name }}
            <span v-if="!project.active" class="muted">{{ t('projects.inactive') }}</span>
          </span>

          <!-- Nur Symbole: In einer schmalen Karte schoben sich die Wörter
               „läuft" und „Wechseln" über den Projektnamen. -->
          <span class="actions">
            <span
              v-if="isCurrent(project)"
              class="state"
              :class="store.isPaused ? 'paused' : 'running'"
              :title="store.isPaused ? t('projects.paused') : t('projects.running')"
            >
              <Icon :name="store.isPaused ? 'pause' : 'record'" :size="14" :filled="!store.isPaused" />
            </span>
            <button
              v-else-if="project.active"
              class="ghost accent"
              :style="projectButtonStyle(project.color) ?? undefined"
              :title="store.status === 'idle' ? t('projects.startTitle') : t('projects.switchTitle')"
              @click="startOrSwitch(project)"
            >
              <Icon :name="store.status === 'idle' ? 'play' : 'switch'" :size="14" />
            </button>

            <button class="ghost" :title="t('common.edit')" @click="beginEdit(project)">
              <Icon name="edit" :size="15" />
            </button>
            <button
              class="ghost"
              :title="project.active ? t('projects.deactivate') : t('projects.activate')"
              @click="toggleActive(project)"
            >
              <Icon :name="project.active ? 'x' : 'check'" :size="15" />
            </button>
            <button class="ghost danger" :title="t('common.delete')" @click="remove(project)">
              <Icon name="trash" :size="15" />
            </button>
          </span>
        </template>
      </li>
      <li v-if="!store.projects.length" class="muted empty">{{ t('projects.empty') }}</li>
    </ul>

    <div class="row add">
      <input
        v-model="newName"
        class="grow"
        :placeholder="t('projects.newPlaceholder')"
        @keyup.enter="add"
      />
      <input v-model="newColor" type="color" />
      <button class="primary" :disabled="!newName.trim()" @click="add">
        <Icon name="plus" :size="14" /> {{ t('projects.create') }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.slots-hint {
  margin-top: -8px;
}

.slots {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  margin-bottom: 16px;
}

.slot {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 5px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px 12px;
}

.slot-name {
  display: flex;
  align-items: baseline;
  gap: 6px;
  font-size: 12.5px;
  min-width: 0;
}

/* Auswahlfelder dürfen ihren Kasten nicht sprengen; die Höhe kommt aus der
   gemeinsamen Vorgabe (--control-h). */
.slot select {
  width: 100%;
  min-width: 0;
}

.projects {
  list-style: none;
  margin: 0 0 14px;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.projects li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}

.name {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: 13px;
}

/* Die Schaltflächen bleiben zusammen und schrumpfen nicht - sonst rutschen sie
   in den Projektnamen. */
.actions {
  display: flex;
  align-items: center;
  gap: 1px;
  flex: none;
}

.actions button {
  padding: 4px 5px;
}

/* Farbige Schaltfläche in Projektfarbe: als Symbolknopf ohne Text. */
.actions button.accent {
  border-radius: 7px;
  color: #fff;
  padding: 5px 7px;
}

.state {
  display: inline-flex;
  padding: 4px 6px;
}

.state.running {
  color: var(--success);
}

.state.paused {
  color: var(--warning);
}

.projects li.inactive {
  opacity: 0.6;
}

.projects li.empty {
  border-bottom: none;
  padding: 12px 0;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex: none;
}

.dot.pale {
  opacity: 0.35;
}

.grow {
  flex: 1;
  min-width: 0;
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

.add {
  margin-top: 4px;
}
</style>
