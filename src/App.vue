<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { openUrl } from '@tauri-apps/plugin-opener'

import {
  COPYRIGHT_HOLDER,
  PRODUCT_NAME,
  WEBSITE_LABEL,
  WEBSITE_URL,
  copyrightYear,
} from './branding'
import { LOCALES, locale, t } from './i18n'
import CameraPreview from './components/CameraPreview.vue'
import CollapsibleCard from './components/CollapsibleCard.vue'
import Icon from './components/Icon.vue'
import ProjectSelector from './components/ProjectSelector.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import StatusPanel from './components/StatusPanel.vue'
import TodayOverview from './components/TodayOverview.vue'
import { useTimetrackStore } from './stores/timetrack'

const store = useTimetrackStore()
const year = copyrightYear()

/** Sprache umschalten - ohne Umweg über die Einstellungen. */
function setLanguage(code: string) {
  if (code === store.settings?.language) return
  void store.changeSetting('language', code)
}
const linkError = ref(false)
/** Version der Anwendung selbst - nicht aus einer zweiten Quelle abgeschrieben. */
const version = ref('')

/**
 * Links gehen in den Standardbrowser: Das Anwendungsfenster soll nie zu einem
 * Browser werden, und die Anwendung selbst spricht mit keinem Server.
 */
async function openWebsite() {
  try {
    await openUrl(WEBSITE_URL)
  } catch {
    linkError.value = true
  }
}

onMounted(async () => {
  void store.init()
  version.value = await getVersion().catch(() => '')
})
onBeforeUnmount(() => store.dispose())
</script>

<template>
  <div class="app">
    <header>
      <div>
        <h1>{{ PRODUCT_NAME }}</h1>
        <p class="muted">{{ t('app.claim') }}</p>
      </div>

      <div class="brand muted">
        <!-- Sprachschalter im Kopf: häufiger gebraucht, als es ein Eintrag in
             den Einstellungen vermuten lässt. -->
        <div class="lang" :title="t('settings.languageNote')">
          <Icon name="globe" :size="13" />
          <button
            v-for="option in LOCALES"
            :key="option.code"
            class="code"
            :class="{ active: locale === option.code }"
            :aria-pressed="locale === option.code"
            @click="setLanguage(option.code)"
          >
            {{ option.code.toUpperCase() }}
          </button>
        </div>

        <span>© {{ year }} {{ COPYRIGHT_HOLDER }}</span>
        <button class="link" :title="WEBSITE_URL" @click="openWebsite">
          <Icon name="external" :size="13" />
          {{ WEBSITE_LABEL }}
        </button>
        <span class="license">
          <span v-if="version" class="mono">v{{ version }}</span>
          <span v-if="version"> · </span>{{ t('common.license') }}
        </span>
        <span v-if="linkError" class="license">
          {{ t('app.linkFailed', { url: WEBSITE_URL }) }}
        </span>
      </div>
    </header>

    <p v-if="store.error" class="error" @click="store.error = null">{{ store.error }}</p>

    <StatusPanel />

    <!-- Raster über zwölf Spalten: die drei Karten oben teilen sich eine Reihe,
         Vorschau und Auswertung brauchen die ganze Breite. So bleibt links kein
         Leerraum stehen, wenn rechts eine Karte ausgeklappt ist. -->
    <div class="grid">
      <!-- Arbeitsbereich: links die Projekte, rechts die Auswertung. Deren
           Tabelle hat acht Spalten und braucht deshalb zwei Drittel. -->
      <div class="col-projects"><ProjectSelector /></div>
      <div class="col-report"><TodayOverview /></div>

      <!-- Werkzeuge unten: beides eingeklappt, beides selten gebraucht. -->
      <div class="col-preview">
        <CollapsibleCard
          :title="t('app.previewTitle')"
          :hint="t('app.previewHint')"
          storage-key="preview"
          icon="camera"
          :summary="t('app.previewSummary')"
        >
          <CameraPreview />
        </CollapsibleCard>
      </div>
      <div class="col-settings"><SettingsPanel /></div>
    </div>

    <footer class="muted">{{ t('app.privacy') }}</footer>
  </div>
</template>

<style scoped>
.app {
  /* Breit genug für drei Karten nebeneinander auf HD-Bildschirmen, aber
     begrenzt: über etwa 1500 px werden Textzeilen unangenehm lang. */
  max-width: 1500px;
  margin: 0 auto;
  padding: 22px 24px 36px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

header h1 {
  margin: 0;
  font-size: 20px;
  letter-spacing: -0.3px;
}

header p {
  margin: 2px 0 0;
  font-size: 13px;
}

/* Rechts außen: Rechteinhaber, Website, Lizenz - untereinander und rechts
   ausgerichtet, damit der Kopf ruhig bleibt. */
.brand {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  font-size: 11.5px;
  text-align: right;
  line-height: 1.45;
}

.lang {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 4px;
  padding: 2px 6px 2px 7px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--surface);
}

.lang .code {
  padding: 1px 5px;
  border: none;
  border-radius: 999px;
  background: none;
  color: var(--muted);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.3px;
}

.lang .code:hover {
  background: var(--surface-2);
}

.lang .code.active {
  background: var(--accent);
  color: #fff;
}

.brand .link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0;
  border: none;
  background: none;
  color: var(--accent);
  font-size: 11.5px;
}

.brand .link:hover {
  background: none;
  text-decoration: underline;
}

.brand .license {
  opacity: 0.75;
}

.grid {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  gap: 18px;
  align-items: start;
}

.col-projects {
  grid-column: span 4;
}

.col-report {
  grid-column: span 8;
}

.col-preview {
  grid-column: span 7;
}

.col-settings {
  grid-column: span 5;
}

/* Untereinander, sobald zwei Karten nebeneinander zu eng würden. */
@media (max-width: 1100px) {
  .col-projects,
  .col-report,
  .col-preview,
  .col-settings {
    grid-column: 1 / -1;
  }
}

.error {
  margin: 0;
  padding: 10px 14px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  color: var(--danger);
  font-size: 13px;
  cursor: pointer;
}

footer {
  font-size: 12px;
  text-align: center;
  padding-top: 4px;
}
</style>
