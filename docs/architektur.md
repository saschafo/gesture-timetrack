# Architektur

## Aufteilung

```
src-tauri/                Rust: Zustand, Daten, System-Integration
├── src/lib.rs            Aufbau der App, Plugins, Fenster-Events, Ticker
├── src/state.rs          Zustandsmaschine, Sitzung, Einstellungen, Snapshot
├── src/commands.rs       Tauri-Commands + Aktionen (Start/Stopp/Pause/Slot)
├── src/camera.rs         optionale Netzwerk-Kamera (MJPEG im WLAN)
├── src/db.rs             SQLite (rusqlite), Migration, Abfragen
├── src/export.rs         CSV-Export
├── src/hotkey.rs         globaler Hotkey
├── src/overlay.rs        Overlay-Fenster (Position, Anzeigen/Verbergen)
└── src/tray.rs           Tray-Icon, Menü, Status

src/                      Vue 3: Anzeige und Gestenerkennung
├── gesture/              MediaPipe-Anbindung, Klassifikator, Training (+ Tests)
├── components/           Hauptfenster-Karten und Overlay
├── stores/timetrack.ts   Pinia-Store (spiegelt den Backend-Zustand)
└── api/backend.ts        typisierte Hülle um die Commands
```

## Ein Zustand, drei Auslöser

Geste, Tray-Menü und Hauptfenster rufen dieselben Funktionen in
`commands.rs` auf. Der Zustand (`Tracker`) liegt ausschließlich im Backend;
jede Änderung sendet einen `Snapshot` per Event `tracker:state` an alle Fenster
und aktualisiert das Tray. Das Frontend zählt zwischen zwei Snapshots nur lokal
die Sekunden hoch, damit die Uhr flüssig läuft.

## Fenster

| Fenster | Rolle |
|---|---|
| `main` | Projekte, Slots, Vorschau samt Gestentraining, Auswertung, Einstellungen. Schließen versteckt nur – die App lebt im Tray weiter, sonst wäre der Hotkey weg. |
| `overlay` | 220 × 250 px, randlos, immer im Vordergrund, oben rechts, **ohne Fokus** (`focus: false`) – der Nutzer bleibt in seinem Arbeitsfenster. Normalerweise versteckt. |
| `panel` | 268 px breit, randlos, unter dem Symbol in der Menüleiste. Hier ist Fokus **gewollt**: nur damit lässt sich das Fenster beim Klick daneben wieder schließen (`WindowEvent::Focused(false)`). Die Höhe folgt dem Inhalt (`resize_panel`). |

## Bildquellen

Die Erkennung kennt nur die Schnittstelle `FrameSource`
(`src/gesture/frame-source.ts`) und bekommt pro Takt ein fertiges Bild:

| Quelle | Weg |
|---|---|
| `WebcamSource` (Standard) | `getUserMedia` → `<video>` → direkt an MediaPipe |
| `NetworkCameraSource` | Rust holt den MJPEG-Stream → Einzelbilder per IPC → `createImageBitmap` → MediaPipe, Anzeige über ein Canvas |

Beide Quellen bedienen Overlay **und** Vorschau im Hauptfenster. Zur Kamera im
Netz besteht dabei immer nur **eine** Verbindung: verbreitete Kamera-Apps
bedienen nur einen Stream-Client und antworten dem zweiten mit ihrer
Bedienseite. Solange die Vorschau läuft, hält sie die Verbindung
(`set_camera_preview`); das Schließen des Overlays beendet sie dann nicht.
Nach einem Fehlversuch wird die Wartezeit verdoppelt (0,7 s bis 3 s) – zu
schnelles Nachfragen würde den einzigen Stream-Platz dauerhaft blockieren.

Der Umweg über Rust ist kein Selbstzweck: ein fremder Stream ist im Webview eine
Cross-Origin-Bildquelle, die WebGL nicht in eine Textur laden darf. So bleibt
außerdem die CSP unverändert eng.

Das Zeitfenster hängt an der Quelle (`overlay_timeout_ms` bzw.
`overlay_timeout_network_ms`, Standard 3 s / 4 s) und beginnt erst mit dem
ersten eintreffenden Bild – WLAN-Latenz geht so nicht von der Erkennungszeit ab.

## Zeitrechnung

* Zeitstempel liegen als lokale Zeit (`YYYY-MM-DD HH:MM:SS`) in der Datenbank,
  damit `date(start_ts)` für Tagesauswertungen direkt funktioniert.
* Gespeichert wird die **Nettozeit**: Bruttodauer minus Pausen. Negative Werte
  sind ausgeschlossen (relevant bei Zeitumstellung).
* Pausenzeiten werden beim Fortsetzen sofort in die Datenbank geschrieben,
  damit ein Absturz sie nicht verschluckt.

## Fensteraufbau

Das Hauptfenster liegt auf einem Raster über zwölf Spalten
([App.vue](../src/App.vue)), begrenzt auf 1500 px Breite - darüber werden
Textzeilen unangenehm lang.

| Reihe | Aufteilung |
|---|---|
| Statuskarte | volle Breite, mit Gestenübersicht im freien Raum neben der Uhr |
| Arbeiten | Projekte 4 · Auswertung 8 |
| Werkzeuge | Kamera-Vorschau 7 · Einstellungen 5 |

Die Gestenübersicht hat keine eigene Karte mehr: Sie ist Nachschlage-Information
und sitzt als drei Gegensatzpaare (Start/Stopp, Pause/Weiter, Slot 1/Slot 2) in
der Statuskarte, wo ohnehin Platz frei war. Die Erklärung zum Slot-Wechsel steht
bei den Projekten - dort, wo die Slots eingestellt werden.

Zwei Karten passen ihre innere Aufteilung an die **Kartenbreite** an, nicht an die
Fensterbreite (`container-type: inline-size`): die Vorschau (Bild neben Training)
und die Auswertung (Werkzeugleiste neben oder unter dem Titel). Eine
Fensterabfrage wäre hier falsch - eine Karte über sieben Spalten ist auch in
einem breiten Fenster schmal. Für Webviews ohne Container-Abfragen gibt es einen
`@supports`-Ausweg.

Unter 1100 px stehen alle Karten untereinander.

Wichtig für die Anordnung: Die Karten hängen **nicht** in zwei festen Spalten
untereinander. Sonst entsteht Leerraum, sobald auf einer Seite eine Karte
ausgeklappt wird - genau das war vorher der Fall.

## Sprachen

Zwei Wörterbücher, eines je Seite:

* Frontend [src/i18n.ts](../src/i18n.ts) - `t('kennung', { platzhalter })`, die
  Sprache liegt in einem `ref`, damit ein Wechsel die Oberfläche sofort umstellt.
* Backend [src-tauri/src/i18n.rs](../src-tauri/src/i18n.rs) - eine Tabelle
  `(Kennung, deutsch, englisch)`.

Im Backend entstehen Meldungen an Dutzenden Stellen, oft ohne Zugriff auf die
Datenbank. Statt die Sprache durch jede Funktion zu fädeln, hält `i18n.rs` sie
als Prozesszustand; gesetzt wird sie beim Start und beim Umschalten. Fehler
tragen deshalb keine Texte mehr, sondern **Kennungen**
(`AppError::key("track.not_running")`) - übersetzt wird erst beim Anzeigen, also
in der Sprache, die dann gilt.

Beide Seiten haben Tests, die vergleichen: gleiche Kennungen, keine leeren
Einträge, gleiche Platzhalter. Eine fehlende Übersetzung fällt damit im Test auf,
nicht beim Kunden.

**Zwei Quellen, eine Wahrheit.** Die Oberfläche liest die Sprache aus den
Einstellungen (Datenbank), die Backend-Meldungen aus dem Prozesszustand. Beide
konnten auseinanderlaufen - beobachtet als englischer Statustext in einer
deutschen Oberfläche. Deshalb gleicht `get_settings` den Prozesszustand jetzt bei
jedem Aufruf mit der Datenbank ab; das passiert bei jedem Fensterstart und nach
jeder Änderung. Im Entwicklungsmodus schreibt der Start zusätzlich die erkannte
Sprache ins Log.

## Mehrere Bildschirme

Ein eigenes Modul ([screens.rs](../src-tauri/src/screens.rs)), weil hier zwei
Fallen zusammenkommen.

**Falle 1 – falscher Bildschirm.** `current_monitor()` liefert den Bildschirm,
auf dem ein Fenster *zuletzt stand*. Für ein normalerweise verstecktes Fenster
ist das beliebig. Maßgeblich ist stattdessen der Bildschirm unter dem
**Mauszeiger**: dort arbeitet der Nutzer, und beim Klick auf das
Menüleisten-Symbol steht der Zeiger ohnehin genau dort.

**Falle 2 – gemischte Koordinaten.** Unter macOS liefert Tauri

* Mauszeiger und die Lage des Menüleisten-Symbols in Pixeln des
  **Hauptbildschirms** (Punkte × dessen Skalierung),
* die Position der Bildschirme dagegen in **Punkten**,
* deren Größe wieder **physisch**.

Mit einem Retina- und einem gewöhnlichen Bildschirm ergibt jede Mischung
Unsinn: ein Mauszeiger bei Punkt 1596 wird als 3193 gemeldet und landet damit im
Bereich des rechten Bildschirms. Genau so gingen Overlay und Menüleisten-Fenster
auf dem falschen Monitor auf.

`screens.rs` rechnet deshalb alles in eine Einheit um - Punkte unter macOS,
physische Pixel sonst - und gibt die Position erst beim Setzen plattformgerecht
weiter. Der Bildschirm zu einem Punkt wird selbst gesucht
(`monitor_at`), weil `monitor_from_point` physische Pixel erwartet.

Im Entwicklungsmodus schreibt die Anwendung die erkannte Anordnung und jede
Platzierung mit (`[screens]`, `[panel]`, `[overlay]`) - ohne diese Zahlen ist ein
solcher Fehler aus der Ferne nicht zu finden.

## Tray

Die Menüleiste bekommt zwei verschiedene Aktualisierungen: jede Sekunde nur
Uhrzeit und Kurzinfo (`tray::set_clock`), alle 30 Sekunden bzw. bei jeder
Zustandsänderung zusätzlich das Menü (`tray::refresh`). Ein Menüneubau pro
Sekunde wäre Verschwendung, und die Fenster zählen ohnehin selbst weiter.

Beide Wege laufen über `AppHandle::run_on_main_thread`: unter macOS lässt sich
die Menüleiste nur vom Haupt-Thread zuverlässig ändern - aus dem Uhren-Thread
heraus blieb die Anzeige stehen.

Das Menü enthält ein Untermenü mit allen aktiven Projekten (Kennung
`project:<id>`), das je nach Zustand startet oder wechselt.

Ein **Linksklick** auf das Symbol öffnet stattdessen das Fenster `panel`
(`src-tauri/src/panel.rs`). Positioniert wird es aus dem Rechteck, das das
Tray-Ereignis mitliefert: mittig unter dem Symbol, an den Bildschirmrändern
begrenzt. Die Maße kommen je Plattform logisch oder physisch, deshalb wird erst
beim Positionieren mit dem Skalierungsfaktor umgerechnet.

## Projektwechsel

Ein Wechsel ist aus jedem Zustand möglich - gestoppt, laufend und pausiert. Der
offene Eintrag wird mit seiner Nettozeit geschlossen, der neue beginnt
unmittelbar; es entstehen also zwei Einträge, die sich in der Auswertung
summieren. Ausgelöst wird das über denselben Pfad, egal ob per Fingergeste,
Tray-Menü oder Klick in der Projektliste.

## Nachträgliche Änderungen

Einträge können bearbeitet, gelöscht und von Hand angelegt werden. Die Prüfung
sitzt in `state::plan_entry` (Ende nach Beginn, Pause kürzer als die Gesamtzeit,
Nettozeit berechnet) - also an einer Stelle, unabhängig davon, aus welchem
Formular die Werte kommen. Von Hand erfasste Einträge bekommen
`gesture_triggered = 0` und sind in der Auswertung als „manuell" erkennbar.

Der offene Eintrag ist gesperrt (`ensure_not_running`): er gehört der
Zustandsmaschine, und eine Änderung nebenher würde Uhr und Datenbank
auseinanderlaufen lassen.

## Absturz-Wiederherstellung

Beim Start sucht `recover_session` einen Eintrag ohne Endzeit:

* jünger als 12 Stunden → Sitzung wird **pausiert** wiederhergestellt, der
  Nutzer entscheidet über Fortsetzen oder Stoppen;
* älter → Eintrag wird mit Dauer 0 geschlossen, statt eine durchgearbeitete
  Nacht zu erfinden.

## Bewusste Grenzen

* Keine Netzwerk-, Shell- oder allgemeinen Dateisystem-Rechte (siehe
  `capabilities/default.json`).
* Kein Auto-Update im MVP.
* Keine Migrationsversionierung über `CREATE TABLE IF NOT EXISTS` hinaus – beim
  ersten Schemabruch gehört eine echte Migrationsliste her.
