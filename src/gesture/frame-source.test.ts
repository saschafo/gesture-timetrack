import assert from 'node:assert/strict'
import test from 'node:test'

import { cameraSourceOf, coverBox, detectionTimeoutMs } from './frame-source.ts'
import type { AppSettings } from '../api/backend.ts'

const settings = (overrides: Partial<AppSettings> = {}): AppSettings => ({
  hotkey: 'CommandOrControl+Alt+Space',
  hotkey_active: true,
  hotkey_error: null,
  confidence_threshold: 0.85,
  overlay_timeout_ms: 3000,
  overlay_timeout_network_ms: 4000,
  camera_source: 'builtin',
  camera_url: '',
  sound_cue: true,
  slot_1_project_id: null,
  slot_2_project_id: null,
  active_slot: 1,
  use_training: false,
  language: 'de',
  ...overrides,
})

test('Standard ist die eingebaute Webcam', () => {
  assert.equal(cameraSourceOf(settings()), 'builtin')
  assert.equal(detectionTimeoutMs(settings()), 3000)
})

test('Netzwerk-Kamera nur mit hinterlegter Adresse', () => {
  // Umgestellt, aber keine Adresse: der Standardweg darf nicht kaputtgehen.
  assert.equal(cameraSourceOf(settings({ camera_source: 'network' })), 'builtin')
  assert.equal(
    cameraSourceOf(settings({ camera_source: 'network', camera_url: '   ' })),
    'builtin',
  )
  assert.equal(
    cameraSourceOf(
      settings({ camera_source: 'network', camera_url: 'http://192.168.1.20:4747/video' }),
    ),
    'network',
  )
})

test('Netzwerk-Kamera bekommt das längere Zeitfenster', () => {
  const network = settings({
    camera_source: 'network',
    camera_url: 'http://192.168.1.20:4747/video',
  })
  assert.equal(detectionTimeoutMs(network), 4000)

  // Der Wert kommt aus den Einstellungen, ist also änderbar.
  assert.equal(detectionTimeoutMs({ ...network, overlay_timeout_network_ms: 6000 }), 6000)
})

test('coverBox schneidet wie object-fit: cover zu', () => {
  // Breites Bild in quadratischer Vorschau: links und rechts wird beschnitten.
  const wide = coverBox(640, 480, 200, 200)
  assert.equal(wide.height, 200)
  assert.ok(wide.width > 200, `Breite ${wide.width} muss überstehen`)
  assert.equal(wide.y, 0)
  assert.ok(wide.x < 0, 'horizontal zentriert beschnitten')

  // Hochformat (Handy): oben und unten wird beschnitten.
  const tall = coverBox(480, 640, 200, 200)
  assert.equal(tall.width, 200)
  assert.ok(tall.height > 200)
  assert.equal(tall.x, 0)

  // Unbekannte Maße dürfen die Anzeige nicht sprengen.
  assert.deepEqual(coverBox(0, 0, 200, 200), { x: 0, y: 0, width: 200, height: 200 })
})
