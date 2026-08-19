/**
 * Synthetische Handlandmarks für die Tests des Klassifikators.
 *
 * Die Werte bilden eine Hand in Normkoordinaten nach (0..1, y wächst nach
 * unten), wie MediaPipe sie liefert: Handgelenk unten, Knöchel in der Mitte,
 * Finger nach oben. Damit lässt sich das Gestenvokabular prüfen, ohne eine
 * echte Kamera - die Geometrie ist bewusst grob, damit die Tests nicht an
 * Nachkommastellen hängen.
 */

import type { Point } from './gesture-classifier.ts'

const point = (x: number, y: number): Point => ({ x, y, z: 0 })

const WRIST = point(0.5, 0.95)

/** Grundgelenke von Zeige- bis kleinem Finger. */
const MCP: Record<string, Point> = {
  index: point(0.42, 0.65),
  middle: point(0.5, 0.63),
  ring: point(0.58, 0.65),
  pinky: point(0.65, 0.68),
}

/**
 * Gestreckter Finger: Glieder liegen auf einer Linie, die Luftlinie zur Spitze
 * entspricht damit der Fingerlänge.
 *
 * `length` erlaubt kurze und lange Finger - genau daran ist die frühere
 * Messung gescheitert, deshalb prüfen die Tests beides.
 */
function extended(mcp: Point, length = 0.27): Point[] {
  return [
    mcp,
    point(mcp.x, mcp.y - length * 0.41),
    point(mcp.x, mcp.y - length * 0.7),
    point(mcp.x, mcp.y - length),
  ]
}

/** Eingeklappter Finger: Spitze zurück Richtung Handfläche. */
function curled(mcp: Point, length = 0.27): Point[] {
  return [
    mcp,
    point(mcp.x, mcp.y - length * 0.33),
    point(mcp.x, mcp.y - length * 0.15),
    point(mcp.x, mcp.y + length * 0.07),
  ]
}

/** Leicht gebeugter Finger - der Alltagsfall bei der offenen Hand. */
function relaxed(mcp: Point, length = 0.27): Point[] {
  return [
    mcp,
    point(mcp.x, mcp.y - length * 0.4),
    point(mcp.x + length * 0.05, mcp.y - length * 0.66),
    point(mcp.x + length * 0.13, mcp.y - length * 0.86),
  ]
}

export type ThumbPose = 'spread' | 'tucked' | 'loose' | 'up'

function thumb(pose: ThumbPose): Point[] {
  const cmc = point(0.42, 0.86)
  switch (pose) {
    // Abgespreizt neben der offenen Hand.
    case 'spread':
      return [cmc, point(0.34, 0.79), point(0.28, 0.71), point(0.22, 0.62)]
    // An die Faust angelegt, auf Höhe der Knöchel.
    case 'tucked':
      return [cmc, point(0.42, 0.78), point(0.44, 0.70), point(0.47, 0.64)]
    // Faust mit lose abgespreiztem Daumen - der Alltagsfall.
    case 'loose':
      return [cmc, point(0.38, 0.80), point(0.33, 0.73), point(0.28, 0.66)]
    case 'up':
      return [cmc, point(0.40, 0.74), point(0.39, 0.60), point(0.38, 0.44)]
  }
}

/** Zustand eines Fingers im Testmodell. */
export type FingerPose = boolean | 'relaxed'

export interface HandPose {
  thumb: ThumbPose
  index: FingerPose
  middle: FingerPose
  ring: FingerPose
  pinky: FingerPose
}

/**
 * Natürliche Fingerlängen, relativ zueinander. Der kleine Finger ist deutlich
 * kürzer als der Mittelfinger - das muss das Testmodell abbilden.
 */
const LENGTHS: Record<string, number> = {
  index: 0.26,
  middle: 0.29,
  ring: 0.26,
  pinky: 0.2,
}

/** Baut die 21 Landmarks in MediaPipe-Reihenfolge. */
export function hand(pose: HandPose): Point[] {
  const finger = (name: keyof typeof MCP, state: FingerPose) => {
    const length = LENGTHS[name]
    if (state === 'relaxed') return relaxed(MCP[name], length)
    return state ? extended(MCP[name], length) : curled(MCP[name], length)
  }

  return [
    WRIST,
    ...thumb(pose.thumb),
    ...finger('index', pose.index),
    ...finger('middle', pose.middle),
    ...finger('ring', pose.ring),
    ...finger('pinky', pose.pinky),
  ]
}

/** Fingerglieder je Finger: [MCP, PIP, DIP, TIP]. */
const JOINTS: Record<keyof typeof MCP, [number, number, number, number]> = {
  index: [5, 6, 7, 8],
  middle: [9, 10, 11, 12],
  ring: [13, 14, 15, 16],
  pinky: [17, 18, 19, 20],
}

/**
 * Klappt Finger in die **Tiefe** ein: im Bild bleiben sie gerade, im Raum
 * krümmen sie sich zur Handfläche.
 *
 * Genau diese Haltung entsteht, wenn man einen Finger zeigt und die übrigen
 * hinter der Handfläche einklappt - und genau sie wurde mit einer rein
 * zweidimensionalen Messung für eine offene Hand gehalten.
 */
export function foldInDepth(
  points: Point[],
  fingers: Array<keyof typeof MCP>,
  depth = 0.24,
): Point[] {
  const copy = points.map((point) => ({ ...point }))
  for (const finger of fingers) {
    const [, pip, dip, tip] = JOINTS[finger]
    copy[pip].z = -depth * 0.5
    copy[dip].z = -depth
    copy[tip].z = -depth * 0.5
  }
  return copy
}

export const POSES: Record<string, HandPose> = {
  open_hand: { thumb: 'spread', index: true, middle: true, ring: true, pinky: true },
  fist: { thumb: 'tucked', index: false, middle: false, ring: false, pinky: false },
  thumb_up: { thumb: 'up', index: false, middle: false, ring: false, pinky: false },
  one_finger: { thumb: 'tucked', index: true, middle: false, ring: false, pinky: false },
  two_fingers: { thumb: 'tucked', index: true, middle: true, ring: false, pinky: false },
}
