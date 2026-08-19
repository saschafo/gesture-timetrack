/**
 * Sinnbilder und Reihenfolge der Gesten - an einer Stelle, damit Statuskarte
 * und Trainingsbereich dieselben verwenden.
 *
 * Emoji-Handzeichen statt gezeichneter Symbole: Sie zeigen die Handhaltung
 * deutlicher, als es ein Strichsymbol könnte.
 */

import type { Gesture } from './gesture-classifier.ts'

export const GESTURE_SYMBOLS: Record<Gesture, string> = {
  open_hand: '🖐️',
  fist: '✊',
  thumb_up: '👍',
  one_finger: '☝️',
  two_fingers: '✌️',
}

export const GESTURE_ORDER: Gesture[] = [
  'open_hand',
  'fist',
  'thumb_up',
  'one_finger',
  'two_fingers',
]

/** Ein Eintrag der Übersicht: welche Geste, welche Aktion sie auslöst. */
export interface LegendEntry {
  gesture: Gesture
  /** Kennung des Aktionstexts im Wörterbuch. */
  action: string
}

/**
 * Gruppen für die Übersicht: Gegensätze zusammen, damit sich das Vokabular auf
 * einen Blick erfassen lässt.
 *
 * „Weiter" hat keine eigene Geste - die offene Hand setzt eine Pause fort.
 * Sie steht deshalb zweimal in der Liste: Der Nutzer soll sehen, wie er
 * fortsetzt, ohne dafür eine sechste Geste lernen zu müssen.
 */
export const LEGEND_GROUPS: LegendEntry[][] = [
  [
    { gesture: 'open_hand', action: 'gestureAction.open_hand' },
    { gesture: 'fist', action: 'gestureAction.fist' },
  ],
  [
    { gesture: 'thumb_up', action: 'gestureAction.thumb_up' },
    { gesture: 'open_hand', action: 'gestureAction.resume' },
  ],
  [
    { gesture: 'one_finger', action: 'gestureAction.one_finger' },
    { gesture: 'two_fingers', action: 'gestureAction.two_fingers' },
  ],
]
