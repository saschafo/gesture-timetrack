/**
 * Übersetzungen der Oberfläche.
 *
 * Bewusst ohne Fremdbibliothek: Es geht um zwei Sprachen und einfache
 * Platzhalter. Ein eigenes Wörterbuch bleibt klein, lädt nichts nach und wird
 * durch Tests abgesichert (gleiche Kennungen, gleiche Platzhalter, nichts leer).
 *
 * Die Sprache liegt in den Einstellungen und gilt für Oberfläche **und**
 * Backend-Meldungen; die Umschaltung setzt beides.
 */

import { ref } from 'vue'

export type Locale = 'de' | 'en'

export const LOCALES: Array<{ code: Locale; label: string }> = [
  { code: 'de', label: 'Deutsch' },
  { code: 'en', label: 'English' },
]

const de: Record<string, string> = {
  'common.cancel': 'Abbrechen',
  'common.save': 'Speichern',
  'common.delete': 'Löschen',
  'common.edit': 'Bearbeiten',
  'common.close': 'Schließen',
  'common.to': 'bis',
  'common.today': 'Heute',
  'common.license': 'MIT-Lizenz',
  'key.space': 'Leertaste',

  'app.claim': 'Projektzeiten per Handgeste – vollständig offline.',
  'app.privacy':
    'Die Kamera läuft ausschließlich während des Erkennungsfensters. Bilder werden lokal im Arbeitsspeicher ausgewertet, nicht gespeichert und nicht übertragen.',
  'app.linkFailed': 'Link ließ sich nicht öffnen – {url}',
  'app.previewTitle': 'Kamera-Vorschau',
  'app.previewHint': 'Zum Ausrichten und zum Prüfen der Erkennung – bucht nichts.',
  'app.previewSummary': 'Kamera aus, solange eingeklappt',

  'status.noProject': 'Kein Projekt gewählt',
  'status.slotPending': 'Slot {slot} vorgemerkt',
  'status.break': 'Pause {time}',
  'status.today': 'heute {time}',
  'status.start': 'Start · {name}',
  'status.noSlot': 'kein Slot belegt',
  'status.pause': 'Pause',
  'status.resume': 'Weiter',
  'status.stop': 'Stopp',
  'status.hotkey': 'Geste erfassen:',
  'status.hotkeyInactive': 'Hotkey nicht aktiv – bitte in den Einstellungen neu belegen',

  'projects.title': 'Projekte',
  'projects.hint': 'Die beiden Slots bestimmen, welches Projekt eine Fingergeste auswählt.',
  'projects.slot': 'Slot {slot}',
  'projects.oneFinger': 'ein Finger',
  'projects.twoFingers': 'zwei Finger',
  'projects.unassigned': '– nicht belegt –',
  'projects.empty': 'Noch keine Projekte – das erste Projekt landet automatisch auf Slot 1.',
  'projects.newPlaceholder': 'Neues Projekt, z. B. Kunde Meier – Website',
  'projects.create': 'Anlegen',
  'projects.start': 'Start',
  'projects.switch': 'Wechseln',
  'projects.startTitle': 'Erfassung für dieses Projekt starten',
  'projects.switchTitle': 'Laufenden Eintrag abschließen und hier weitermachen',
  'projects.running': 'läuft',
  'projects.paused': 'pausiert',
  'projects.inactive': '(inaktiv)',
  'projects.activate': 'Aktivieren',
  'projects.deactivate': 'Deaktivieren',
  'projects.keptInactive': '„{name}“ hat erfasste Zeiten und wurde nur deaktiviert.',

  'gestures.hint': 'Hotkey drücken, Geste in die Kamera halten – fertig.',
  'gestures.switchHint':
    'Ein/zwei Finger starten das Projekt des Slots – aus jedem Zustand heraus. Läuft schon etwas anderes, wird der offene Eintrag abgeschlossen; eine Pause desselben Projekts wird fortgesetzt.',
  'gesture.open_hand': 'Offene Hand',
  'gesture.fist': 'Faust',
  'gesture.thumb_up': 'Daumen hoch',
  'gesture.one_finger': 'Ein Finger',
  'gesture.two_fingers': 'Zwei Finger',
  'gestureAction.open_hand': 'Start',
  'gestureAction.resume': 'Weiter',
  'gestureAction.fist': 'Stopp',
  'gestureAction.thumb_up': 'Pause',
  'gestureAction.one_finger': 'Slot 1 starten',
  'gestureAction.two_fingers': 'Slot 2 starten',

  'settings.title': 'Einstellungen',
  'settings.hint': 'Gilt sofort – die Werte liegen in der lokalen Datenbank.',
  'settings.language': 'Sprache',
  'settings.languageNote': 'Gilt für die Oberfläche, die Meldungen und den CSV-Export.',
  'settings.hotkey': 'Hotkey für die Gestenerkennung',
  'settings.hotkeyFired': '✓ ausgelöst',
  'settings.hotkeyPlaceholder': 'Tasten drücken …',
  'settings.testNow': 'Jetzt testen',
  'settings.presets': 'Frei wählbar oder direkt:',
  'settings.standard': 'Standard',
  'settings.hotkeyNote':
    'Feld anklicken und die Kombination drücken (Funktionstasten wie F13 gehen auch allein). Reagiert der Hotkey nicht, hat ihn das Betriebssystem für sich reserviert{macHint}; dann eine andere Kombination wählen. Kommt der Tastendruck an, erscheint hier oben kurz „ausgelöst“.',
  'settings.hotkeyMacHint': ' – ⌘⌥Leertaste ist z. B. die Finder-Suche',
  'settings.camera': 'Kamera',
  'settings.cameraBuiltin': 'Eingebaute Webcam',
  'settings.cameraNetwork': 'Netzwerk-Kamera im WLAN (z. B. Handy)',
  'settings.cameraTest': 'Verbindung testen',
  'settings.cameraTesting': 'prüfe …',
  'settings.cameraPrivacy':
    'Der Videostream bleibt im lokalen WLAN: Die App holt die Bilder direkt von der eingetragenen Adresse und verarbeitet sie auf diesem Gerät. Keine Cloud, kein Upload, keine Speicherung.',
  'settings.cameraNote':
    'Beliebige Kamera-App auf dem Handy (etwa DroidCam oder IP Webcam), Adresse des MJPEG-Streams eintragen. Es wird keine bestimmte App vorausgesetzt.',
  'settings.cameraBuiltinNote': 'Standard: die eingebaute Kamera. Dafür ist kein Netzwerk nötig.',
  'settings.threshold': 'Konfidenz-Schwelle',
  'settings.thresholdNote':
    'Höher = weniger Fehlauslösungen, aber die Geste muss deutlicher gezeigt werden. Unterhalb der Schwelle wird nichts gebucht.',
  'settings.window': 'Erkennungsfenster',
  'settings.windowNote':
    'So lange wird nach dem Hotkey nach einer Geste gesucht{networkHint}. Die Zeit läuft erst ab dem ersten Bild.',
  'settings.windowNetworkHint':
    ' – bei der Netzwerk-Kamera eigener Wert, weil der Stream später ankommt',
  'settings.sound': 'Tonsignal bei Erkennung',
  'update.title': 'Aktualisierung',
  'update.check': 'Nach Updates suchen',
  'update.checking': 'suche …',
  'update.current': 'Version {version} – aktuell.',
  'update.available': 'Version {version} verfügbar.',
  'update.install': 'Installieren und neu starten',
  'update.installing': 'lädt … {percent} %',
  'update.failed': 'Update fehlgeschlagen: {error}',
  'update.note':
    'Die Suche ist der einzige Moment, in dem die App von sich aus ins Netz geht – nur auf diesen Knopf hin, nie im Hintergrund. Abgerufen wird ausschließlich die Versionsdatei der Veröffentlichungsseite; es werden keine Daten über Sie übertragen.',
  'settings.summaryNetwork': 'Netzwerk-Kamera',
  'settings.summaryBuiltin': 'eingebaute Webcam',

  'preview.start': 'Vorschau starten',
  'preview.starting': 'startet …',
  'preview.stop': 'Vorschau beenden',
  'preview.off': 'Vorschau aus – die Kamera ist nicht aktiv.',
  'preview.waiting': 'Warte auf Bild …',
  'preview.noHand': 'keine Hand im Bild',
  'preview.modeRules': 'feste Regeln',
  'preview.modeTraining': 'eigenes Training',
  'preview.modeHybrid': 'Regeln + eigene Gesten',
  'preview.note':
    'Die Vorschau bucht nichts – sie zeigt nur, was die Erkennung sieht. Der Strich in der Leiste ist die Konfidenz-Schwelle; die Zahlen darunter sind die Streckung je Finger (Z/M/R/K) in Prozent.',
  'preview.fingers': 'Finger',
  'preview.thumb': 'Daumen',
  'preview.notReady': 'Vorschau nicht bereit',
  'preview.noAccess': 'Kein Kamerazugriff erlaubt – bitte in den Systemeinstellungen freigeben.',
  'preview.noImage': 'Es kommt kein Kamerabild an.',

  'training.intro':
    'Jede Geste einmal aufnehmen: Hand ins Bild halten, „Aufnehmen“ drücken, nach dem Countdown die Geste ruhig halten. Aufgenommen werden keine Bilder, sondern nur Maßverhältnisse der Handhaltung.',
  'training.record': 'Aufnehmen',
  'training.recordAgain': 'Neu aufnehmen',
  'training.deleteSamples': 'Aufnahmen löschen',
  'training.needPreview': 'Zum Aufnehmen zuerst die Vorschau starten.',
  'training.use': 'Eigenes Training verwenden',
  'training.needComplete': '– erst wenn alle sechs Gesten aufgenommen sind',
  'training.reset': 'Training zurücksetzen',
  'training.countdown': '{name} zeigen … {step}',
  'training.hold': '{name} ruhig halten …',
  'training.saved': '{name}: {count} Aufnahmen gespeichert.',
  'training.tooFew': 'Zu wenige Bilder mit Hand – bitte die Hand vollständig im Bild halten.',
  'training.ownTitle': 'Eigene Gesten',
  'training.ownHint':
    'Frei erfundene Handhaltungen mit eigener Bedeutung. Sie funktionieren ausschließlich über Aufnahmen und greifen erst, wenn keine der sechs Grundgesten sicher erkannt wurde – „Stopp“ lässt sich also nicht versehentlich überschreiben. Am besten deutlich anders halten als die Grundgesten.',
  'training.ownName': 'Name, z. B. „Handkante“',
  'training.ownAdd': 'Hinzufügen',
  'training.ownDelete': 'Geste löschen',
  'training.chooseProject': '– Projekt wählen –',
  'training.ownFallbackName': 'Eigene Geste',

  'overview.title': 'Auswertung',
  'overview.hint': 'Zeiten je Projekt, exportierbar für die Buchhaltung.',
  'overview.allProjects': 'Alle Projekte',
  'overview.filterTitle': 'Nach Projekt filtern',
  'overview.addEntry': 'Eintrag nachtragen',
  'overview.export': 'CSV exportieren',
  'overview.exportTitle': 'Zeiten als CSV exportieren',
  'overview.todayPerProject': 'Heute je Projekt',
  'overview.nothingToday': 'Heute noch nichts erfasst.',
  'overview.colDate': 'Datum',
  'overview.colProject': 'Projekt',
  'overview.colFrom': 'Von',
  'overview.colTo': 'Bis',
  'overview.colPause': 'Pause',
  'overview.minutes': '{count} min',
  'overview.colDuration': 'Dauer',
  'overview.colHours': 'Stunden',
  'overview.colSource': 'Auslöser',
  'overview.sourceGesture': 'Geste',
  'overview.sourceManual': 'manuell',
  'overview.running': 'läuft',
  'overview.stopFirst': 'erst stoppen',
  'overview.empty': 'Keine Einträge im gewählten Zeitraum.',
  'overview.emptyFiltered': 'Keine Einträge im gewählten Zeitraum für „{name}“.',
  'overview.sum': 'Summe (gesamter Zeitraum)',
  'overview.pageInfo': '{shown} von {total} Einträgen',
  'overview.showMore': 'Weitere {count} anzeigen',
  'overview.showAll': 'Alle anzeigen',
  'overview.showLess': 'Weniger anzeigen',
  'overview.exported': '{count} Einträge exportiert.',
  'overview.exportedOne': 'Ein Eintrag exportiert.',
  'overview.entryAdded': 'Eintrag nachgetragen.',
  'overview.entryChanged': 'Eintrag geändert.',
  'overview.entryDeleted': 'Eintrag gelöscht.',
  'overview.editTitle': 'Eintrag bearbeiten',
  'overview.addTitle': 'Eintrag nachtragen',
  'overview.modalHint': 'Die Nettozeit rechnet die App selbst: Gesamtdauer abzüglich Pause.',

  'entry.project': 'Projekt',
  'entry.from': 'von',
  'entry.to': 'bis',
  'entry.pause': 'Pause (min)',

  'panel.startProject': 'Projekt starten',
  'panel.switchProject': 'Projekt wechseln',
  'panel.noProjects': 'Noch keine Projekte – im Fenster anlegen.',
  'panel.slot': 'Slot {slot}',
  'panel.openWindow': 'Hauptfenster öffnen',
  'panel.captureGesture': 'Gestenerkennung starten',
  'panel.hotkeyInactive': 'Hotkey nicht aktiv',
  'panel.gesture': 'Geste:',

  'overlay.cameraStarting': 'Kamera startet …',
  'overlay.networkConnecting': 'Netzwerk-Kamera verbinden …',
  'overlay.showGesture': 'Geste zeigen',
  'overlay.noGesture': 'Keine Geste erkannt',
  'overlay.cameraOff': 'Kamera aus',
  'overlay.noAccess': 'Kein Kamerazugriff erlaubt',
  'overlay.cameraUnavailable': 'Kamera nicht verfügbar',
  'overlay.noImage': 'Kein Kamerabild',
  'overlay.notReady': 'Overlay nicht bereit',
}

const en: Record<string, string> = {
  'common.cancel': 'Cancel',
  'common.save': 'Save',
  'common.delete': 'Delete',
  'common.edit': 'Edit',
  'common.close': 'Close',
  'common.to': 'to',
  'common.today': 'Today',
  'common.license': 'MIT licence',
  'key.space': 'Space',

  'app.claim': 'Project time tracking by hand gesture – fully offline.',
  'app.privacy':
    'The camera only runs during the recognition window. Frames are processed locally in memory, never stored and never transmitted.',
  'app.linkFailed': 'The link could not be opened – {url}',
  'app.previewTitle': 'Camera preview',
  'app.previewHint': 'For aiming the camera and checking recognition – records no time.',
  'app.previewSummary': 'camera off while collapsed',

  'status.noProject': 'No project selected',
  'status.slotPending': 'slot {slot} queued',
  'status.break': 'break {time}',
  'status.today': 'today {time}',
  'status.start': 'Start · {name}',
  'status.noSlot': 'no slot assigned',
  'status.pause': 'Pause',
  'status.resume': 'Resume',
  'status.stop': 'Stop',
  'status.hotkey': 'Capture gesture:',
  'status.hotkeyInactive': 'Hotkey inactive – please choose another one in the settings',

  'projects.title': 'Projects',
  'projects.hint': 'The two slots decide which project a finger gesture selects.',
  'projects.slot': 'Slot {slot}',
  'projects.oneFinger': 'one finger',
  'projects.twoFingers': 'two fingers',
  'projects.unassigned': '– not assigned –',
  'projects.empty': 'No projects yet – the first one is assigned to slot 1 automatically.',
  'projects.newPlaceholder': 'New project, e.g. Acme – Website',
  'projects.create': 'Create',
  'projects.start': 'Start',
  'projects.switch': 'Switch',
  'projects.startTitle': 'Start tracking this project',
  'projects.switchTitle': 'Close the open entry and continue here',
  'projects.running': 'running',
  'projects.paused': 'on break',
  'projects.inactive': '(inactive)',
  'projects.activate': 'Activate',
  'projects.deactivate': 'Deactivate',
  'projects.keptInactive': '“{name}” has tracked time and was only deactivated.',

  'gestures.hint': 'Press the hotkey, hold the gesture up to the camera – done.',
  'gestures.switchHint':
    'One or two fingers start the slot’s project – from any state. If something else is running, its entry is closed; a break on the same project is resumed.',
  'gesture.open_hand': 'Open hand',
  'gesture.fist': 'Fist',
  'gesture.thumb_up': 'Thumbs up',
  'gesture.one_finger': 'One finger',
  'gesture.two_fingers': 'Two fingers',
  'gestureAction.open_hand': 'Start',
  'gestureAction.resume': 'Resume',
  'gestureAction.fist': 'Stop',
  'gestureAction.thumb_up': 'Pause',
  'gestureAction.one_finger': 'Start slot 1',
  'gestureAction.two_fingers': 'Start slot 2',

  'settings.title': 'Settings',
  'settings.hint': 'Applies immediately – values are stored in the local database.',
  'settings.language': 'Language',
  'settings.languageNote': 'Applies to the interface, all messages and the CSV export.',
  'settings.hotkey': 'Hotkey for gesture recognition',
  'settings.hotkeyFired': '✓ triggered',
  'settings.hotkeyPlaceholder': 'Press keys …',
  'settings.testNow': 'Test now',
  'settings.presets': 'Choose freely or pick one:',
  'settings.standard': 'Default',
  'settings.hotkeyNote':
    'Click the field and press the combination (function keys such as F13 work on their own). If the hotkey does not respond, the operating system has reserved it{macHint}; pick another one. When a keypress arrives, “triggered” appears above for a moment.',
  'settings.hotkeyMacHint': ' – ⌘⌥Space is the Finder search, for example',
  'settings.camera': 'Camera',
  'settings.cameraBuiltin': 'Built-in webcam',
  'settings.cameraNetwork': 'Network camera on your Wi-Fi (e.g. a phone)',
  'settings.cameraTest': 'Test connection',
  'settings.cameraTesting': 'testing …',
  'settings.cameraPrivacy':
    'The video stream stays on your local network: the app fetches the frames straight from the address you entered and processes them on this device. No cloud, no upload, no storage.',
  'settings.cameraNote':
    'Any camera app on your phone (DroidCam, IP Webcam, …) – just enter the address of its MJPEG stream. No particular app is required.',
  'settings.cameraBuiltinNote': 'Default: the built-in camera. No network needed for it.',
  'settings.threshold': 'Confidence threshold',
  'settings.thresholdNote':
    'Higher = fewer false triggers, but the gesture has to be clearer. Below the threshold nothing is recorded.',
  'settings.window': 'Recognition window',
  'settings.windowNote':
    'How long a gesture is looked for after the hotkey{networkHint}. The clock starts with the first frame.',
  'settings.windowNetworkHint':
    ' – the network camera has its own value because its stream arrives later',
  'settings.sound': 'Sound on recognition',
  'update.title': 'Updates',
  'update.check': 'Check for updates',
  'update.checking': 'checking …',
  'update.current': 'Version {version} – up to date.',
  'update.available': 'Version {version} available.',
  'update.install': 'Install and restart',
  'update.installing': 'downloading … {percent}%',
  'update.failed': 'Update failed: {error}',
  'update.note':
    'This check is the only moment the app reaches the network on its own – only on this button, never in the background. It fetches the release manifest and nothing else; no data about you is sent.',
  'settings.summaryNetwork': 'network camera',
  'settings.summaryBuiltin': 'built-in webcam',

  'preview.start': 'Start preview',
  'preview.starting': 'starting …',
  'preview.stop': 'Stop preview',
  'preview.off': 'Preview off – the camera is not active.',
  'preview.waiting': 'Waiting for a frame …',
  'preview.noHand': 'no hand in frame',
  'preview.modeRules': 'fixed rules',
  'preview.modeTraining': 'your training',
  'preview.modeHybrid': 'rules + custom gestures',
  'preview.note':
    'The preview records no time – it only shows what recognition sees. The mark in the bar is the confidence threshold; the numbers below are how straight each finger is (I/M/R/L) in percent.',
  'preview.fingers': 'Fingers',
  'preview.thumb': 'Thumb',
  'preview.notReady': 'Preview not ready',
  'preview.noAccess': 'Camera access denied – please allow it in your system settings.',
  'preview.noImage': 'No camera frames are arriving.',

  'training.intro':
    'Record each gesture once: hold your hand in frame, press “Record”, then hold the gesture still after the countdown. No images are stored – only proportions of your hand pose.',
  'training.record': 'Record',
  'training.recordAgain': 'Record again',
  'training.deleteSamples': 'Delete recordings',
  'training.needPreview': 'Start the preview first to record.',
  'training.use': 'Use my training',
  'training.needComplete': '– only once all six gestures are recorded',
  'training.reset': 'Reset training',
  'training.countdown': 'Show {name} … {step}',
  'training.hold': 'Hold {name} still …',
  'training.saved': '{name}: {count} recordings saved.',
  'training.tooFew': 'Too few frames with a hand – please keep your hand fully in frame.',
  'training.ownTitle': 'Custom gestures',
  'training.ownHint':
    'Hand poses you invent yourself, with a meaning you choose. They work only from recordings and apply only when none of the six base gestures was recognised confidently – so “Stop” can never be overridden by accident. Best to hold them clearly differently from the base gestures.',
  'training.ownName': 'Name, e.g. “edge of hand”',
  'training.ownAdd': 'Add',
  'training.ownDelete': 'Delete gesture',
  'training.chooseProject': '– choose a project –',
  'training.ownFallbackName': 'Custom gesture',

  'overview.title': 'Reports',
  'overview.hint': 'Time per project, exportable for accounting.',
  'overview.allProjects': 'All projects',
  'overview.filterTitle': 'Filter by project',
  'overview.addEntry': 'Add entry',
  'overview.export': 'Export CSV',
  'overview.exportTitle': 'Export time as CSV',
  'overview.todayPerProject': 'Today per project',
  'overview.nothingToday': 'Nothing tracked today yet.',
  'overview.colDate': 'Date',
  'overview.colProject': 'Project',
  'overview.colFrom': 'From',
  'overview.colTo': 'To',
  'overview.colPause': 'Break',
  'overview.minutes': '{count} min',
  'overview.colDuration': 'Duration',
  'overview.colHours': 'Hours',
  'overview.colSource': 'Trigger',
  'overview.sourceGesture': 'gesture',
  'overview.sourceManual': 'manual',
  'overview.running': 'running',
  'overview.stopFirst': 'stop first',
  'overview.empty': 'No entries in the selected period.',
  'overview.emptyFiltered': 'No entries in the selected period for “{name}”.',
  'overview.sum': 'Total (entire period)',
  'overview.pageInfo': '{shown} of {total} entries',
  'overview.showMore': 'Show {count} more',
  'overview.showAll': 'Show all',
  'overview.showLess': 'Show fewer',
  'overview.exported': '{count} entries exported.',
  'overview.exportedOne': 'One entry exported.',
  'overview.entryAdded': 'Entry added.',
  'overview.entryChanged': 'Entry updated.',
  'overview.entryDeleted': 'Entry deleted.',
  'overview.editTitle': 'Edit entry',
  'overview.addTitle': 'Add entry',
  'overview.modalHint': 'The app computes net time itself: total duration minus break.',

  'entry.project': 'Project',
  'entry.from': 'from',
  'entry.to': 'to',
  'entry.pause': 'Break (min)',

  'panel.startProject': 'Start project',
  'panel.switchProject': 'Switch project',
  'panel.noProjects': 'No projects yet – create one in the main window.',
  'panel.slot': 'Slot {slot}',
  'panel.openWindow': 'Open main window',
  'panel.captureGesture': 'Start gesture recognition',
  'panel.hotkeyInactive': 'Hotkey inactive',
  'panel.gesture': 'Gesture:',

  'overlay.cameraStarting': 'Camera starting …',
  'overlay.networkConnecting': 'Connecting to network camera …',
  'overlay.showGesture': 'Show a gesture',
  'overlay.noGesture': 'No gesture recognised',
  'overlay.cameraOff': 'Camera off',
  'overlay.noAccess': 'Camera access denied',
  'overlay.cameraUnavailable': 'Camera unavailable',
  'overlay.noImage': 'No camera frames',
  'overlay.notReady': 'Overlay not ready',
}

const DICTS: Record<Locale, Record<string, string>> = { de, en }

/** Aktuelle Sprache. Reaktiv, damit ein Wechsel die Oberfläche sofort umstellt. */
export const locale = ref<Locale>('de')

export function setLocale(value: string | undefined | null): void {
  locale.value = value === 'en' ? 'en' : 'de'
}

/**
 * Übersetzung mit Platzhaltern: `{name}` wird ersetzt. Fehlt eine Kennung,
 * kommt sie selbst zurück - dann fällt der Fehler sofort auf.
 */
export function t(key: string, params?: Record<string, string | number>): string {
  let text = DICTS[locale.value][key] ?? DICTS.de[key] ?? key
  if (params) {
    for (const [name, value] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${name}\\}`, 'g'), String(value))
    }
  }
  return text
}

/** Nur für Tests: Wörterbücher zum Vergleichen. */
export const dictionaries = DICTS
