<script setup lang="ts">
/**
 * Karte mit einklappbarem Inhalt. Der Zustand landet im `localStorage`, damit
 * das Fenster beim nächsten Start so aussieht wie beim Schließen.
 */
import { computed, ref, watch } from 'vue'

import Icon, { type IconName } from './Icon.vue'

const props = defineProps<{
  title: string
  hint?: string
  /** Schlüssel für das Merken des Zustands. */
  storageKey: string
  /** Zustand beim allerersten Öffnen. */
  defaultOpen?: boolean
  /** Kurzinfo rechts in der Kopfzeile, wenn eingeklappt. */
  summary?: string
  /** Symbol vor dem Titel. */
  icon?: IconName
}>()

const stored = localStorage.getItem(`card:${props.storageKey}`)
const open = ref(stored === null ? (props.defaultOpen ?? true) : stored === '1')

watch(open, (value) => localStorage.setItem(`card:${props.storageKey}`, value ? '1' : '0'))

const label = computed(() => (open.value ? 'Einklappen' : 'Ausklappen'))
</script>

<template>
  <section class="card" :class="{ collapsed: !open }">
    <button class="head" :aria-expanded="open" :title="label" @click="open = !open">
      <span class="chevron" :class="{ open }"><Icon name="chevron" :size="15" /></span>
      <Icon v-if="icon" :name="icon" :size="17" class="lead" />
      <!-- Kurzinfo unter den Titel, nicht daneben: in einer schmalen Karte
           würden beide sonst übereinanderliegen. -->
      <span class="titles">
        <span class="title">{{ title }}</span>
        <span v-if="hint && open" class="hint">{{ hint }}</span>
        <span v-if="summary && !open" class="summary muted">{{ summary }}</span>
      </span>
    </button>
    <!-- Bewusst v-if: eingeklappter Inhalt wird abgebaut. Damit schaltet z. B.
         die Kamera-Vorschau garantiert ab, statt unsichtbar weiterzulaufen. -->
    <div v-if="open" class="body">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.card.collapsed {
  padding-bottom: 12px;
}

.head {
  display: flex;
  align-items: baseline;
  gap: 8px;
  width: 100%;
  padding: 0;
  border: none;
  background: none;
  text-align: left;
  color: inherit;
}

.head:hover {
  background: none;
}

.chevron {
  display: inline-flex;
  color: var(--muted);
  transition: transform 0.15s ease;
}

.lead {
  color: var(--accent);
  align-self: center;
}

.chevron.open {
  transform: rotate(90deg);
}

.titles {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.title {
  font-size: 15px;
  font-weight: 650;
}

.hint {
  color: var(--muted);
  font-size: 12.5px;
  font-weight: 400;
}

.summary {
  font-size: 12.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.body {
  margin-top: 14px;
}
</style>
