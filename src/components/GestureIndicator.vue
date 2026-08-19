<script setup lang="ts">
/**
 * Visuelles Signal im Overlay: grüner Rahmen = Geste übernommen,
 * roter Rahmen = nicht erkannt bzw. nicht ausführbar.
 */
import { computed } from 'vue'

import type { IndicatorState } from './indicator-state'

const props = defineProps<{
  state: IndicatorState
  message: string
  /** Konfidenz der zuletzt gesehenen Handhaltung, 0 bis 1. */
  confidence: number
  /** Verbleibende Zeit bis zum Timeout, 1 bis 0. */
  remaining: number
}>()

const color = computed(() => {
  switch (props.state) {
    case 'accepted':
      return 'var(--success)'
    case 'rejected':
      return 'var(--danger)'
    default:
      return 'rgba(255, 255, 255, 0.25)'
  }
})
</script>

<template>
  <div class="indicator" :style="{ '--frame': color }">
    <div class="frame" :class="state"></div>
    <div class="bar" v-if="state === 'searching' || state === 'starting'">
      <span class="fill" :style="{ width: `${Math.round(remaining * 100)}%` }"></span>
      <span class="confidence" :style="{ width: `${Math.round(confidence * 100)}%` }"></span>
    </div>
    <p class="message" :class="state">{{ message }}</p>
  </div>
</template>

<style scoped>
.indicator {
  position: absolute;
  inset: 0;
  pointer-events: none;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
}

.frame {
  position: absolute;
  inset: 0;
  border: 3px solid var(--frame);
  border-radius: 10px;
  transition: border-color 0.12s ease;
}

.frame.accepted,
.frame.rejected {
  animation: pulse 0.35s ease;
}

@keyframes pulse {
  from {
    box-shadow: inset 0 0 0 0 var(--frame);
  }
  to {
    box-shadow: inset 0 0 28px -6px var(--frame);
  }
}

.bar {
  position: relative;
  height: 3px;
  margin: 0 3px;
  background: rgba(255, 255, 255, 0.12);
}

/* Oben die Restzeit, darunter dünn die aktuelle Konfidenz. */
.fill,
.confidence {
  position: absolute;
  left: 0;
  height: 100%;
  background: rgba(255, 255, 255, 0.5);
  transition: width 0.1s linear;
}

.confidence {
  height: 100%;
  background: var(--accent);
  opacity: 0.9;
}

.message {
  margin: 0;
  padding: 5px 8px 6px;
  font-size: 11.5px;
  line-height: 1.25;
  text-align: center;
  color: #fff;
  background: rgba(9, 11, 18, 0.82);
  border-radius: 0 0 8px 8px;
}

.message.accepted {
  color: #86efac;
}

.message.rejected {
  color: #fca5a5;
}
</style>
