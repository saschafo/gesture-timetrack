<script setup lang="ts">
/**
 * Einfacher Dialog über dem Fenster. Bewusst schlank gehalten - er muss nur
 * ein Formular tragen.
 *
 * Schließen per Esc, Klick auf den Hintergrund oder Kreuz. Der Inhalt steckt
 * in einem Slot und wird erst erzeugt, wenn der Dialog offen ist; ein
 * geschlossener Dialog hält also keinen Zustand fest.
 */
import { onBeforeUnmount, watch } from 'vue'

import { t } from '../i18n'
import Icon from './Icon.vue'

const props = defineProps<{
  open: boolean
  title: string
  /** Kurze Erklärung unter dem Titel. */
  hint?: string
}>()

const emit = defineEmits<{ close: [] }>()

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close')
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      window.addEventListener('keydown', onKeydown)
    } else {
      window.removeEventListener('keydown', onKeydown)
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="backdrop" @click.self="emit('close')">
      <div class="dialog" role="dialog" aria-modal="true" :aria-label="title">
        <header>
          <div>
            <h3>{{ title }}</h3>
            <p v-if="hint" class="hint">{{ hint }}</p>
          </div>
          <button class="ghost close" :title="t('common.close')" @click="emit('close')">
            <Icon name="x" :size="16" />
          </button>
        </header>
        <div class="body">
          <slot />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(12, 14, 22, 0.45);
  backdrop-filter: blur(2px);
}

.dialog {
  width: 100%;
  max-width: 460px;
  max-height: 100%;
  overflow: auto;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: 0 12px 40px rgba(12, 14, 22, 0.28);
  padding: 18px 20px 20px;
}

header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

h3 {
  margin: 0;
  font-size: 15.5px;
  font-weight: 650;
}

.hint {
  margin: 3px 0 0;
  color: var(--muted);
  font-size: 12.5px;
}

.close {
  color: var(--muted);
}

.body {
  margin-top: 14px;
}
</style>
