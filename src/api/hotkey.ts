import { locale, t } from '../i18n.ts'

/**
 * Tastenkombinationen: Aufnehmen, Anzeigen, Vorschläge.
 *
 * Bewusst ohne Vue- und Tauri-Bezug, damit die Umwandlung
 * Tastendruck -> Tauri-Schreibweise testbar bleibt.
 */

export const DEFAULT_HOTKEY = 'CommandOrControl+Alt+Space'

/**
 * Ausweichkombinationen für den häufigen Fall, dass die Voreinstellung schon
 * belegt ist. Auf macOS ist ⌘⌥Leertaste die Finder-Suche - das System fängt den
 * Tastendruck ab, bevor eine Anwendung ihn sieht.
 */
export const HOTKEY_PRESETS = [
  'CommandOrControl+Alt+G',
  'CommandOrControl+Shift+Space',
  'F13',
] as const

const MODIFIER_CODES = [
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'MetaLeft',
  'MetaRight',
  'ShiftLeft',
  'ShiftRight',
  'CapsLock',
]

/** Nur die Felder, die zum Aufnehmen gebraucht werden - so bleibt es testbar. */
export interface KeyStroke {
  code: string
  metaKey: boolean
  ctrlKey: boolean
  altKey: boolean
  shiftKey: boolean
}

const isFunctionKey = (code: string) => /^F([1-9]|1\d|20)$/.test(code)

/**
 * Übersetzt einen Tastendruck in die Schreibweise, die Tauri erwartet
 * (z. B. `CommandOrControl+Alt+G`). Gibt `null` zurück, wenn die Eingabe noch
 * keine brauchbare Kombination ist.
 */
export function shortcutFromEvent(stroke: KeyStroke): string | null {
  if (MODIFIER_CODES.includes(stroke.code)) return null

  const parts: string[] = []
  if (stroke.metaKey || stroke.ctrlKey) parts.push('CommandOrControl')
  if (stroke.altKey) parts.push('Alt')
  if (stroke.shiftKey) parts.push('Shift')

  const key = stroke.code.startsWith('Key')
    ? stroke.code.slice(3)
    : stroke.code.startsWith('Digit')
      ? stroke.code.slice(5)
      : stroke.code

  if (!key) return null
  // Funktionstasten funktionieren allein, alles andere braucht eine Zusatztaste -
  // sonst würde ein einzelnes "G" den Hotkey belegen.
  if (!parts.length && !isFunctionKey(key)) return null

  return [...parts, key].join('+')
}

/**
 * Tastenbeschriftungen. Auf macOS sind die Zeichen sprachunabhängig, sonst
 * hängen sie an der Sprache - „Strg" heißt englisch „Ctrl".
 */
const MAC_NAMES: Record<string, string> = {
  CommandOrControl: '⌘',
  Command: '⌘',
  Control: '⌃',
  Alt: '⌥',
  Shift: '⇧',
}

const NAMES_DE: Record<string, string> = {
  CommandOrControl: 'Strg',
  Command: 'Win',
  Control: 'Strg',
  Alt: 'Alt',
  Shift: 'Umschalt',
}

const NAMES_EN: Record<string, string> = {
  CommandOrControl: 'Ctrl',
  Command: 'Win',
  Control: 'Ctrl',
  Alt: 'Alt',
  Shift: 'Shift',
}

export function isMacPlatform(): boolean {
  return typeof navigator !== 'undefined' && /mac/i.test(navigator.platform)
}

/** Macht aus `CommandOrControl+Alt+Space` eine lesbare Tastenfolge. */
export function formatHotkey(shortcut: string | undefined | null, mac = isMacPlatform()): string {
  if (!shortcut) return '…'
  const modifiers = mac ? MAC_NAMES : locale.value === 'en' ? NAMES_EN : NAMES_DE
  const space = t('key.space')

  return shortcut
    .split('+')
    .map((part) => (part === 'Space' ? space : (modifiers[part] ?? part)))
    .join(mac ? ' ' : ' + ')
}
