import assert from 'node:assert/strict'
import test from 'node:test'

import { featureVector, FEATURE_VERSION } from './features.ts'
import { POSES, hand, type HandPose } from './fixtures.ts'
import {
  buildModel,
  builtinsTrained,
  classifyTrained,
  MIN_SAMPLES,
  type TrainingSample,
} from './trained-classifier.ts'
import { createRecognizer } from './recognizer.ts'
import type { Point } from './gesture-classifier.ts'

/** Reproduzierbarer Zufall - Tests dürfen nicht mal so, mal so ausgehen. */
function seeded(seed: number) {
  let state = seed
  return () => {
    state = (state * 1103515245 + 12345) % 2147483648
    return state / 2147483648
  }
}

/** Rauschen auf die Landmarks, wie es eine echte Aufnahme mitbringt. */
function jitter(points: Point[], random: () => number, amount = 0.006): Point[] {
  return points.map((point) => ({
    x: point.x + (random() - 0.5) * amount,
    y: point.y + (random() - 0.5) * amount,
    z: point.z,
  }))
}

/** Nimmt jede Geste mehrfach auf - wie der Nutzer beim Einlernen. */
function record(samplesPerGesture = MIN_SAMPLES): TrainingSample[] {
  const random = seeded(42)
  const samples: TrainingSample[] = []
  for (const [gesture, pose] of Object.entries(POSES)) {
    for (let take = 0; take < samplesPerGesture; take++) {
      samples.push({ gesture, features: featureVector(jitter(hand(pose), random)) })
    }
  }
  return samples
}

test('Modell entsteht erst mit genügend Aufnahmen je Geste', () => {
  assert.equal(buildModel([]), null)
  assert.equal(buildModel(record(MIN_SAMPLES - 1)), null, 'zu wenige Aufnahmen')
  assert.ok(buildModel(record()), 'vollständiges Training ergibt ein Modell')
})

test('erkennt eingelernte Gesten wieder', () => {
  const model = buildModel(record())!
  const random = seeded(7)

  for (const [name, pose] of Object.entries(POSES)) {
    const result = classifyTrained(model, jitter(hand(pose), random))
    assert.equal(result.gesture, name, `${name} wurde als ${result.gesture} gelesen`)
    assert.ok(
      result.confidence >= 0.85,
      `${name} nur mit ${result.confidence.toFixed(2)} erkannt`,
    )
  }
})

test('unbekannte Haltung bleibt unter der Schwelle', () => {
  const model = buildModel(record())!
  // Halb geöffnete Hand: in keiner Aufnahme enthalten.
  const unknown: HandPose = {
    thumb: 'tucked',
    index: true,
    middle: 'relaxed',
    ring: false,
    pinky: 'relaxed',
  }
  const result = classifyTrained(model, hand(unknown))
  assert.ok(
    result.confidence < 0.85,
    `fremde Haltung wurde mit ${result.confidence.toFixed(2)} als ${result.gesture} gewertet`,
  )
})

test('liefert weiter die Fingerwerte für die Anzeige', () => {
  const model = buildModel(record())!
  const result = classifyTrained(model, hand(POSES.open_hand))
  assert.ok(result.fingers.index > 0.9)
  assert.ok(result.fingers.pinky > 0.9)
})

test('Erkenner fällt ohne brauchbares Training auf die Regeln zurück', () => {
  const samples = record()

  assert.equal(
    createRecognizer({ use_training: false }, { version: FEATURE_VERSION, samples }).mode,
    'geometric',
    'ohne eigene Gesten bleibt es bei den Regeln',
  )
  assert.equal(createRecognizer({ use_training: true }, null).mode, 'geometric')
  // Aufnahmen eines älteren Merkmalssatzes werden nicht verwendet.
  assert.equal(
    createRecognizer({ use_training: true }, { version: FEATURE_VERSION - 1, samples }).mode,
    'geometric',
  )
  assert.equal(
    createRecognizer({ use_training: true }, { version: FEATURE_VERSION, samples: [] }).mode,
    'geometric',
  )
  assert.equal(
    createRecognizer({ use_training: true }, { version: FEATURE_VERSION, samples }).mode,
    'trained',
  )
})

/** Aufnahmen einer eigenen Geste unter der Kennung custom:<id>. */
function recordCustom(id: number, pose: HandPose, takes = MIN_SAMPLES): TrainingSample[] {
  const random = seeded(id * 977)
  const samples: TrainingSample[] = []
  for (let take = 0; take < takes; take++) {
    samples.push({ gesture: `custom:${id}`, features: featureVector(jitter(hand(pose), random)) })
  }
  return samples
}

/** Eine Haltung, die keiner Grundgeste entspricht: nur der kleine Finger. */
const PINKY_ONLY: HandPose = {
  thumb: 'tucked',
  index: false,
  middle: false,
  ring: false,
  pinky: true,
}

test('eigene Gesten wirken auch ohne eingelernte Grundgesten', () => {
  const samples = recordCustom(3, PINKY_ONLY)
  assert.equal(builtinsTrained(samples), false)

  const recognizer = createRecognizer(
    { use_training: false, confidence_threshold: 0.85 },
    { version: FEATURE_VERSION, samples },
  )
  assert.equal(recognizer.mode, 'hybrid')

  const result = recognizer.classify(hand(PINKY_ONLY))
  assert.equal(result.label, 'custom:3')
  assert.equal(result.gesture, null, 'keine Grundgeste')
  assert.ok(result.confidence >= 0.85, `nur ${result.confidence.toFixed(2)}`)
})

test('eine eigene Geste verdeckt keine Grundgeste', () => {
  // Eigene Geste absichtlich auf die Faust eingelernt - der Regelweg muss
  // trotzdem gewinnen, sonst wäre „Stopp" nicht mehr erreichbar.
  const recognizer = createRecognizer(
    { use_training: false, confidence_threshold: 0.85 },
    { version: FEATURE_VERSION, samples: recordCustom(4, POSES.fist) },
  )
  const result = recognizer.classify(hand(POSES.fist))
  assert.equal(result.gesture, 'fist')
  assert.equal(result.label, 'fist')
})

test('vollständiges Training kennt Grundgesten und eigene Gesten zugleich', () => {
  const samples = [...record(), ...recordCustom(5, PINKY_ONLY)]
  const recognizer = createRecognizer(
    { use_training: true, confidence_threshold: 0.85 },
    { version: FEATURE_VERSION, samples },
  )
  assert.equal(recognizer.mode, 'trained')
  assert.equal(recognizer.classify(hand(PINKY_ONLY)).label, 'custom:5')
  assert.equal(recognizer.classify(hand(POSES.open_hand)).label, 'open_hand')
  assert.equal(recognizer.classify(hand(POSES.fist)).label, 'fist')
})

test('eingelernte Erkennung liefert dieselben Gesten wie die Regeln', () => {
  const recognizer = createRecognizer(
    { use_training: true },
    { version: FEATURE_VERSION, samples: record() },
  )
  for (const [name, pose] of Object.entries(POSES)) {
    assert.equal(recognizer.classify(hand(pose)).gesture, name)
  }
})
