import assert from 'node:assert/strict'
import test from 'node:test'

import { parseHex, projectButtonStyle, readableTextOn } from './colors.ts'

test('liest kurze und lange Hex-Schreibweise', () => {
  assert.deepEqual(parseHex('#4f46e5'), [79, 70, 229])
  assert.deepEqual(parseHex('#FFF'), [255, 255, 255])
  assert.deepEqual(parseHex('  #0891b2  '), [8, 145, 178])

  assert.equal(parseHex('var(--border)'), null)
  assert.equal(parseHex('rot'), null)
  assert.equal(parseHex(null), null)
})

test('wählt die lesbare Textfarbe zur Projektfarbe', () => {
  // Dunkle Projektfarben tragen weißen Text ...
  assert.equal(readableTextOn('#4f46e5'), '#ffffff')
  assert.equal(readableTextOn('#0891b2'), '#ffffff')
  // ... helle dagegen dunklen.
  assert.equal(readableTextOn('#fde047'), '#14161f')
  assert.equal(readableTextOn('#ffffff'), '#14161f')
})

test('gibt ohne verwertbare Farbe keinen Stil vor', () => {
  assert.equal(projectButtonStyle('var(--border)'), null)
  assert.equal(projectButtonStyle(undefined), null)
  assert.deepEqual(projectButtonStyle('#16a34a'), {
    background: '#16a34a',
    borderColor: '#16a34a',
    color: '#ffffff',
  })
})
