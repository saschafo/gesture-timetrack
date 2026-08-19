<script setup lang="ts">
/**
 * Icon-Satz als Inline-SVG.
 *
 * Bewusst selbst gezeichnet statt Icon-Bibliothek oder Icon-Font: nichts wird
 * nachgeladen, das Bundle bleibt klein, und die Strichstärke passt zur
 * Schriftfarbe (`currentColor`), also auch im dunklen Erscheinungsbild.
 */
import { computed } from 'vue'

export type IconName =
  | 'play'
  | 'pause'
  | 'stop'
  | 'switch'
  | 'plus'
  | 'edit'
  | 'trash'
  | 'download'
  | 'camera'
  | 'sliders'
  | 'chart'
  | 'folder'
  | 'keyboard'
  | 'wifi'
  | 'hand'
  | 'check'
  | 'chevron'
  | 'external'
  | 'clock'
  | 'globe'
  | 'record'
  | 'x'

/** Strichzüge je Icon, gezeichnet in einem 24×24-Feld. */
const PATHS: Record<IconName, string[]> = {
  play: ['M8 5.5v13l11-6.5-11-6.5z'],
  pause: ['M9.5 5.5v13', 'M14.5 5.5v13'],
  stop: ['M6.5 6.5h11v11h-11z'],
  switch: ['M3.5 8.5h14l-4-4', 'M20.5 15.5h-14l4 4'],
  plus: ['M12 5.5v13', 'M5.5 12h13'],
  edit: ['M4 20h4L19 9l-4-4L4 16v4z', 'M14.5 5.5l4 4'],
  trash: ['M4.5 7h15', 'M9.5 7V4h5v3', 'M6.5 7l1 13h9l1-13', 'M10.5 11v5', 'M13.5 11v5'],
  download: ['M12 4v11', 'M8 11l4 4 4-4', 'M5 19.5h14'],
  camera: ['M4 8.5h3.5L9 6.5h6l1.5 2H20v10H4z', 'M12 16.5a3 3 0 100-6 3 3 0 000 6z'],
  sliders: ['M4 7.5h9', 'M17 7.5h3', 'M4 16.5h3', 'M11 16.5h9', 'M15 5.5v4', 'M9 14.5v4'],
  chart: ['M4 4.5v15h16', 'M8 16v-4', 'M12.5 16V8', 'M17 16v-6'],
  folder: ['M4 6.5h5l2 2h9v10H4z'],
  keyboard: ['M3 7.5h18v9H3z', 'M7 11h.01', 'M11 11h.01', 'M15 11h.01', 'M8 14h8'],
  wifi: ['M5 11a10 10 0 0114 0', 'M8 14a6 6 0 018 0', 'M12 17.5h.01'],
  hand: [
    'M9 11.5V6a1.5 1.5 0 013 0v5.5',
    'M12 11.5V5a1.5 1.5 0 013 0v6.5',
    'M15 11.5V7.5a1.5 1.5 0 013 0V14a6 6 0 01-6 6h-1a6 6 0 01-6-6v-3a1.5 1.5 0 013 0v2',
  ],
  check: ['M5 12.5l4.5 4.5L19 7.5'],
  chevron: ['M9.5 6l6 6-6 6'],
  external: ['M14 4.5h5.5V10', 'M19.5 4.5L12 12', 'M18 14v5.5h-14V5.5H10'],
  globe: [
    'M12 20.5a8.5 8.5 0 100-17 8.5 8.5 0 000 17z',
    'M3.5 12h17',
    'M12 3.5c2.2 2.3 3.4 5.3 3.4 8.5S14.2 18.2 12 20.5C9.8 18.2 8.6 15.2 8.6 12S9.8 5.8 12 3.5z',
  ],
  clock: ['M12 20.5a8.5 8.5 0 100-17 8.5 8.5 0 000 17z', 'M12 8v4.5l3 1.8'],
  record: ['M12 19a7 7 0 100-14 7 7 0 000 14z'],
  x: ['M6.5 6.5l11 11', 'M17.5 6.5l-11 11'],
}

const props = withDefaults(
  defineProps<{
    name: IconName
    /** Kantenlänge in Pixeln. */
    size?: number
    /** Fläche füllen statt nur zeichnen (z. B. beim Aufnahmepunkt). */
    filled?: boolean
  }>(),
  { size: 16, filled: false },
)

const paths = computed(() => PATHS[props.name] ?? [])
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    :fill="filled ? 'currentColor' : 'none'"
    :stroke="filled ? 'none' : 'currentColor'"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <path v-for="(path, index) in paths" :key="index" :d="path" />
  </svg>
</template>

<style scoped>
.icon {
  flex: none;
  vertical-align: -0.18em;
}
</style>
