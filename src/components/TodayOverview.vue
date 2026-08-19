<script setup lang="ts">
/** Tagesübersicht je Projekt, Einzelbuchungen und CSV-Export. */
import { computed, onMounted, ref, watch } from 'vue'
import { save } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'

import EntryEditor from './EntryEditor.vue'
import Icon from './Icon.vue'
import ModalDialog from './ModalDialog.vue'
import {
  createEntry,
  deleteEntry,
  exportCsv,
  formatDuration,
  formatHours,
  today,
  updateEntry,
  type EntryInput,
  type TimeEntry,
} from '../api/backend'
import { t } from '../i18n'
import { useTimetrackStore } from '../stores/timetrack'

const store = useTimetrackStore()

const from = ref(today())
const to = ref(today())
/** Projektfilter; `null` heißt „alle Projekte". */
const projectFilter = ref<number | null>(null)
const notice = ref<string | null>(null)
const problem = ref<string | null>(null)

/** Eintrag im Bearbeitungsmodus bzw. `'new'` für einen Nachtrag. */
const editing = ref<number | 'new' | null>(null)
const busy = ref(false)

/**
 * Wie viele Zeilen die Tabelle zeigt. Ein voller Monat hat schnell hundert
 * Einträge - die Liste soll die Seite nicht sprengen.
 */
const PAGE_SIZE = 20
const visibleCount = ref(PAGE_SIZE)

const visibleEntries = computed(() => store.entries.slice(0, visibleCount.value))
const hiddenCount = computed(() => Math.max(0, store.entries.length - visibleCount.value))
/** Lohnt eine Fußzeile mit Umschalter? */
const paginated = computed(() => store.entries.length > PAGE_SIZE)
const expanded = computed(() => visibleCount.value > PAGE_SIZE)

function showMore(all = false) {
  visibleCount.value = all ? store.entries.length : visibleCount.value + PAGE_SIZE
}

/** Zurück auf die kurze Liste - sonst wäre „Alle anzeigen" eine Einbahnstraße. */
function showLess() {
  visibleCount.value = PAGE_SIZE
}

const editingEntry = computed(() =>
  typeof editing.value === 'number'
    ? (store.entries.find((entry) => entry.id === editing.value) ?? null)
    : null,
)

/** Der offene Eintrag gehört der Zustandsmaschine, nicht dem Formular. */
function isOpen(entry: TimeEntry) {
  return !entry.end_ts
}

/** Gehört der Eintrag zur gerade laufenden bzw. pausierten Erfassung? */
function isActive(entry: TimeEntry) {
  return isOpen(entry) && entry.project_id === store.snapshot?.project_id
}

/**
 * Zustand des offenen Eintrags - „läuft" wäre bei einer Pause schlicht falsch.
 */
const openState = computed(() =>
  store.isPaused ? t('projects.paused') : t('projects.running'),
)

/**
 * Farbmarke vor dem Datum: normalerweise die Projektfarbe, beim aktiven Eintrag
 * grün bzw. orange. So ist auf einen Blick erkennbar, welche Zeile die laufende
 * ist - und ob sie pausiert.
 */
function markerColor(entry: TimeEntry) {
  if (isActive(entry)) {
    return store.isPaused ? 'var(--warning)' : 'var(--success)'
  }
  return (
    store.projects.find((project) => project.id === entry.project_id)?.color ?? 'var(--border)'
  )
}

function markerTitle(entry: TimeEntry) {
  return isActive(entry) ? `${entry.project_name} · ${openState.value}` : entry.project_name
}

function edit(entry: TimeEntry) {
  problem.value = null
  notice.value = null
  editing.value = entry.id
}

async function saveEntry(input: EntryInput) {
  busy.value = true
  problem.value = null
  try {
    if (editing.value === 'new') {
      await createEntry(input)
      notice.value = t('overview.entryAdded')
    } else if (typeof editing.value === 'number') {
      await updateEntry(editing.value, input)
      notice.value = t('overview.entryChanged')
    }
    editing.value = null
    await store.refresh()
    reload()
  } catch (cause) {
    problem.value = String(cause)
  } finally {
    busy.value = false
  }
}

async function remove(entry: TimeEntry) {
  problem.value = null
  notice.value = null
  busy.value = true
  try {
    await deleteEntry(entry.id)
    notice.value = t('overview.entryDeleted')
    await store.refresh()
    reload()
  } catch (cause) {
    problem.value = String(cause)
  } finally {
    busy.value = false
  }
}

const totalSeconds = computed(() =>
  store.entries.reduce((sum, entry) => sum + durationOf(entry), 0),
)

/** Pausenzeit des Zeitraums - in Minuten, wie sie auch eingegeben wird. */
const totalPauseMinutes = computed(() =>
  store.entries.reduce((sum, entry) => sum + pauseMinutes(entry), 0),
)

function pauseMinutes(entry: TimeEntry) {
  return Math.round(entry.pause_duration_seconds / 60)
}

const maxSeconds = computed(() =>
  Math.max(1, ...store.totals.map((total) => total.seconds)),
)

function reload() {
  void store.loadEntries(from.value, to.value, projectFilter.value)
}

/**
 * Alles, was die Liste veralten lässt. Der Status allein genügt nicht: beim
 * Wechsel von einem laufenden Projekt zum nächsten bleibt er „läuft", während
 * ein Eintrag geschlossen und ein neuer geöffnet wird.
 */
const dataKey = computed(() =>
  [
    store.snapshot?.status,
    store.snapshot?.project_id,
    store.snapshot?.today_seconds,
    store.projects.length,
  ].join('|'),
)

onMounted(reload)
watch([from, to, projectFilter], reload)
watch(dataKey, reload)
// Bei neuem Zeitraum oder Filter wieder oben anfangen.
watch([from, to, projectFilter], () => (visibleCount.value = PAGE_SIZE))

/** Name des gefilterten Projekts - für Dateiname und Anzeige. */
const filterName = computed(
  () => store.projects.find((project) => project.id === projectFilter.value)?.name ?? null,
)

/** Dateinamen aus Projektnamen bauen: nur unverfängliche Zeichen. */
function slug(value: string) {
  return value
    .toLowerCase()
    .replace(/[äöüß]/g, (match) => ({ ä: 'ae', ö: 'oe', ü: 'ue', ß: 'ss' })[match] ?? match)
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
}

async function exportRange() {
  notice.value = null
  const project = filterName.value ? `_${slug(filterName.value)}` : ''
  const path = await save({
    title: t('overview.exportTitle'),
    defaultPath: `zeiten${project}_${from.value}_bis_${to.value}.csv`,
    filters: [{ name: 'CSV', extensions: ['csv'] }],
  })
  if (!path) return

  const rows = await exportCsv(path, from.value, to.value, projectFilter.value)
  notice.value =
    rows === 1 ? t('overview.exportedOne') : t('overview.exported', { count: rows })
  await revealItemInDir(path)
}

function timeOnly(ts: string | null) {
  return ts ? ts.slice(11, 16) : '–'
}

/**
 * Beim noch offenen Eintrag steht in der Datenbank noch keine Dauer - dort die
 * laufende Zeit anzeigen statt 00:00:00.
 */
function durationOf(entry: TimeEntry) {
  if (entry.end_ts) return entry.duration_seconds
  return entry.project_id === store.snapshot?.project_id ? store.elapsedSeconds : 0
}
</script>

<template>
  <section class="card report">
    <div class="head">
      <div class="titles">
        <h2><Icon name="chart" :size="17" class="lead" /> {{ t('overview.title') }}</h2>
        <p class="hint">{{ t('overview.hint') }}</p>
      </div>
      <div class="toolbar">
        <select v-model="projectFilter" class="filter" :title="t('overview.filterTitle')">
          <option :value="null">{{ t('overview.allProjects') }}</option>
          <option v-for="project in store.projects" :key="project.id" :value="project.id">
            {{ project.name }}
          </option>
        </select>
        <input v-model="from" type="date" />
        <span class="muted">{{ t('common.to') }}</span>
        <input v-model="to" type="date" />
        <button :disabled="!store.activeProjects.length" @click="editing = 'new'">
          <Icon name="plus" :size="14" /> {{ t('overview.addEntry') }}
        </button>
        <button @click="exportRange">
          <Icon name="download" :size="14" /> {{ t('overview.export') }}
        </button>
      </div>
    </div>

    <p v-if="notice" class="notice">{{ notice }}</p>
    <p v-if="problem && editing === null" class="problem card-problem">{{ problem }}</p>

    <p class="section muted">{{ t('overview.todayPerProject') }}</p>
    <div class="totals">
      <div v-for="total in store.totals" :key="total.project_id ?? 0" class="total">
        <div class="row spread">
          <span>{{ total.project_name }}</span>
          <span class="mono">{{ formatDuration(total.seconds) }}</span>
        </div>
        <div class="bar">
          <span
            :style="{
              width: `${(total.seconds / maxSeconds) * 100}%`,
              background: total.color,
            }"
          ></span>
        </div>
      </div>
      <p v-if="!store.totals.length" class="muted">{{ t('overview.nothingToday') }}</p>
    </div>

    <table class="entries">
      <!-- Feste Spaltenbreiten: das Datum braucht wenig, der Projektname viel. -->
      <colgroup>
        <col class="col-marker" />
        <col class="col-date" />
        <col />
        <col class="col-time" />
        <col class="col-time" />
        <col class="col-pause" />
        <col class="col-duration" />
        <col class="col-hours" />
        <col class="col-source" />
        <col class="col-actions" />
      </colgroup>
      <thead>
        <tr>
          <th></th>
          <th>{{ t('overview.colDate') }}</th>
          <th>{{ t('overview.colProject') }}</th>
          <th>{{ t('overview.colFrom') }}</th>
          <th>{{ t('overview.colTo') }}</th>
          <th class="right">{{ t('overview.colPause') }}</th>
          <th class="right">{{ t('overview.colDuration') }}</th>
          <th class="right">{{ t('overview.colHours') }}</th>
          <th>{{ t('overview.colSource') }}</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <template v-for="entry in visibleEntries" :key="entry.id">
          <tr :class="{ active: isActive(entry) }">
            <td class="marker">
              <span
                :class="{ pulse: isActive(entry), paused: isActive(entry) && store.isPaused }"
                :style="{ background: markerColor(entry) }"
                :title="markerTitle(entry)"
              ></span>
            </td>
            <td>{{ entry.start_ts.slice(0, 10) }}</td>
            <td>{{ entry.project_name }}</td>
            <td class="mono">{{ timeOnly(entry.start_ts) }}</td>
            <td class="mono">
              {{ entry.end_ts ? timeOnly(entry.end_ts) : openState }}
            </td>
            <td class="mono right" :class="{ muted: !pauseMinutes(entry) }">
              {{ pauseMinutes(entry) ? t('overview.minutes', { count: pauseMinutes(entry) }) : '–' }}
            </td>
            <td class="mono right">{{ formatDuration(durationOf(entry)) }}</td>
            <td class="mono right">{{ formatHours(durationOf(entry)) }}</td>
            <td class="muted">
              {{ entry.gesture_triggered ? t('overview.sourceGesture') : t('overview.sourceManual') }}
            </td>
            <td class="actions">
              <template v-if="isOpen(entry)">
                <span class="muted">{{ t('overview.stopFirst') }}</span>
              </template>
              <template v-else>
                <button class="ghost" :title="t('common.edit')" :disabled="busy" @click="edit(entry)">
                  <Icon name="edit" :size="15" />
                </button>
                <button
                  class="ghost danger"
                  :title="t('common.delete')"
                  :disabled="busy"
                  @click="remove(entry)"
                >
                  <Icon name="trash" :size="15" />
                </button>
              </template>
            </td>
          </tr>
        </template>

        <tr v-if="!store.entries.length">
          <td colspan="10" class="muted">
            {{
              filterName
                ? t('overview.emptyFiltered', { name: filterName })
                : t('overview.empty')
            }}
          </td>
        </tr>

        <tr v-if="paginated">
          <!-- Der Flex-Container gehört in die Zelle, nicht auf sie: eine Zelle
               mit display:flex ist keine Tabellenzelle mehr und schrumpft bei
               festem Tabellenlayout auf die erste Spaltenbreite. -->
          <td colspan="10">
            <div class="more">
              <span class="muted">
                {{
                  t('overview.pageInfo', {
                    shown: visibleEntries.length,
                    total: store.entries.length,
                  })
                }}
              </span>
              <button v-if="hiddenCount" class="ghost" @click="showMore()">
                <Icon name="plus" :size="13" />
                {{ t('overview.showMore', { count: Math.min(hiddenCount, PAGE_SIZE) }) }}
              </button>
              <button v-if="hiddenCount" class="ghost" @click="showMore(true)">
                {{ t('overview.showAll') }}
              </button>
              <button v-if="expanded" class="ghost" @click="showLess">
                <Icon name="chevron" :size="13" class="up" />
                {{ t('overview.showLess') }}
              </button>
            </div>
          </td>
        </tr>
      </tbody>
      <tfoot v-if="store.entries.length">
        <tr>
          <td colspan="5">{{ t('overview.sum') }}</td>
          <td class="mono right">
            {{ totalPauseMinutes ? t('overview.minutes', { count: totalPauseMinutes }) : '–' }}
          </td>
          <td class="mono right">{{ formatDuration(totalSeconds) }}</td>
          <td class="mono right">{{ formatHours(totalSeconds) }}</td>
          <td colspan="2"></td>
        </tr>
      </tfoot>
    </table>
    <ModalDialog
      :open="editing !== null"
      :title="editing === 'new' ? t('overview.addTitle') : t('overview.editTitle')"
      :hint="t('overview.modalHint')"
      @close="editing = null"
    >
      <p v-if="problem" class="problem">{{ problem }}</p>
      <EntryEditor
        :projects="store.activeProjects"
        :entry="editingEntry"
        :default-project-id="store.snapshot?.project_id ?? null"
        :busy="busy"
        vertical
        @save="saveEntry"
        @cancel="editing = null"
      />
    </ModalDialog>
  </section>
</template>

<style scoped>
.report {
  /* Maßgeblich ist die Kartenbreite: die Karte nimmt zwei Drittel des Rasters
     ein, nicht die Fensterbreite. */
  container-type: inline-size;
}

.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
}

.titles {
  min-width: 0;
}

.toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.toolbar button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
}

/* Datumsfelder brauchen ihre Breite, sonst schneidet der Webview die Jahreszahl ab. */
.toolbar input[type='date'] {
  min-width: 136px;
}

/* Schmale Karte: Werkzeuge unter den Titel, statt ihn zu quetschen. */
@container (max-width: 860px) {
  .head {
    flex-direction: column;
    align-items: stretch;
  }
}

.section {
  margin: 14px 0 4px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.totals {
  margin: 0 0 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.bar {
  height: 6px;
  border-radius: 999px;
  background: var(--surface-2);
  overflow: hidden;
  margin-top: 4px;
}

.bar span {
  display: block;
  height: 100%;
  border-radius: 999px;
}

.notice {
  margin: 10px 0 0;
  color: var(--success);
  font-size: 12.5px;
}

.problem {
  margin: 0 0 10px;
}

.card-problem {
  margin: 10px 0 0;
  color: var(--danger);
  font-size: 12.5px;
}

.more {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  padding: 10px 0 2px;
  font-size: 12.5px;
}

.more span,
.more button {
  white-space: nowrap;
}

.more button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

/* Pfeil nach oben: dasselbe Symbol, gedreht. */
.more .up {
  transform: rotate(-90deg);
}

.actions {
  white-space: nowrap;
  text-align: right;
}

.actions .danger {
  color: var(--danger);
}

.lead {
  color: var(--accent);
}

.row button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.entries {
  width: 100%;
  table-layout: fixed;
  border-collapse: collapse;
  font-size: 13px;
}

.col-marker {
  width: 20px;
}

.col-date {
  width: 92px;
}

/* Farbmarke: Projektfarbe, beim aktiven Eintrag grün bzw. orange. */
.marker span {
  display: block;
  width: 9px;
  height: 9px;
  border-radius: 3px;
}

/* Der aktive Eintrag pulsiert - langsam und nur in der Deckkraft. Ein hartes
   Blinken wäre bei einer Zeile, die dauerhaft sichtbar ist, unruhig. */
.marker span.pulse {
  animation: pulse 1.7s ease-in-out infinite;
}

/* Pausiert: längerer Takt mit deutlicher Ruhephase - das liest sich als Warten. */
.marker span.pulse.paused {
  animation-name: pulse-paused;
  animation-duration: 2.6s;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.3;
  }
}

@keyframes pulse-paused {
  0%,
  30% {
    opacity: 1;
  }
  60%,
  100% {
    opacity: 0.25;
  }
}

/* Wer Bewegung im System abbestellt hat, bekommt eine ruhige Marke. */
@media (prefers-reduced-motion: reduce) {
  .marker span.pulse {
    animation: none;
  }
}

.entries tr.active td {
  background: var(--surface-2);
}

.col-time {
  width: 58px;
}

.col-pause {
  width: 74px;
}

.col-duration {
  width: 84px;
}

.col-hours {
  width: 66px;
}

.col-source {
  width: 72px;
}

.col-actions {
  width: 78px;
}

.filter {
  max-width: 190px;
}

.entries th {
  text-align: left;
  font-weight: 600;
  color: var(--muted);
  font-size: 12px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
  /* Kopfzeilen bleiben einzeilig - ein Umbruch mitten in der Beschriftung
     verschiebt die ganze Zeile. */
  white-space: nowrap;
}

.entries td {
  padding: 6px 8px 6px 0;
  border-bottom: 1px solid var(--border);
}

.entries tfoot td {
  font-weight: 600;
  border-bottom: none;
}

.right {
  text-align: right;
}
</style>
