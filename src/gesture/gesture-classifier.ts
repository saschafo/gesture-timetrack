/**
 * Ordnet MediaPipe-Handlandmarks den Gesten des MVP zu.
 *
 * Bewusst geometrisch statt trainiert: Das Verhalten ist nachvollziehbar, ohne
 * zusätzliche Modelldatei, und jede Erkennung liefert einen echten stetigen
 * Konfidenzwert zwischen 0 und 1 statt eines Ja/Nein.
 *
 * Alle Maße sind bewusst so gewählt, dass sie weder von der Handgröße noch vom
 * Abstand zur Kamera noch von der Fingerlänge abhängen - sonst bewertet man
 * einen gestreckten kleinen Finger schlechter als einen gestreckten
 * Mittelfinger, und die offene Hand fällt an ihrem schwächsten Glied durch.
 */

/**
 * Die Gesten des Vokabulars. Beschriftungen dazu stehen im Wörterbuch
 * (`gesture.*` und `gestureAction.*`) - sie sind Sache der Oberfläche.
 */
export type Gesture =
  | 'open_hand'
  | 'fist'
  | 'thumb_up'
  | 'one_finger'
  | 'two_fingers'

export interface Point {
  x: number
  y: number
  /** Tiefe. Bei MediaPipes Weltkoordinaten in Metern, sonst bildbezogen. */
  z: number
}

/** Streckungsgrad je Finger, 0 = eingeklappt, 1 = gestreckt. */
export interface FingerScores {
  index: number
  middle: number
  ring: number
  pinky: number
}

/** Wie deutlich zeigt der Daumen nach oben? */
export interface ThumbSignal {
  up: number
}

export interface Classification {
  /** Grundgeste - bei einer eigenen Geste `null`. */
  gesture: Gesture | null
  /**
   * Kennung des Ergebnisses: eine Grundgeste (`open_hand`, …) oder eine eigene
   * Geste (`custom:<id>`). Dieselbe Schreibweise wie in der Datenbank.
   */
  label: string | null
  confidence: number
  fingers: FingerScores
  thumb: ThumbSignal
}

/** Kennung einer eigenen Geste - Gegenstück zu state::custom_sample_key. */
export const customLabel = (id: number) => `custom:${id}`

/** Liest die Kennung wieder aus; `null` bei einer Grundgeste. */
export function parseCustomLabel(label: string | null | undefined): number | null {
  const raw = label?.startsWith('custom:') ? label.slice('custom:'.length) : null
  if (raw === null) return null
  const id = Number.parseInt(raw, 10)
  return Number.isFinite(id) ? id : null
}

const WRIST = 0
const THUMB_TIP = 4
const INDEX_MCP = 5
const MIDDLE_MCP = 9

/** Fingerglieder: [Name, MCP, PIP, DIP, TIP] */
const FINGERS: Array<[keyof FingerScores, number, number, number, number]> = [
  ['index', 5, 6, 7, 8],
  ['middle', 9, 10, 11, 12],
  ['ring', 13, 14, 15, 16],
  ['pinky', 17, 18, 19, 20],
]

const FINGER_NAMES = FINGERS.map(([name]) => name)

/**
 * Ein Kandidat gilt als ausgeschlossen, wenn ein einzelner Messwert klar
 * dagegen spricht - unabhängig davon, wie gut der Rest passt.
 */
const VETO = 0.5

const clamp01 = (value: number) => Math.min(1, Math.max(0, value))

/**
 * Abstand im Raum, ausdrücklich mit z.
 *
 * Nur zwei Dimensionen zu nehmen wäre eine Falle: ein eingeklappter Finger
 * hinter der Handfläche erscheint im Bild kurz und gerade und wird dann für
 * gestreckt gehalten. Genau daran lag es, dass ein einzelner Finger als offene
 * Hand gelesen wurde.
 */
const distance = (a: Point, b: Point) =>
  Math.hypot(a.x - b.x, a.y - b.y, (a.z ?? 0) - (b.z ?? 0))

/** Abstand in der Bildebene - für Richtungsfragen, die die Ansicht betreffen. */
const distance2d = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y)

/** Weiche Rampe: unterhalb `low` = 0, oberhalb `high` = 1. */
const ramp = (value: number, low: number, high: number) => clamp01((value - low) / (high - low))

/** Referenzlänge der Hand: Handgelenk bis Mittelfinger-Grundgelenk. */
const handScale = (points: Point[]) =>
  Math.max(distance(points[WRIST], points[MIDDLE_MCP]), 1e-6)

/** Dieselbe Referenzlänge, aber in der Bildebene. */
const handScale2d = (points: Point[]) =>
  Math.max(distance2d(points[WRIST], points[MIDDLE_MCP]), 1e-6)

/**
 * Streckung je Finger, aus zwei voneinander unabhängigen Messungen.
 *
 * 1. **Geradheit**: Verhältnis von „Luftlinie Grundgelenk → Fingerspitze" zu
 *    „Länge des Fingers entlang seiner Glieder". Gestreckt ≈ 1, eingeklappt
 *    deutlich kleiner. Dimensionslos, also unabhängig von Handgröße,
 *    Kameraabstand und Fingerlänge.
 * 2. **Reichweite**: Liegt die Fingerspitze weiter vom Handgelenk entfernt als
 *    das Mittelgelenk? Bei eingeklappten Fingern wandert die Spitze zurück zur
 *    Handfläche.
 *
 * Gezählt wird das Minimum: ein Finger gilt nur als gestreckt, wenn **beide**
 * Messungen dafür sprechen. Einzeln lässt sich jede täuschen - zusammen kaum.
 */
export function fingerScores(points: Point[]): FingerScores {
  const scores: FingerScores = { index: 0, middle: 0, ring: 0, pinky: 0 }
  const scale = handScale(points)
  const wrist = points[WRIST]

  for (const [name, mcp, pip, dip, tip] of FINGERS) {
    const span = distance(points[tip], points[mcp])
    const chain =
      distance(points[mcp], points[pip]) +
      distance(points[pip], points[dip]) +
      distance(points[dip], points[tip])
    const straight = ramp(span / Math.max(chain, 1e-6), 0.68, 0.9)

    const reach = (distance(points[tip], wrist) - distance(points[pip], wrist)) / scale
    scores[name] = Math.min(straight, ramp(reach, 0.05, 0.35))
  }
  return scores
}

/**
 * Daumenhaltung: zeigt der Daumen deutlich nach oben?
 *
 * Bezugspunkt ist das Zeigefinger-Grundgelenk, nicht das Handgelenk: Bei einer
 * senkrecht gehaltenen Faust liegt der Daumen ebenfalls über dem Handgelenk,
 * aber eben auf Höhe der Knöchel. Erst der Abstand dorthin trennt Faust und
 * Daumengeste zuverlässig.
 *
 * Gemessen wird in der Bildebene - „oben" ist eine Aussage über die Ansicht.
 * Absichtlich nur diese eine Größe: Wie weit der Daumen **abgespreizt** ist,
 * lässt sich nicht verlässlich messen (kleiner Finger, häufig von der Hand
 * verdeckt). Gesten, die daran hingen, sind deshalb aus dem Vokabular
 * verschwunden.
 */
export function thumbSignal(points: Point[]): ThumbSignal {
  const rise = (points[INDEX_MCP].y - points[THUMB_TIP].y) / handScale2d(points)
  return { up: ramp(rise, 0.25, 0.6) }
}

type Pattern = Partial<Record<keyof FingerScores, 0 | 1>>

/**
 * Bewertet ein Sollmuster.
 *
 * Gemittelt wird über alle geforderten Merkmale - ein leicht gebeugter kleiner
 * Finger soll die offene Hand nicht sofort verwerfen. Ein Merkmal, das klar
 * widerspricht, hat aber Vetorecht: dann bleibt die Konfidenz auf jeden Fall
 * unter jeder sinnvollen Schwelle.
 */
function score(scores: FingerScores, pattern: Pattern, extra?: number): number {
  const values: number[] = []
  for (const [finger, expected] of Object.entries(pattern) as Array<[keyof FingerScores, 0 | 1]>) {
    values.push(expected === 1 ? scores[finger] : 1 - scores[finger])
  }
  if (extra !== undefined) values.push(extra)

  const mean = values.reduce((sum, value) => sum + value, 0) / values.length
  const weakest = Math.min(...values)
  return weakest < VETO ? Math.min(mean, VETO) : mean
}

const ALL_CURLED: Pattern = { index: 0, middle: 0, ring: 0, pinky: 0 }

/**
 * Klassifiziert eine Hand. Zurück kommt die beste Übereinstimmung mit ihrem
 * Konfidenzwert - ob der reicht, entscheidet der Aufrufer anhand des
 * konfigurierten Schwellwerts.
 */
/**
 * Klassifiziert eine Hand.
 *
 * `world` sind MediaPipes metrische Weltkoordinaten. Sie werden für alles
 * verwendet, was die Form der Hand betrifft: Sie sind frei von Perspektive und
 * vom Seitenverhältnis des Bildes - bei einem 16:9-Handybild sind normierte
 * Bildkoordinaten in x und y unterschiedlich gestreckt. Für Richtungsfragen
 * („Daumen hoch") bleiben die Bildkoordinaten maßgeblich.
 */
export function classify(points: Point[], world?: Point[] | null): Classification {
  if (!points || points.length < 21) {
    return {
      gesture: null,
      label: null,
      confidence: 0,
      fingers: { index: 0, middle: 0, ring: 0, pinky: 0 },
      thumb: { up: 0 },
    }
  }

  const shape = world && world.length >= 21 ? world : points
  const fingers = fingerScores(shape)
  const thumb = thumbSignal(points)

  // Der Daumen geht nur dort ein, wo er die Geste ausmacht. Für die offene Hand
  // zählen die vier Finger: ob der Daumen dabei absteht, ist für die Absicht
  // des Nutzers unerheblich, für die Messung aber die unsicherste Größe.
  const candidates: Array<[Gesture, number]> = [
    ['open_hand', score(fingers, { index: 1, middle: 1, ring: 1, pinky: 1 })],
    // Faust: alles eingeklappt und der Daumen zeigt nicht nach oben. Ob er dabei
    // anliegt oder etwas absteht, bleibt offen - das ist nicht messbar genug,
    // um daran eine Geste zu hängen.
    ['fist', score(fingers, ALL_CURLED, 1 - thumb.up)],
    ['thumb_up', score(fingers, ALL_CURLED, thumb.up)],
    ['one_finger', score(fingers, { index: 1, middle: 0, ring: 0, pinky: 0 })],
    ['two_fingers', score(fingers, { index: 1, middle: 1, ring: 0, pinky: 0 })],
  ]

  candidates.sort((a, b) => b[1] - a[1])
  const [gesture, confidence] = candidates[0]

  // Liegen die zwei besten Kandidaten dicht beieinander, ist die Handhaltung
  // mehrdeutig - das drückt die Konfidenz, statt zu raten.
  const margin = ramp(confidence - candidates[1][1], 0, 0.12)
  return {
    gesture,
    label: gesture,
    confidence: clamp01(confidence * (0.8 + 0.2 * margin)),
    fingers,
    thumb,
  }
}

export { FINGER_NAMES }

/**
 * Glättet über mehrere Frames: erst wenn dieselbe Geste `frames` Mal in Folge
 * über dem Schwellwert liegt, gilt sie als erkannt. Verhindert, dass eine
 * zufällige Zwischenhaltung beim Heben der Hand eine Aktion auslöst.
 */
export class GestureStabilizer {
  /** Kennung der aktuell beobachteten Geste (Grundgeste oder `custom:<id>`). */
  private current: string | null = null
  private gesture: Gesture | null = null
  private values: number[] = []
  private readonly threshold: number
  private readonly frames: number

  constructor(threshold: number, frames = 3) {
    this.threshold = threshold
    this.frames = frames
  }

  /** Gibt die bestätigte Geste samt Durchschnittskonfidenz zurück - oder null. */
  push(
    result: Classification,
  ): { gesture: Gesture | null; label: string; confidence: number } | null {
    if (!result.label || result.confidence < this.threshold) {
      this.reset()
      return null
    }
    if (result.label !== this.current) {
      this.current = result.label
      this.gesture = result.gesture
      this.values = []
    }
    this.values.push(result.confidence)
    if (this.values.length < this.frames) return null

    const confidence = this.values.reduce((sum, value) => sum + value, 0) / this.values.length
    const confirmed = { gesture: this.gesture, label: this.current, confidence }
    this.reset()
    return confirmed
  }

  reset() {
    this.current = null
    this.gesture = null
    this.values = []
  }
}
