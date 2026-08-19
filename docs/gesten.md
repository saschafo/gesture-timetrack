# Gestenerkennung im Detail

## Kette

```
Hotkey → Overlay sichtbar → Bildquelle an → pro Frame:
  MediaPipe Hand Landmarker  →  21 Landmarks
  gesture-classifier.ts      →  Geste + Konfidenz
  GestureStabilizer          →  3 gleiche Frames in Folge über der Schwelle
  Backend apply_gesture      →  Zustandsmaschine + Buchung
→ Rückmeldung (Rahmen/Ton) → Kamera aus → Overlay zu
```

## Warum ein eigener Klassifikator?

MediaPipe liefert die Landmarks, nicht die Bedeutung. Die Zuordnung geschieht
geometrisch in [`src/gesture/gesture-classifier.ts`](../src/gesture/gesture-classifier.ts):

1. **Streckungsgrad je Finger** (0 bis 1, stetig): Verhältnis von *Luftlinie
   Grundgelenk → Fingerspitze* zu *Fingerlänge entlang der Glieder*. Bei
   gestrecktem Finger sind beide Wege gleich lang (Verhältnis ≈ 1), bei
   eingeklapptem ist die Luftlinie deutlich kürzer.

   Dieses Verhältnis ist dimensionslos und hängt daher weder an der Handgröße,
   noch am Kameraabstand, noch an der **Fingerlänge**. Der letzte Punkt war ein
   echter Fehler in der ersten Fassung: normiert auf die Handgröße erreicht der
   kurze kleine Finger nie den Wert eines gestreckten Mittelfingers – die offene
   Hand wurde dadurch praktisch nie erkannt. Dagegen gibt es jetzt einen Test.
2. **Daumenhaltung**: nur eine Größe – die Höhe der Daumenspitze gegenüber dem
   Zeigefinger-Grundgelenk (Bildebene, y wächst nach unten). Bezugspunkt ist
   nicht das Handgelenk: bei einer senkrecht gehaltenen Faust liegt der Daumen
   ebenfalls darüber, aber eben auf Höhe der Knöchel.

   Wie weit der Daumen **abgespreizt** ist, wird bewusst nicht ausgewertet – der
   Daumen ist klein und häufig von der Hand verdeckt. Für die Faust heißt das:
   die Daumenlage spielt keine Rolle, sie wird auch mit abstehendem Daumen
   erkannt.

   Dahinter stecken zwei verworfene Versuche für *Weiter*: „Daumen runter"
   (unbequem, die ganze Hand muss sich drehen) und „Daumen seitlich" (nicht
   verlässlich von einer Faust mit lose abstehendem Daumen zu trennen). Die
   Lösung war, die Geste ganz zu streichen: Die **offene Hand** setzt eine Pause
   fort, weil „Start" und „Weiter" für den Nutzer dasselbe Anliegen sind. Fünf
   Gesten statt sechs – und die unsicherste Messung fällt weg.

   Für die offene Hand wird der Daumen ebenfalls nicht ausgewertet: für die
   Absicht „Start" ist er unerheblich.

3. **Musterabgleich**: Jede Geste ist ein Sollmuster (z. B. Faust = alles
   eingeklappt). Bewertet wird der **Mittelwert** der geforderten Merkmale, damit
   ein leicht gebeugter Finger die Geste nicht sofort verwirft. Ein Merkmal, das
   klar widerspricht (unter 50 %), hat aber **Vetorecht** – dann bleibt die
   Konfidenz unter jeder sinnvollen Schwelle.
4. **Abstandsstrafe**: Liegen die zwei besten Kandidaten dicht beieinander, ist
   die Haltung mehrdeutig; die Konfidenz sinkt, statt zu raten.

Vorteil gegenüber einem trainierten Zusatzmodell: nachvollziehbares Verhalten,
keine weitere Modelldatei, und ein echter stetiger Konfidenzwert, an dem die
Schwelle aus den Einstellungen greifen kann.

## Schutz vor Fehlauslösungen

* **Schwelle** (Standard 85 %, einstellbar 50–99 %).
* **Zeitliche Stabilisierung**: drei aufeinanderfolgende Frames müssen dieselbe
  Geste zeigen. Die Zwischenhaltungen beim Heben der Hand fallen damit weg.
* **Kein Fallback**: Wird nichts sicher erkannt, passiert nichts. Das Overlay
  meldet „Keine Geste erkannt“ und schließt sich.
* **Zweite Prüfung im Backend**: `apply_gesture` vergleicht die gemeldete
  Konfidenz erneut mit dem Wert aus der Datenbank.
* **Plausibilität**: „Pause“ ohne laufende Erfassung, „Start“ bei bereits
  laufender Erfassung usw. werden abgelehnt und rot zurückgemeldet. Die
  Slot-Gesten sind davon ausgenommen: sie starten aus jedem Zustand heraus, weil
  wer einen Slot wählt, erfassen will.

## Tests

`npm test` prüft den Klassifikator gegen synthetische Handmodelle
([`fixtures.ts`](../src/gesture/fixtures.ts)): jede der sechs Gesten muss über
der Schwelle erkannt werden, Faust und Daumengesten dürfen sich nicht
verwechseln, und eine Zwischenhaltung muss unter der Schwelle bleiben.

## Eingelernte Gesten

Alternativ zu den Regeln kann der Nutzer die Gesten mit seiner eigenen Hand
einlernen (*Kamera-Vorschau → Aufnehmen*).

**Merkmale** ([features.ts](../src/gesture/features.ts)), zehn dimensionslose
Werte je Aufnahme:

| # | Merkmal |
|---|---|
| 0–3 | Streckung von Zeige-, Mittel-, Ring- und kleinem Finger |
| 4 | Streckung des Daumens |
| 5 | Abstand Daumenspitze ↔ Zeigefinger-Grundgelenk |
| 6 | Höhe der Daumenspitze gegenüber den Knöcheln |
| 7–9 | Abstände benachbarter Fingerspitzen |

Merkmal 6 ist absichtlich **nicht** drehinvariant – „Daumen hoch" und „Daumen
runter" unterscheiden sich ausschließlich durch die Richtung im Bild. Alle
anderen sind Verhältnisse und damit unabhängig von Handgröße, Kameraabstand und
Bildausschnitt.

**Erkennung** ([trained-classifier.ts](../src/gesture/trained-classifier.ts)):
nächster Nachbar im gewichteten Merkmalsraum. Merkmale, die über alle Aufnahmen
stark streuen, zählen weniger (Gewicht 1/Streuung). Die Konfidenz entsteht aus
zwei Größen – wie gut die Haltung überhaupt zu einer Aufnahme passt und wie
deutlich sie sich von der zweitbesten Geste abhebt.

Bewusst kein neuronales Netz: nachvollziehbar (jede Entscheidung hängt an einer
konkreten Aufnahme), kein Trainingslauf, keine weitere Modelldatei, brauchbar
schon ab acht Aufnahmen je Geste.

**Rückfall auf die Regeln** ([recognizer.ts](../src/gesture/recognizer.ts)) –
immer dann, wenn das Training nicht eingeschaltet ist, eine Geste weniger als
acht Aufnahmen hat oder die Aufnahmen zu einem älteren Merkmalssatz gehören
(`FEATURE_VERSION`, gespiegelt in `src-tauri/src/state.rs`). Die Erkennung wird
so nie stillschweigend schlechter.

Gespeichert wird in `gesture_samples` (Gestenname, Merkmalsversion,
Merkmalsvektor als JSON) – **keine Bilder**.

## Eigene Gesten

Selbst definierte Gesten liegen in `custom_gestures` (Name, Aktion, optional ein
Projekt); ihre Aufnahmen stehen in `gesture_samples` unter der Kennung
`custom:<id>`. Ausgelöst werden sie über `apply_custom_gesture`, das dieselben
Funktionen aufruft wie die Grundgesten - eine eigene Geste kann also nichts, was
das Tray-Menü nicht auch könnte.

Mögliche Aktionen: Start, Stopp, Pause, Weiter, Projekt-Slot 1, Projekt-Slot 2
sowie „bestimmtes Projekt starten" (mit Projektangabe).

**Vorrang**: Erkennt der Regelweg eine Grundgeste über der Schwelle, gewinnt
diese immer. Eine eigene Geste kann damit keine Grundgeste verdecken - auch
nicht, wenn sie versehentlich ähnlich eingelernt wurde. Dafür gibt es einen
Test.

**Betriebsarten** ([recognizer.ts](../src/gesture/recognizer.ts)):

| Art | wann | wie |
|---|---|---|
| `geometric` | Standard | nur die festen Regeln |
| `trained` | Training eingeschaltet **und** alle Grundgesten eingelernt | ein Modell über alle Kennungen, eigene Gesten inklusive |
| `hybrid` | eigene Gesten vorhanden, Grundgesten nicht eingelernt | Regeln zuerst, eigene Gesten nachrangig |

## Nachjustieren

Die *Kamera-Vorschau* im Hauptfenster zeigt live die erkannte Geste, den
Konfidenzwert mit Schwellenmarke und die Streckung je Finger in Prozent
(Z/M/R/K). Wird eine Geste nicht angenommen, steht dort, woran es liegt – etwa
ein kleiner Finger, der beim „Stopp“ nicht ganz eingeklappt ist.

## Grenzen des MVP

* Eine Hand gleichzeitig (`numHands: 1`).
* Zwei Projekt-Slots. Mehr Projekte lassen sich per Maus starten; für mehr
  Slots wäre eine zweistufige Gestenfolge nötig (siehe Ausblick im Briefing).
* Eine Hand gleichzeitig, keine Gestenfolgen (etwa „zwei Finger, dann Faust").
* Ändert sich das Vokabular, werden Aufnahmen entfallener Gesten beim Start
  entfernt
  (`prune_gesture_samples`) - sonst schlüge die Erkennung eine Geste vor, die das
  Backend nicht mehr kennt.
* Eigene Gesten brauchen Aufnahmen - ohne Training wirken sie nicht.
* Bei einer Netzwerk-Kamera hängt die Trefferquote an der Bildrate des Streams:
  unter etwa 10 Bildern pro Sekunde kommen die drei geforderten gleichen Frames
  nur langsam zusammen. Notfalls das Erkennungsfenster in den Einstellungen
  verlängern.
