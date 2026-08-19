# Datenschutz – was die App tut und was nicht

Das Verkaufsargument gegenüber einem Hardware-Buzzer ist Vertrauen. Deshalb hier
präzise, was technisch passiert.

## Kamera

* Die Kamera wird **ausschließlich** im Erkennungsfenster aktiviert – also nach
  einem bewussten Hotkey-Druck, für die eingestellte Dauer (Standard 3 Sekunden).
* Danach ruft die App `MediaStreamTrack.stop()` für jeden Track auf; die
  Kamera-LED erlischt. Kein Hintergrund-Streaming, kein „Aufwärmen“ der Kamera.
* Es gibt in der gesamten Codebasis keine Stelle, die ein Bild, ein Einzelframe
  oder ein Video schreibt. Die Frames leben im Arbeitsspeicher des Webviews und
  werden vom Browser-Backend überschrieben.
* An das Rust-Backend geht **kein** Bild, sondern ausschließlich das Ergebnis:
  ein Gestenname (`open_hand`, `fist`, …) und ein Konfidenzwert.

## Kamera-Vorschau

Die Vorschau im Hauptfenster ist der einzige Fall, in dem die Kamera länger als
ein Erkennungsfenster läuft. Sie startet nur durch Klick, ist beim Programmstart
aus, und schaltet ab, sobald sie beendet oder ihre Karte eingeklappt wird
(der Inhalt wird dabei abgebaut, nicht bloß versteckt). Auch hier wird kein Bild
gespeichert und keine Zeit gebucht.

## Netzwerk

* Die App enthält keine Telemetrie.
* Die **Update-Prüfung** läuft ausschließlich auf Knopfdruck
  (*Einstellungen → Aktualisierung*), nie im Hintergrund und nie beim Start.
  Abgerufen wird eine Versionsdatei der Veröffentlichungsseite; übertragen wird
  dabei nichts über den Nutzer. Wer nie darauf klickt, hat nie einen
  Netzwerkzugriff aus diesem Grund.
* Ein HTTP-Abruf existiert ausschließlich für die **optionale Netzwerk-Kamera**
  und richtet sich ausschließlich an die vom Nutzer selbst eingetragene Adresse
  (siehe unten). Ohne diese Option findet kein einziger Netzwerkzugriff statt.
* Die Content-Security-Policy erlaubt `connect-src` nur für `self` und die
  Tauri-IPC-Adresse – ein Aufruf nach außen würde vom Webview blockiert.
* Die Tauri-Berechtigungen (`src-tauri/capabilities/default.json`) enthalten
  weder HTTP- noch Shell- noch allgemeine Dateisystem-Rechte. Erlaubt sind nur:
  Speichern-Dialog, globaler Hotkey und der Opener-Standardsatz - letzterer für
  genau zwei Dinge: die exportierte Datei im Dateimanager zeigen und die
  Website-Adresse im **Standardbrowser** öffnen. Die Anwendung selbst ruft dabei
  nichts ab; sie übergibt die Adresse ans Betriebssystem.
* Einziger Netzwerkzugriff im Projekt: `scripts/fetch-assets.mjs` lädt beim
  `npm install` das MediaPipe-Modell und die WASM-Laufzeit herunter. Das ist
  Bauzeit, nicht Laufzeit – im ausgelieferten Bundle liegen beide als Dateien.

## Optionale Netzwerk-Kamera

Wird als Bildquelle eine Kamera im WLAN gewählt (z. B. ein Handy mit
Kamera-App), gilt zusätzlich:

* Verbunden wird **nur** mit der eingetragenen Adresse, **nur** während das
  Erkennungsfenster offen ist. Beim Schließen wird die Verbindung beendet
  (`NetworkCamera::stop`).
* Die Bilder holt das Rust-Backend (`src-tauri/src/camera.rs`) und reicht sie
  direkt an das Overlay weiter. Sie werden nicht auf die Festplatte geschrieben;
  gehalten wird immer nur das zuletzt empfangene Einzelbild im Arbeitsspeicher.
* Der Umweg über Rust ist technisch nötig: Ein fremder MJPEG-Stream wäre für den
  Webview eine Cross-Origin-Bildquelle, die WebGL – und damit MediaPipe – nicht
  verarbeiten darf. Nebeneffekt: Die CSP des Fensters bleibt unverändert streng,
  der Webview selbst spricht weiterhin mit niemandem.
* Der Stream bleibt damit im lokalen Netz. Wer die Adresse eines Dienstes
  außerhalb des eigenen Netzes einträgt, verlässt diese Zusage bewusst selbst –
  die App prüft nur das Schema (`http://` oder `https://`), nicht das Ziel.

## Gespeicherte Daten

Eine einzige SQLite-Datei im App-Data-Verzeichnis des Nutzers:

| Tabelle | Inhalt |
|---|---|
| `projects` | Projektname, Farbe, aktiv/inaktiv |
| `time_entries` | Beginn, Ende, Pausendauer, Dauer, Auslöser (Geste/manuell) |
| `settings` | Hotkey, Konfidenz-Schwelle, Erkennungsdauer, Ton, Slot-Belegung, Kameraquelle und -adresse, Sprache |
| `gesture_samples` | eingelernte Gesten: Kennung plus zehn Maßverhältnisse je Aufnahme – **keine Bilder** |
| `custom_gestures` | selbst definierte Gesten: Name, ausgelöste Aktion, ggf. Projekt |

Keine Nutzerkonten, keine IDs, keine biometrischen Merkmale. Auch die
eingelernten Gesten enthalten keine: gespeichert werden Verhältniszahlen einer
Handhaltung (etwa „Zeigefinger 0,97 gestreckt"), aus denen sich weder ein Bild
noch eine Person rekonstruieren lässt. Die Handlandmarks
werden pro Frame berechnet und sofort verworfen – gespeichert wird nur, welche
Aktion daraus folgte.

## Systemberechtigungen

* **macOS**: Kamerazugriff (`NSCameraUsageDescription` in `src-tauri/Info.plist`).
  Der Text erklärt dem Nutzer beim ersten Mal genau das Obige.
* **Global Shortcut**: nötig, damit der Hotkey auch bei fokussiertem
  Fremdfenster greift. Es wird ausschließlich die konfigurierte Kombination
  registriert, kein allgemeines Tastatur-Mitlesen.
