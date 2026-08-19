# Gesture TimeTrack

*[English version](README.md)*

Zeiterfassung per Handgeste – der Software-Ersatz für einen Hardware-Buzzer.
Hotkey drücken, Geste in die Kamera halten, weiterarbeiten.

**100 % offline.** Die Kamerabilder werden ausschließlich auf dem Gerät
ausgewertet. Es gibt keinen Cloud-Dienst, keinen Upload, kein Konto und keine
gespeicherten Bilder. Der einzige Netzwerkweg ist optional und bleibt im eigenen
WLAN: die selbst eingetragene Adresse einer Netzwerk-Kamera (siehe unten).

![Hauptfenster von Gesture TimeTrack](docs/screenshots/hauptfenster.png)

---

## Gesten-Vokabular

| Geste | Bedeutung |
|---|---|
| 🖐️ Offene Hand | **Start** – und **Weiter** aus einer Pause |
| ✊ Faust | **Stopp** – Eintrag wird gebucht |
| 👍 Daumen hoch | **Pause** |
| ☝️ Ein Finger | Projekt-**Slot 1** starten |
| ✌️ Zwei Finger | Projekt-**Slot 2** starten |

Fünf Gesten, kein eigenes „Weiter": Die offene Hand setzt eine Pause fort. Eine
Geste weniger zu merken, eine Verwechslungsmöglichkeit weniger.

Eine Slot-Geste **startet** die Erfassung – aus jedem Zustand heraus. Läuft
gerade ein anderes Projekt (oder ist es pausiert), schließt die App dessen
Eintrag mit seiner Nettozeit ab und beginnt sofort den neuen; eine Pause
desselben Projekts wird fortgesetzt. Per Maus geht
das genauso: in der Projektliste heißt der Knopf dann *Wechseln*. Die offene
Hand setzt eine pausierte Erfassung fort, statt einen Fehler zu melden.

## Ablauf einer Buchung

1. **Hotkey** (Standard `Strg/Cmd + Alt + Leertaste`) – das Kamera-Overlay
   erscheint oben rechts, ohne den Fokus des Arbeitsfensters zu stehlen. Bei
   mehreren Bildschirmen auf dem, auf dem der Mauszeiger steht.
2. Die Kamera läuft für die eingestellte Zeitspanne (Standard 3 s).
3. Eine Geste zählt erst, wenn sie **drei Frames in Folge** über der
   Konfidenz-Schwelle (Standard 85 %) liegt.
4. Rückmeldung im Overlay: grüner Rahmen = übernommen, roter Rahmen = nicht
   erkannt. Optional ein kurzer Ton.
5. Overlay schließt sich, Kamera aus. Bei Unsicherheit wird **nichts** gebucht –
   es gibt bewusst keinen Fallback auf „irgendeine“ Aktion.

`Esc` bricht das Erkennungsfenster jederzeit ab.

## Optional: Handy als Kamera

Wer keine brauchbare eingebaute Webcam hat, kann eine **Netzwerk-Kamera im
eigenen WLAN** verwenden – etwa ein Handy mit einer beliebigen Kamera-App
(DroidCam, IP Webcam o. ä.; keine bestimmte App wird vorausgesetzt oder
mitgeliefert).

*Einstellungen → Kamera → Netzwerk-Kamera im WLAN*, dann die Stream-Adresse
eintragen, z. B. `http://192.168.1.20:4747/video`, und **Verbindung testen**
drücken – der Test sagt im Klartext, ob ein Stream ankommt und in welcher
Auflösung. Unterstützt werden fortlaufende MJPEG-Streams und Adressen, die pro
Aufruf ein Einzelbild liefern (`…/shot.jpg`).

* Der Stream bleibt im lokalen Netz: Die App holt die Bilder direkt von der
  eingetragenen Adresse und wertet sie auf dem Gerät aus. Keine Cloud.
* Verbunden wird **nur** während des Erkennungsfensters, nie im Hintergrund.
* Das Erkennungsfenster ist hier standardmäßig **4 Sekunden** statt 3, weil
  MJPEG über WLAN spürbar später ankommt. Die Zeit läuft erst ab dem ersten
  Bild; beide Werte stehen als Einstellung in der Datenbank.
* Der Standardweg bleibt unangetastet: ohne diese Option kommt die App ohne
  Netzwerk aus.

Zu beachten: Viele Kamera-Apps bedienen nur **einen** Stream-Client. Läuft
parallel ein Browser-Tab, OBS oder der Desktop-Client derselben App, antwortet
die Kamera der App mit ihrer Bedienseite statt mit Bildern – dann meldet
Gesture TimeTrack genau das, statt eine schwarze Vorschau zu zeigen.

## Installation & Entwicklung

Voraussetzungen: Node ≥ 20, Rust ≥ 1.88, die
[Tauri-Systemabhängigkeiten](https://tauri.app/start/prerequisites/).

```bash
npm install          # lädt zusätzlich Modell + WASM-Laufzeit nach public/
npm run tauri:dev    # Entwicklung
npm run tauri:build  # Installer bauen
```

`npm install` holt einmalig das MediaPipe-Handmodell (~7,5 MB) und die
WASM-Laufzeit in `public/`. Das ist der **einzige** Netzwerkzugriff des gesamten
Projekts, und er passiert zur Bauzeit, nicht zur Laufzeit. Beides liegt danach
im Bundle; die fertige App lädt nichts nach. Erneut auslösen:
`npm run assets`.

## Kamera-Vorschau im Hauptfenster

*Kamera-Vorschau* zeigt live, was die Erkennung sieht: das Bild, die erkannten
Handpunkte, die aktuell erkannte Geste und ihren Konfidenzwert samt Schwelle.
Gedacht zum Ausrichten der Kamera und zum Nachjustieren der Schwelle – **es wird
dabei nichts gebucht**.

Die Vorschau läuft nur auf Knopfdruck und schaltet die Kamera ab, sobald sie
beendet oder die Karte eingeklappt wird.

## Gesten einlernen

Wird eine Geste bei deiner Hand nicht zuverlässig erkannt, kannst du sie
**einlernen**: *Kamera-Vorschau → Vorschau starten →* pro Geste *Aufnehmen*.
Nach einem Countdown hältst du die Geste rund 1,5 Sekunden ruhig; die App
speichert daraus etwa 20–40 Messungen. Sind alle sechs Gesten aufgenommen, lässt
sich *Eigenes Training verwenden* einschalten.

Wie das funktioniert: Gespeichert werden **keine Bilder**, sondern zehn
Maßverhältnisse der Handhaltung (Streckung je Finger, Daumenlage, Abstände der
Fingerspitzen) – alles dimensionslos, also unabhängig von Handgröße und
Kameraabstand. Erkannt wird danach über den nächsten Nachbarn im Merkmalsraum:
die Haltung wird der Geste zugeordnet, deren Aufnahme ihr am nächsten liegt. Kein
neuronales Netz, kein Trainingslauf, keine zusätzliche Modelldatei – und jede
Entscheidung bleibt auf eine konkrete Aufnahme zurückführbar.

Ohne eingeschaltetes Training gelten die festen geometrischen Regeln. Dorthin
fällt die App auch selbsttätig zurück, wenn Aufnahmen gelöscht wurden oder zu
einem älteren Merkmalssatz gehören – die Erkennung wird also nie stillschweigend
schlechter. Welche Art gerade greift, zeigt die Vorschau an („feste Regeln" bzw.
„eigenes Training").

## Eigene Gesten

Zusätzlich zu den sechs Grundgesten lassen sich **eigene** anlegen: Name
vergeben, Aktion wählen (Start, Stopp, Pause, Weiter, Slot 1/2 oder *ein
bestimmtes Projekt starten*), aufnehmen – fertig. Eine eigene Geste je Projekt
ist damit möglich, ohne den Umweg über die Slots.

Eigene Gesten greifen **nachrangig**: erkennt der Regelweg eine Grundgeste
sicher, gewinnt diese. „Stopp“ lässt sich also nicht versehentlich
überschreiben. Sie funktionieren auch ohne vollständig eingelerntes Training –
dann im Mischbetrieb, den die Vorschau als „Regeln + eigene Gesten“ anzeigt.
Halte sie deutlich anders als die Grundgesten, sonst bleibt sie unerreichbar.

## Bedienung ohne Geste

**Klick auf das Symbol in der Menüleiste** öffnet ein kleines Fenster direkt
darunter: laufende Zeit, Start/Pause/Weiter/Stopp und die Projektliste zum
Wechseln mit einem Klick. Es schließt sich wieder, sobald es den Fokus verliert –
wie ein Menü. Rechtsklick öffnet weiterhin das klassische Menü, ein Klick auf das
Pfeilsymbol im kleinen Fenster das Hauptfenster.

Alles geht auch per Maus: Hauptfenster für Projekte, Slots, Auswertung und
Einstellungen; Tray-Menü für Start/Stopp/Pause, Projektwechsel und den
aktuellen Stand.

Das Schließen des Fensters beendet die App **nicht** – sie lebt im Tray weiter,
sonst wäre der Hotkey weg. Zurück ins Fenster führen drei Wege: Klick auf das
Symbol in der Menüleiste (dort der Pfeil-Knopf), der Menüeintrag *Fenster
öffnen* und ein Klick auf das Dock-Symbol. Wirklich beenden über *Beenden* im
Tray-Menü. In der Menüleiste laufen Projektname und Zeit sekundengenau
mit (`Kunde Meier · 01:01:01`, pausiert mit `‖`); über *Projekt starten* bzw.
*Projekt wechseln* im Tray-Menü lässt sich jedes aktive Projekt direkt
auswählen.
Der Hotkey ist frei belegbar (Feld anklicken, Kombination drücken; Funktionstasten
wie `F13` gehen auch allein). Reagiert er nicht, hat ihn das System reserviert –
auf macOS ist `⌘⌥Leertaste` z. B. die Finder-Suche. Kommt ein Tastendruck an,
bestätigt das Feld das kurz mit „ausgelöst“. Die App
läuft nach dem Schließen des Fensters im Tray weiter, damit der Hotkey
erreichbar bleibt.

## Daten & Export

Alle Daten liegen in einer SQLite-Datei im App-Data-Verzeichnis des Nutzers
(macOS: `~/Library/Application Support/de.swd.gesture-timetrack/`).

Einträge lassen sich in der Auswertung **bearbeiten und löschen** (Dialogfenster),
und über *Eintrag nachtragen* auch von Hand anlegen – für vergessene Zeiten oder eine
Fehlerkennung. Geprüft wird dabei im Backend: Ende nach Beginn, Pause kürzer als
der Eintrag; die Nettozeit rechnet die App selbst. Nur der gerade laufende
Eintrag ist gesperrt – dafür erst stoppen.

Die Tabelle zeigt neben Beginn und Ende auch die **Pause** in Minuten – die
Dauer daneben ist bereits die Nettozeit, sonst wäre der Zusammenhang nicht
nachvollziehbar. Die Summenzeile addiert beides.

Vor dem Datum steht eine kleine Farbmarke: normalerweise die Projektfarbe, beim
laufenden Eintrag **grün**, bei pausierter Erfassung **orange**. Der aktive
Eintrag pulsiert dabei langsam – laufend in ruhigem Takt, pausiert mit längerer
Ruhephase. Wer im System „Bewegung reduzieren" eingestellt hat, sieht eine
ruhige Marke. Der offene
Eintrag zeigt in der Spalte *Bis* seinen Zustand („läuft" bzw. „pausiert") statt
einer Uhrzeit.

Die Tabelle zeigt zunächst 20 Zeilen; *Weitere anzeigen* bzw. *Alle anzeigen*
holen den Rest, *Weniger anzeigen* führt zurück zur kurzen Liste. Die Summe unten rechnet immer den **gesamten** Zeitraum, nicht
nur die sichtbaren Zeilen.

Über die Auswahl **Alle Projekte** lässt sich die Liste auf ein Projekt
einschränken – der Filter gilt auch für den Export, damit sich pro Kunde
abrechnen lässt (der Projektname landet dann im Dateinamen).

CSV-Export (Semikolon, deutsche Dezimalkommas – Excel-tauglich) über
*Auswertung → CSV exportieren*, mit Datum, Projekt, Beginn, Ende, Dauer als
`hh:mm:ss` und als Dezimalstunden, Pausendauer und Auslöser (Geste/manuell).

## Technik

| Bereich | Wahl |
|---|---|
| App-Framework | Tauri 2 (Rust) |
| Frontend | Vue 3 + Pinia + Vite |
| Gestenerkennung | MediaPipe Hand Landmarker (Tasks Vision, WASM) |
| Bildquelle | eingebaute Webcam (`getUserMedia`) oder MJPEG-Kamera im WLAN |
| Klassifikation | eigene geometrische Auswertung der 21 Landmarks |
| Datenhaltung | SQLite via `rusqlite` |
| Hotkey / Tray | Tauri Global Shortcut Plugin, Tray Icon API |

Zustand (läuft/pausiert/gestoppt) liegt vollständig im Rust-Backend – Geste,
Tray-Menü und Fenster nutzen dieselben Pfade. Details:
[docs/architektur.md](docs/architektur.md),
[docs/gesten.md](docs/gesten.md),
[docs/datenschutz.md](docs/datenschutz.md).

## Sprache

Deutsch und Englisch, umschaltbar über den **DE|EN-Schalter oben rechts im
Fensterkopf**. Die Wahl gilt
für die Oberfläche, für alle Meldungen aus dem Backend (Tray-Menü,
Fehlermeldungen, Rückmeldung im Overlay) **und** für den CSV-Export: deutsch mit
Semikolon und Dezimalkomma, englisch mit Komma und Dezimalpunkt – so öffnet
Excel die Datei in beiden Fällen ohne Import-Assistent.

Umgesetzt ohne Übersetzungsbibliothek: je ein Wörterbuch im Frontend
([src/i18n.ts](src/i18n.ts)) und im Backend
([src-tauri/src/i18n.rs](src-tauri/src/i18n.rs)), beide durch Tests
abgesichert – gleiche Kennungen in beiden Sprachen, nichts leer, gleiche
Platzhalter. Fehlt eine Übersetzung, erscheint die Kennung selbst; das fällt
sofort auf, statt still auf die falsche Sprache zurückzufallen.

## Auslieferung

Native Pakete für **Windows 11** und **macOS** entstehen aus derselben
Codebasis (`npm run tauri:build`, jeweils auf dem Zielsystem). Ein annotierter Versions-Tag
(`git tag -a v1.0.0 -m "Version 1.0.0"`) löst den Workflow aus, der beide
Systeme baut und einen Release-Entwurf anlegt.
Versionsnummern, Signatur und die je System zu prüfenden Stellen stehen in
[docs/release.md](docs/release.md). Die laufende Version zeigt die App im
Fensterkopf.

**Updates** kommen über GitHub Releases und werden **nur auf Knopfdruck**
gesucht (*Einstellungen → Aktualisierung*) – nie im Hintergrund. Diese Prüfung
ist der einzige Netzwerkzugriff der fertigen Anwendung, und der Nutzer löst ihn
selbst aus.

## Anbieterangaben anpassen

Name, Website und Lizenzhinweis im Fensterkopf stehen an einer Stelle:
[src/branding.ts](src/branding.ts). Das Jahr wächst selbst mit. Der Link öffnet
den Standardbrowser (Tauri-Opener-Plugin) – das Anwendungsfenster wird nie zum
Browser.

Symbole sind selbst gezeichnete Inline-SVGs
([src/components/Icon.vue](src/components/Icon.vue)): keine Icon-Bibliothek,
keine Icon-Schrift, nichts nachzuladen. Sie erben Farbe und Strichstärke vom
Text und passen damit automatisch zum hellen und dunklen Erscheinungsbild. Für
die Gesten selbst bleiben die Emoji-Handzeichen – sie zeigen die Handhaltung
deutlicher als jedes gezeichnete Symbol.

## Lizenz

MIT – siehe [LICENSE](LICENSE).
