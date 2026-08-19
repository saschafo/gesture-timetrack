import assert from 'node:assert/strict'
import test from 'node:test'

import {
  GestureStabilizer,
  classify,
  fingerScores,
  type Classification,
  type Gesture,
} from './gesture-classifier.ts'
import { POSES, foldInDepth, hand } from './fixtures.ts'

const THRESHOLD = 0.85

test('erkennt jede Geste des Vokabulars über der Konfidenz-Schwelle', () => {
  for (const [name, pose] of Object.entries(POSES)) {
    const result = classify(hand(pose))
    assert.equal(result.gesture, name, `${name} wurde als ${result.gesture} gelesen`)
    assert.ok(
      result.confidence >= THRESHOLD,
      `${name} nur mit ${result.confidence.toFixed(2)} erkannt`,
    )
  }
})

test('bewertet kurze Finger wie lange', () => {
  // Der Grund für die Umstellung des Maßes: Der kleine Finger ist deutlich
  // kürzer als der Mittelfinger. Wird auf die Handgröße statt auf die
  // Fingerlänge normiert, gilt er nie als gestreckt - und die offene Hand
  // scheitert an ihrem schwächsten Glied.
  const scores = fingerScores(hand(POSES.open_hand))
  for (const [finger, value] of Object.entries(scores)) {
    assert.ok(value > 0.9, `${finger} nur mit ${value.toFixed(2)} gestreckt`)
  }
})

test('leicht gebeugte Finger gelten noch als gestreckt', () => {
  const relaxed = classify(
    hand({ thumb: 'spread', index: 'relaxed', middle: 'relaxed', ring: 'relaxed', pinky: 'relaxed' }),
  )
  assert.equal(relaxed.gesture, 'open_hand')
  assert.ok(
    relaxed.confidence >= THRESHOLD,
    `locker geöffnete Hand nur mit ${relaxed.confidence.toFixed(2)}`,
  )
})

test('erkennt in die Tiefe eingeklappte Finger', () => {
  // Ein Finger gezeigt, die übrigen hinter der Handfläche eingeklappt.
  const folded = foldInDepth(hand(POSES.open_hand), ['middle', 'ring', 'pinky'])
  assert.equal(classify(folded).gesture, 'one_finger')

  // Gegenprobe ohne Tiefenangabe: dieselbe Hand galt als offen - das war der
  // gemeldete Fehler, und deshalb zählt z jetzt mit.
  const flat = folded.map((point) => ({ ...point, z: 0 }))
  assert.equal(classify(flat).gesture, 'open_hand')
})

test('offene Hand zählt auch mit angelegtem Daumen', () => {
  // Der Daumen ist die unsicherste Messgröße; für die Absicht "Start" ist er
  // unerheblich.
  const tucked = classify(
    hand({ thumb: 'tucked', index: true, middle: true, ring: true, pinky: true }),
  )
  assert.equal(tucked.gesture, 'open_hand')
  assert.ok(tucked.confidence >= THRESHOLD)
})

test('Faust bleibt Faust, egal wie der Daumen liegt', () => {
  assert.equal(classify(hand(POSES.fist)).gesture, 'fist')
  assert.equal(classify(hand(POSES.thumb_up)).gesture, 'thumb_up')

  // Der Daumenabstand geht bewusst nicht mehr in die Bewertung ein: er ist zu
  // unsicher zu messen. Deshalb muss die Faust auch mit lose oder deutlich
  // abgespreiztem Daumen erkannt werden.
  for (const thumb of ['loose', 'spread'] as const) {
    const result = classify(
      hand({ thumb, index: false, middle: false, ring: false, pinky: false }),
    )
    assert.equal(result.gesture, 'fist', `Daumen ${thumb}`)
    assert.ok(result.confidence >= THRESHOLD, `Daumen ${thumb}: ${result.confidence.toFixed(2)}`)
  }
})

test('unterscheidet einen und zwei Finger', () => {
  assert.equal(classify(hand(POSES.one_finger)).gesture, 'one_finger')
  assert.equal(classify(hand(POSES.two_fingers)).gesture, 'two_fingers')

  // Drei gestreckte Finger gehören zu keiner Geste mehr - und dürfen deshalb
  // auch nicht als zwei Finger durchgehen.
  const three = classify(
    hand({ thumb: 'tucked', index: true, middle: true, ring: true, pinky: false }),
  )
  assert.ok(
    three.confidence < THRESHOLD,
    `drei Finger wurden als ${three.gesture} mit ${three.confidence.toFixed(2)} gewertet`,
  )
})

test('liefert keine Geste ohne vollständige Landmarks', () => {
  assert.equal(classify([]).gesture, null)
  assert.equal(classify(hand(POSES.fist).slice(0, 10)).gesture, null)
})

test('Zwischenhaltungen bleiben unter der Schwelle', () => {
  // Zeige- und Ringfinger gestreckt, Mittelfinger eingeklappt: passt zu keiner
  // definierten Geste.
  const ambiguous = classify(
    hand({ thumb: 'tucked', index: true, middle: false, ring: true, pinky: false }),
  )
  assert.ok(
    ambiguous.confidence < THRESHOLD,
    `Zwischenhaltung wurde mit ${ambiguous.confidence.toFixed(2)} als ${ambiguous.gesture} gewertet`,
  )
})

test('Stabilisator bestätigt erst nach mehreren gleichen Frames', () => {
  const stabilizer = new GestureStabilizer(THRESHOLD, 3)
  const frame = classify(hand(POSES.open_hand))

  assert.equal(stabilizer.push(frame), null)
  assert.equal(stabilizer.push(frame), null)
  const confirmed = stabilizer.push(frame)
  assert.equal(confirmed?.gesture, 'open_hand' satisfies Gesture)
  assert.ok((confirmed?.confidence ?? 0) >= THRESHOLD)
})

test('Stabilisator verwirft Wackler und startet neu', () => {
  const stabilizer = new GestureStabilizer(THRESHOLD, 3)
  const open = classify(hand(POSES.open_hand))
  const fist = classify(hand(POSES.fist))

  stabilizer.push(open)
  stabilizer.push(open)
  // Andere Geste dazwischen -> Zähler beginnt von vorn.
  assert.equal(stabilizer.push(fist), null)
  assert.equal(stabilizer.push(fist), null)
  assert.equal(stabilizer.push(fist)?.gesture, 'fist')
})

test('Konfidenz unter der Schwelle löst nichts aus', () => {
  const stabilizer = new GestureStabilizer(0.85, 2)
  const weak: Classification = {
    gesture: 'open_hand',
    label: 'open_hand',
    confidence: 0.7,
    fingers: { index: 0.7, middle: 0.7, ring: 0.7, pinky: 0.7 },
    thumb: { up: 0 },
  }

  // Auch beliebig oft wiederholt: unter der Schwelle wird nichts gebucht.
  for (let attempt = 0; attempt < 5; attempt++) {
    assert.equal(stabilizer.push(weak), null)
  }
})
