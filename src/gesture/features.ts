/**
 * Merkmalsvektor einer Handhaltung - die Grundlage fürs Einlernen eigener
 * Gesten.
 *
 * Anforderungen an die Merkmale: unabhängig von Handgröße, Kameraabstand und
 * Bildausschnitt, damit eine eingelernte Geste auch dann noch passt, wenn die
 * Hand nächstes Mal etwas anders im Bild steht. Deshalb ausschließlich
 * Verhältnisse, keine absoluten Koordinaten.
 *
 * Zwei Merkmale sind bewusst **nicht** drehinvariant (Daumenhöhe): „Daumen
 * hoch" und „Daumen runter" unterscheiden sich nur durch die Richtung im Bild.
 */

import type { Point } from './gesture-classifier.ts'

/**
 * Version des Merkmalssatzes. Wird die Liste unten geändert, muss diese Zahl
 * hoch - und ebenso `FEATURE_VERSION` in src-tauri/src/state.rs. Alte
 * Aufnahmen gelten dann als veraltet, statt falsch gedeutet zu werden.
 *
 * Fassung 2: Formmerkmale kommen aus MediaPipes Weltkoordinaten statt aus
 * normierten Bildkoordinaten - Letztere sind bei 16:9 in x und y
 * unterschiedlich gestreckt und perspektivisch verzerrt.
 */
export const FEATURE_VERSION = 2

/** Klartextnamen in derselben Reihenfolge wie der Vektor - für Diagnose. */
export const FEATURE_LABELS = [
  'Zeigefinger gestreckt',
  'Mittelfinger gestreckt',
  'Ringfinger gestreckt',
  'kleiner Finger gestreckt',
  'Daumen gestreckt',
  'Daumen abgespreizt',
  'Daumenhöhe',
  'Abstand Zeige–Mittel',
  'Abstand Mittel–Ring',
  'Abstand Ring–klein',
] as const

export const FEATURE_LEN = FEATURE_LABELS.length

const WRIST = 0
const THUMB_CMC = 1
const THUMB_MCP = 2
const THUMB_IP = 3
const THUMB_TIP = 4
const INDEX_MCP = 5
const INDEX_TIP = 8
const MIDDLE_MCP = 9
const MIDDLE_TIP = 12
const RING_TIP = 16
const PINKY_TIP = 20

/** Abstand im Raum - z gehört dazu, sonst täuschen verdeckte Finger. */
const distance = (a: Point, b: Point) =>
  Math.hypot(a.x - b.x, a.y - b.y, (a.z ?? 0) - (b.z ?? 0))

/** Abstand in der Bildebene - für Richtungsfragen. */
const distance2d = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y)

/** Referenzlänge der Hand: Handgelenk bis Mittelfinger-Grundgelenk. */
const handScale = (points: Point[]) =>
  Math.max(distance(points[WRIST], points[MIDDLE_MCP]), 1e-6)

/**
 * Streckung eines Fingers: Luftlinie zur Spitze gegen die Länge entlang der
 * Glieder. 1 = gerade, deutlich kleiner = eingeklappt.
 */
function straightness(points: Point[], joints: number[]): number {
  let chain = 0
  for (let index = 1; index < joints.length; index++) {
    chain += distance(points[joints[index - 1]], points[joints[index]])
  }
  const span = distance(points[joints[0]], points[joints[joints.length - 1]])
  return span / Math.max(chain, 1e-6)
}

/**
 * Baut den Merkmalsvektor. Reihenfolge muss zu FEATURE_LABELS passen.
 *
 * `world` sind die metrischen Weltkoordinaten; fehlen sie, wird auf die
 * Bildkoordinaten zurückgefallen. Die Daumenhöhe bleibt immer bildbezogen -
 * „oben" ist eine Aussage über die Ansicht.
 */
export function featureVector(points: Point[], world?: Point[] | null): number[] {
  const shape = world && world.length >= 21 ? world : points
  const scale = handScale(shape)
  const imageScale = Math.max(distance2d(points[WRIST], points[MIDDLE_MCP]), 1e-6)

  return [
    straightness(shape, [5, 6, 7, 8]),
    straightness(shape, [9, 10, 11, 12]),
    straightness(shape, [13, 14, 15, 16]),
    straightness(shape, [17, 18, 19, 20]),
    straightness(shape, [THUMB_CMC, THUMB_MCP, THUMB_IP, THUMB_TIP]),
    distance(shape[THUMB_TIP], shape[INDEX_MCP]) / scale,
    (points[INDEX_MCP].y - points[THUMB_TIP].y) / imageScale,
    distance(shape[INDEX_TIP], shape[MIDDLE_TIP]) / scale,
    distance(shape[MIDDLE_TIP], shape[RING_TIP]) / scale,
    distance(shape[RING_TIP], shape[PINKY_TIP]) / scale,
  ]
}
