/**
 * Wählt die Erkennungsart und hält die Vorrangregeln an einer Stelle - damit
 * Overlay und Vorschau garantiert dasselbe erkennen.
 *
 * Drei Betriebsarten:
 *
 * | Art       | wann | wie |
 * |-----------|------|-----|
 * | `geometric` | Standard | feste geometrische Regeln |
 * | `trained`   | Training eingeschaltet und alle Grundgesten eingelernt | ein Modell über alle Kennungen, auch die eigenen Gesten |
 * | `hybrid`    | eigene Gesten vorhanden, Grundgesten aber nicht eingelernt | Grundgesten nach Regeln; erkennt keine sicher, kommen die eigenen Gesten zum Zug |
 *
 * Im Mischbetrieb haben die Grundgesten bewusst Vorrang: eine eigene Geste soll
 * niemals „Stopp" verdecken können.
 */

import type { AppSettings } from '../api/backend.ts'
import {
  classify as classifyGeometric,
  type Classification,
  type Point,
} from './gesture-classifier.ts'
import { FEATURE_VERSION } from './features.ts'
import {
  BUILTIN_GESTURES,
  buildModel,
  builtinsTrained,
  classifyTrained,
  type TrainingSample,
} from './trained-classifier.ts'

export type RecognizerMode = 'geometric' | 'trained' | 'hybrid'

export interface Recognizer {
  mode: RecognizerMode
  /** `world` sind MediaPipes metrische Weltkoordinaten, falls vorhanden. */
  classify(points: Point[], world?: Point[] | null): Classification
}

export interface TrainingData {
  version: number
  samples: TrainingSample[]
}

type RecognizerSettings = Pick<AppSettings, 'use_training'> &
  Partial<Pick<AppSettings, 'confidence_threshold'>>

const isCustom = (label: string) => label.startsWith('custom:')

/**
 * Baut den Erkenner. Passt das Training nicht (aus, unvollständig, veralteter
 * Merkmalssatz), gelten die geometrischen Regeln - die Erkennung fällt also nie
 * stillschweigend aus.
 */
export function createRecognizer(
  settings: RecognizerSettings,
  training?: TrainingData | null,
): Recognizer {
  const geometric: Recognizer = { mode: 'geometric', classify: classifyGeometric }
  if (!training || training.version !== FEATURE_VERSION) return geometric

  const samples = training.samples
  const threshold = settings.confidence_threshold ?? 0.85

  if (settings.use_training && builtinsTrained(samples)) {
    const model = buildModel(samples)
    if (model) {
      return {
        mode: 'trained',
        classify: (points, world) => classifyTrained(model, points, world),
      }
    }
  }

  // Eigene Gesten funktionieren auch ohne vollständig eingelernte Grundgesten -
  // dann aber nur nachrangig.
  const customLabels = [...new Set(samples.map((sample) => sample.gesture))].filter(isCustom)
  const customModel = customLabels.length ? buildModel(samples, customLabels) : null
  if (!customModel) return geometric

  return {
    mode: 'hybrid',
    classify(points, world) {
      const builtin = classifyGeometric(points, world)
      // Eine sicher erkannte Grundgeste gewinnt immer.
      if (builtin.gesture && builtin.confidence >= threshold) return builtin

      const custom = classifyTrained(customModel, points, world)
      return custom.confidence > builtin.confidence ? custom : builtin
    },
  }
}

export { BUILTIN_GESTURES }
