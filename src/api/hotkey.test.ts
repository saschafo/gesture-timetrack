import assert from 'node:assert/strict'
import test from 'node:test'

import { formatHotkey, shortcutFromEvent, type KeyStroke } from './hotkey.ts'

const stroke = (overrides: Partial<KeyStroke> & { code: string }): KeyStroke => ({
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...overrides,
})

test('setzt Tastendrücke in die Tauri-Schreibweise um', () => {
  assert.equal(
    shortcutFromEvent(stroke({ code: 'Space', metaKey: true, altKey: true })),
    'CommandOrControl+Alt+Space',
  )
  assert.equal(
    shortcutFromEvent(stroke({ code: 'KeyG', ctrlKey: true, altKey: true })),
    'CommandOrControl+Alt+G',
  )
  assert.equal(
    shortcutFromEvent(stroke({ code: 'Digit7', metaKey: true, shiftKey: true })),
    'CommandOrControl+Shift+7',
  )
})

test('Funktionstasten gelten ohne Zusatztaste, Buchstaben nicht', () => {
  assert.equal(shortcutFromEvent(stroke({ code: 'F13' })), 'F13')
  assert.equal(shortcutFromEvent(stroke({ code: 'F20' })), 'F20')
  // Ein einzelnes G würde die Tastatur unbenutzbar machen.
  assert.equal(shortcutFromEvent(stroke({ code: 'KeyG' })), null)
  assert.equal(shortcutFromEvent(stroke({ code: 'Space' })), null)
})

test('reine Zusatztasten lösen keine Aufnahme aus', () => {
  for (const code of ['MetaLeft', 'AltRight', 'ShiftLeft', 'ControlLeft', 'CapsLock']) {
    assert.equal(shortcutFromEvent(stroke({ code, metaKey: true })), null, code)
  }
})

test('zeigt Kombinationen je Plattform lesbar an', () => {
  assert.equal(formatHotkey('CommandOrControl+Alt+Space', true), '⌘ ⌥ Leertaste')
  assert.equal(
    formatHotkey('CommandOrControl+Alt+Space', false),
    'Strg + Alt + Leertaste',
  )
  assert.equal(formatHotkey('F13', true), 'F13')
  assert.equal(formatHotkey(null, true), '…')
})
