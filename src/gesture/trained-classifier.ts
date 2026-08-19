/**
 * Erkennung aus eingelernten Aufnahmen ("Training") statt aus festen Regeln.
 *
 * Verfahren: nächster Nachbar im Merkmalsraum. Für jede eingelernte Geste
 * liegen mehrere aufgenommene Merkmalsvektoren vor; eine neue Handhaltung wird
 * der Geste mit der kleinsten Distanz zugeordnet. Bewusst kein neuronales Netz:
 *
 * * nachvollziehbar - jede Entscheidung lässt sich an einer Aufnahme festmachen,
 * * kein zusätzliches Modell, keine Trainingszeit, kein weiterer Fremdcode,
 * * schon ab wenigen Aufnahmen brauchbar.
 *
 * Die Kennungen sind dieselben wie in der Datenbank: Grundgesten heißen
 * `open_hand`, `fist`, …, eigene Gesten `custom:<id>`.
 */

import { featureVector, FEATURE_LEN } from './features.ts'
import type { Classification, Gesture, Point } from './gesture-classifier.ts'
import { classify as classifyGeometric } from './gesture-classifier.ts'

export interface TrainingSample {
  gesture: string
  features: number[]
}

export interface TrainedModel {
  /** Aufnahmen je Kennung. */
  prototypes: Map<string, number[][]>
  /** Gewicht je Merkmal: streuende Merkmale zählen weniger. */
  weights: number[]
}

/** Mindestzahl an Aufnahmen, damit eine Geste mitspielt. */
export const MIN_SAMPLES = 8

/** Ab dieser gewichteten Distanz passt die Haltung zu keiner Aufnahme mehr. */
const MAX_DISTANCE = 2.2

export const BUILTIN_GESTURES: Gesture[] = [
  'open_hand',
  'fist',
  'thumb_up',
  'one_finger',
  'two_fingers',
]

const clamp01 = (value: number) => Math.min(1, Math.max(0, value))
const ramp = (value: number, low: number, high: number) => clamp01((value - low) / (high - low))

/**
 * Sortiert die Aufnahmen nach Kennung. `only` beschränkt auf bestimmte
 * Kennungen - gebraucht, wenn nur die eigenen Gesten eingelernt sind.
 */
export function buildModel(samples: TrainingSample[], only?: string[]): TrainedModel | null {
  const prototypes = new Map<string, number[][]>()

  for (const sample of samples) {
    if (sample.features.length !== FEATURE_LEN) continue
    if (only && !only.includes(sample.gesture)) continue
    const list = prototypes.get(sample.gesture) ?? []
    list.push(sample.features)
    prototypes.set(sample.gesture, list)
  }

  // Zu dünn eingelernte Kennungen fliegen raus, statt schlecht zu raten.
  for (const [label, list] of [...prototypes]) {
    if (list.length < MIN_SAMPLES) prototypes.delete(label)
  }
  if (prototypes.size === 0) return null

  return { prototypes, weights: featureWeights([...prototypes.values()].flat()) }
}

/** Sind alle Grundgesten ausreichend eingelernt? */
export function builtinsTrained(samples: TrainingSample[]): boolean {
  const counts = new Map<string, number>()
  for (const sample of samples) {
    counts.set(sample.gesture, (counts.get(sample.gesture) ?? 0) + 1)
  }
  return BUILTIN_GESTURES.every((gesture) => (counts.get(gesture) ?? 0) >= MIN_SAMPLES)
}

/**
 * Merkmale mit großer Streuung über alle Aufnahmen sind weniger aussagekräftig
 * und bekommen weniger Gewicht (1/Streuung). Das hält die Distanz über
 * unterschiedlich skalierte Merkmale hinweg vergleichbar.
 */
function featureWeights(samples: number[][]): number[] {
  const weights: number[] = []
  for (let index = 0; index < FEATURE_LEN; index++) {
    const values = samples.map((sample) => sample[index])
    const mean = values.reduce((sum, value) => sum + value, 0) / values.length
    const variance =
      values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / Math.max(values.length - 1, 1)
    // Untergrenze, damit ein zufällig konstantes Merkmal nicht alles dominiert.
    weights.push(1 / Math.max(Math.sqrt(variance), 0.05))
  }
  return weights
}

function weightedDistance(a: number[], b: number[], weights: number[]): number {
  let sum = 0
  for (let index = 0; index < FEATURE_LEN; index++) {
    const delta = (a[index] - b[index]) * weights[index]
    sum += delta * delta
  }
  return Math.sqrt(sum / FEATURE_LEN)
}

/** Kleinste Distanz zu einer Aufnahme dieser Kennung. */
function nearest(model: TrainedModel, label: string, features: number[]): number {
  let best = Number.POSITIVE_INFINITY
  for (const sample of model.prototypes.get(label) ?? []) {
    best = Math.min(best, weightedDistance(features, sample, model.weights))
  }
  return best
}

/**
 * Ordnet eine Handhaltung anhand des Trainings zu.
 *
 * Die Fingerwerte im Ergebnis kommen weiter aus der geometrischen Auswertung -
 * sie dienen der Anzeige in der Vorschau und sind unabhängig davon, wer die
 * Entscheidung getroffen hat.
 */
export function classifyTrained(
  model: TrainedModel,
  points: Point[],
  world?: Point[] | null,
): Classification {
  const geometric = classifyGeometric(points, world)
  if (!points || points.length < 21) return geometric

  const features = featureVector(points, world)
  const ranked = [...model.prototypes.keys()]
    .map((label) => [label, nearest(model, label, features)] as [string, number])
    .sort((a, b) => a[1] - b[1])

  const [label, best] = ranked[0]
  const runnerUp = ranked[1]?.[1] ?? Number.POSITIVE_INFINITY

  // Passt die Haltung überhaupt zu einer Aufnahme?
  const fit = 1 - ramp(best, MAX_DISTANCE * 0.35, MAX_DISTANCE)
  // Und ist sie von der zweitbesten Geste deutlich getrennt?
  const separation = Number.isFinite(runnerUp)
    ? ramp((runnerUp - best) / Math.max(runnerUp + best, 1e-6), 0.05, 0.35)
    : 1

  return {
    gesture: BUILTIN_GESTURES.includes(label as Gesture) ? (label as Gesture) : null,
    label,
    confidence: clamp01(fit * (0.55 + 0.45 * separation)),
    fingers: geometric.fingers,
    thumb: geometric.thumb,
  }
}
