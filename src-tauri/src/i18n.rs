//! Übersetzungen für alles, was das Backend an den Nutzer schreibt.
//!
//! Bewusst eine einzige Tabelle statt vieler Funktionen: So steht jede Meldung
//! in beiden Sprachen direkt nebeneinander, und eine fehlende Übersetzung fällt
//! beim Lesen auf. Ein Test prüft zusätzlich, dass keine Kennung doppelt vorkommt
//! und keine Seite leer bleibt.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::db::Db;
use crate::state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    De,
    En,
}

impl Lang {
    pub fn code(&self) -> &'static str {
        match self {
            Lang::De => "de",
            Lang::En => "en",
        }
    }

    pub fn parse(raw: &str) -> Lang {
        match raw.trim().to_ascii_lowercase().as_str() {
            "en" => Lang::En,
            _ => Lang::De,
        }
    }
}

/// Aktuell gewählte Sprache des laufenden Programms.
///
/// Bewusst ein Prozesszustand: Meldungen entstehen an Dutzenden Stellen, oft
/// ohne Zugriff auf die Datenbank. Die Alternative wäre, die Sprache durch jede
/// Funktion zu fädeln - viel Rauschen für eine Angabe, die für das gesamte
/// Programm gilt.
static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_current(lang: Lang) {
    CURRENT.store(match lang {
        Lang::De => 0,
        Lang::En => 1,
    }, Ordering::Relaxed);
}

pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::En,
        _ => Lang::De,
    }
}

/// Übernimmt die Sprache aus den Einstellungen.
pub fn load_current(db: &Db) {
    set_current(lang(db));
}

/// Meldung in der aktuellen Sprache.
pub fn msg(key: &str) -> String {
    t(current(), key)
}

/// Meldung mit Platzhaltern in der aktuellen Sprache.
pub fn msg_args(key: &str, args: &[(String, String)]) -> String {
    let borrowed: Vec<(&str, &str)> = args
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    ta(current(), key, &borrowed)
}

/// Kurzform für die häufigste Einbettung: ein fremder Fehlertext.
pub fn msg_error(key: &str, error: impl std::fmt::Display) -> String {
    ta(current(), key, &[("error", &error.to_string())])
}

/// Eingestellte Sprache. Fällt auf Deutsch zurück, wenn nichts gesetzt ist.
pub fn lang(db: &Db) -> Lang {
    db.setting(state::KEY_LANGUAGE)
        .ok()
        .flatten()
        .map(|value| Lang::parse(&value))
        .unwrap_or(Lang::De)
}

/// (Kennung, deutsch, englisch)
const MESSAGES: &[(&str, &str, &str)] = &[
    // --- Projekte ---
    ("project.name_empty", "Der Projektname darf nicht leer sein.", "The project name cannot be empty."),
    ("project.missing", "Dieses Projekt existiert nicht.", "This project does not exist."),
    ("project.gone", "Dieses Projekt existiert nicht mehr.", "This project no longer exists."),
    ("project.running", "Für dieses Projekt läuft gerade eine Erfassung.", "Tracking is currently running for this project."),
    ("project.unreadable", "Projekt konnte nicht gelesen werden.", "The project could not be read."),
    // --- Erfassung ---
    ("track.already_running", "Die Erfassung läuft bereits.", "Tracking is already running."),
    ("track.not_running", "Es läuft gerade keine Erfassung.", "No tracking is running right now."),
    ("track.already_paused", "Die Erfassung pausiert bereits.", "Tracking is already paused."),
    ("track.project_running", "„{name}“ läuft bereits.", "“{name}” is already running."),
    ("track.start", "Start: {name}", "Started: {name}"),
    ("track.stop", "Stopp: {name} ({time})", "Stopped: {name} ({time})"),
    ("track.pause", "Pause", "Break"),
    ("track.resume", "Weiter", "Resumed"),
    ("track.project_fallback", "Projekt", "Project"),
    // --- Slots ---
    ("slot.only_two", "Es gibt im MVP nur die Slots 1 und 2.", "Only slots 1 and 2 exist in this version."),
    ("slot.unassigned", "Slot {slot} ist keinem Projekt zugeordnet - bitte im Hauptfenster festlegen.", "Slot {slot} has no project assigned - please set one in the main window."),
    ("slot.start", "Start: {name} (Slot {slot})", "Started: {name} (slot {slot})"),
    ("slot.resumed", "{name} fortgesetzt (Slot {slot})", "{name} resumed (slot {slot})"),
    ("slot.already_running", "Slot {slot}: {name} (läuft bereits)", "Slot {slot}: {name} (already running)"),
    ("slot.switched", "Wechsel zu {name} (Slot {slot})", "Switched to {name} (slot {slot})"),
    ("slot.switched_from_pause", "Aus Pause gewechselt zu {name} (Slot {slot})", "Switched from break to {name} (slot {slot})"),
    ("slot.no_project", "Kein Projekt auf Slot 1 oder 2 - bitte im Hauptfenster zuordnen.", "No project on slot 1 or 2 - please assign one in the main window."),
    // --- Gesten ---
    ("gesture.unknown", "Unbekannte Geste: {name}", "Unknown gesture: {name}"),
    ("gesture.low_confidence", "Nicht sicher genug ({value} % < {threshold} %)", "Not confident enough ({value}% < {threshold}%)"),
    ("gesture.custom_gone", "Diese eigene Geste existiert nicht mehr.", "This custom gesture no longer exists."),
    ("gesture.custom_no_project", "Dieser Geste ist kein Projekt zugeordnet.", "This gesture has no project assigned."),
    ("gesture.custom_needs_name", "Die Geste braucht einen Namen.", "The gesture needs a name."),
    ("gesture.action_unknown", "Unbekannte Aktion: {name}", "Unknown action: {name}"),
    ("gesture.action_needs_project", "Für „bestimmtes Projekt starten“ muss ein Projekt gewählt sein.", "“Start a specific project” requires a project."),
    ("gesture.no_samples", "Keine brauchbare Aufnahme - war die Hand im Bild?", "No usable recording - was your hand in frame?"),
    ("gesture.feature_mismatch", "Die Aufnahme passt nicht zum aktuellen Merkmalssatz.", "The recording does not match the current feature set."),
    ("gesture.feature_count", "Unerwartete Merkmalszahl: {found} statt {expected}.", "Unexpected number of features: {found} instead of {expected}."),
    ("gesture.open_hand", "Offene Hand", "Open hand"),
    ("gesture.fist", "Faust", "Fist"),
    ("gesture.thumb_up", "Daumen hoch", "Thumbs up"),
    ("gesture.one_finger", "Ein Finger", "One finger"),
    ("gesture.two_fingers", "Zwei Finger", "Two fingers"),
    // --- Aktionen eigener Gesten ---
    ("action.start", "Start", "Start"),
    ("action.stop", "Stopp", "Stop"),
    ("action.pause", "Pause", "Pause"),
    ("action.resume", "Weiter", "Resume"),
    ("action.slot_1", "Projekt-Slot 1", "Project slot 1"),
    ("action.slot_2", "Projekt-Slot 2", "Project slot 2"),
    ("action.project", "bestimmtes Projekt starten", "start a specific project"),
    // --- Einträge ---
    ("entry.gone", "Dieser Eintrag existiert nicht mehr.", "This entry no longer exists."),
    ("entry.is_running", "Das ist der laufende Eintrag - bitte zuerst die Erfassung stoppen.", "This is the running entry - please stop tracking first."),
    ("entry.end_before_start", "Das Ende muss nach dem Beginn liegen.", "The end must be after the start."),
    ("entry.negative_pause", "Die Pause kann nicht negativ sein.", "The break cannot be negative."),
    ("entry.pause_too_long", "Die Pause ist länger als der Eintrag selbst.", "The break is longer than the entry itself."),
    ("entry.bad_timestamp", "„{value}“ ist keine gültige Zeitangabe.", "“{value}” is not a valid date and time."),
    ("entry.ambiguous_timestamp", "Mehrdeutige Zeitangabe: {value}", "Ambiguous date and time: {value}"),
    ("entry.deleted_project", "(gelöscht)", "(deleted)"),
    // --- Kamera ---
    ("camera.url_scheme", "Die Adresse muss mit http:// oder https:// beginnen, z. B. http://192.168.1.20:4747/video", "The address must start with http:// or https://, e.g. http://192.168.1.20:4747/video"),
    ("camera.url_incomplete", "Die Adresse ist unvollständig.", "The address is incomplete."),
    ("camera.url_missing", "Bitte zuerst eine Adresse eintragen.", "Please enter an address first."),
    ("camera.unreachable", "Netzwerk-Kamera nicht erreichbar: {error}", "Network camera not reachable: {error}"),
    ("camera.aborted", "Verbindung zur Netzwerk-Kamera abgebrochen: {error}", "Connection to the network camera was lost: {error}"),
    ("camera.probe_aborted", "Verbindung abgebrochen: {error}", "Connection lost: {error}"),
    ("camera.html_response", "Die Adresse liefert eine Webseite statt eines Videostreams. Häufigste Ursache: der Stream ist schon von einem anderen Programm belegt (Kamera-App am Rechner, Browser-Tab, OBS). Sonst die Stream-Adresse prüfen - bei DroidCam etwa http://<ip>:4747/video oder http://<ip>:4747/mjpegfeed.", "The address returns a web page instead of a video stream. Most likely the stream is already in use by another program (a desktop camera app, a browser tab, OBS). Otherwise check the stream address - with DroidCam it is usually http://<ip>:4747/video or http://<ip>:4747/mjpegfeed."),
    ("camera.wrong_type", "Die Adresse liefert „{type}“ statt Bilddaten - bitte die Stream-Adresse prüfen.", "The address returns “{type}” instead of image data - please check the stream address."),
    ("camera.no_frame", "Verbunden, aber es kam kein vollständiges Bild an.", "Connected, but no complete frame arrived."),
    ("camera.probe_ok", "Verbindung steht: {kind}, erstes Bild {size}.", "Connection works: {kind}, first frame {size}."),
    ("camera.kind_stream", "Videostream", "video stream"),
    ("camera.kind_still", "Einzelbild", "single frame"),
    ("camera.pixels", "{width} × {height} Pixel", "{width} × {height} pixels"),
    // --- Hotkey ---
    ("hotkey.invalid", "Ungültige Tastenkombination: {value}", "Invalid key combination: {value}"),
    ("hotkey.taken", "Die Tastenkombination {value} ist bereits von einem anderen Programm belegt.", "The key combination {value} is already used by another program."),
    // --- Fenster ---
    ("window.overlay_missing", "Overlay-Fenster wurde nicht gefunden.", "The overlay window was not found."),
    ("window.main_missing", "Hauptfenster wurde nicht gefunden.", "The main window was not found."),
    ("window.panel_missing", "Menüleisten-Fenster wurde nicht gefunden.", "The menu bar window was not found."),
    // --- Datenbank ---
    ("db.error", "Datenbankfehler: {error}", "Database error: {error}"),
    ("db.io", "Dateifehler: {error}", "File error: {error}"),
    ("db.csv", "CSV-Fehler: {error}", "CSV error: {error}"),
    ("db.window", "Fensterfehler: {error}", "Window error: {error}"),
    ("db.features_unreadable", "Merkmale unlesbar: {error}", "Features could not be read: {error}"),
    ("db.bad_timestamp", "Ungültiger Zeitstempel: {value}", "Invalid timestamp: {value}"),
    // --- Tray ---
    ("tray.stopped", "Gestoppt", "Stopped"),
    ("tray.break", "{name} · Pause ({time})", "{name} · break ({time})"),
    ("tray.today", "Heute erfasst: {time}", "Tracked today: {time}"),
    ("tray.start_project", "Projekt starten", "Start project"),
    ("tray.switch_project", "Projekt wechseln", "Switch project"),
    ("tray.no_projects", "Noch keine Projekte", "No projects yet"),
    ("tray.project_running", "{name} (läuft)", "{name} (running)"),
    ("tray.project_paused", "{name} (pausiert)", "{name} (on break)"),
    ("tray.stop", "Erfassung stoppen", "Stop tracking"),
    ("tray.pause", "Pausieren", "Pause"),
    ("tray.resume", "Fortsetzen", "Resume"),
    ("tray.gesture", "Geste erfassen", "Capture gesture"),
    ("tray.window", "Fenster öffnen", "Open window"),
    ("tray.quit", "Beenden", "Quit"),
    ("tray.tooltip_idle", "Gesture TimeTrack - gestoppt\nHeute: {today}", "Gesture TimeTrack - stopped\nToday: {today}"),
    ("tray.tooltip_active", "{name} - {status}\nLaufend: {running}\nHeute: {today}", "{name} - {status}\nRunning: {running}\nToday: {today}"),
    // --- Status ---
    ("status.idle", "gestoppt", "stopped"),
    ("status.running", "läuft", "running"),
    ("status.paused", "pausiert", "on break"),
    // --- CSV ---
    ("csv.date", "Datum", "Date"),
    ("csv.project", "Projekt", "Project"),
    ("csv.start", "Beginn", "Start"),
    ("csv.end", "Ende", "End"),
    ("csv.duration", "Dauer (hh:mm:ss)", "Duration (hh:mm:ss)"),
    ("csv.hours", "Dauer (Stunden)", "Duration (hours)"),
    ("csv.break", "Pause (Minuten)", "Break (minutes)"),
    ("csv.gesture", "Per Geste", "By gesture"),
    ("csv.running", "läuft", "running"),
    ("csv.yes", "ja", "yes"),
    ("csv.no", "nein", "no"),
];

/// Übersetzung einer Meldung.
pub fn t(lang: Lang, key: &str) -> String {
    MESSAGES
        .iter()
        .find(|(id, _, _)| *id == key)
        .map(|(_, de, en)| match lang {
            Lang::De => *de,
            Lang::En => *en,
        })
        .unwrap_or(key)
        .to_string()
}

/// Übersetzung mit Platzhaltern: `{name}` wird ersetzt.
pub fn ta(lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
    let mut text = t(lang, key);
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprachkennungen_werden_gelesen() {
        assert_eq!(Lang::parse("de"), Lang::De);
        assert_eq!(Lang::parse("EN"), Lang::En);
        assert_eq!(Lang::parse(" en "), Lang::En);
        // Unbekanntes fällt auf Deutsch zurück, statt leer zu bleiben.
        assert_eq!(Lang::parse("fr"), Lang::De);
        assert_eq!(Lang::parse(""), Lang::De);
    }

    #[test]
    fn jede_meldung_ist_vollstaendig_und_einmalig() {
        let mut keys: Vec<&str> = MESSAGES.iter().map(|(key, _, _)| *key).collect();
        let count = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), count, "doppelte Kennung in der Tabelle");

        for (key, de, en) in MESSAGES {
            assert!(!de.trim().is_empty(), "{key}: deutsche Fassung fehlt");
            assert!(!en.trim().is_empty(), "{key}: englische Fassung fehlt");
        }
    }

    #[test]
    fn aktuelle_sprache_ist_umschaltbar() {
        set_current(Lang::En);
        assert_eq!(current(), Lang::En);
        assert_eq!(msg("track.pause"), "Break");

        set_current(Lang::De);
        assert_eq!(msg("track.pause"), "Pause");
        assert_eq!(msg_error("db.error", "kaputt"), "Datenbankfehler: kaputt");
    }

    #[test]
    fn platzhalter_werden_ersetzt() {
        let text = ta(Lang::De, "track.start", &[("name", "Kunde Meier")]);
        assert_eq!(text, "Start: Kunde Meier");
        assert_eq!(
            ta(Lang::En, "track.start", &[("name", "Acme")]),
            "Started: Acme"
        );

        // Unbekannte Kennung: die Kennung selbst, damit der Fehler auffällt.
        assert_eq!(t(Lang::De, "gibt.es.nicht"), "gibt.es.nicht");
    }

    #[test]
    fn platzhalter_stimmen_in_beiden_sprachen_ueberein() {
        fn placeholders(text: &str) -> Vec<String> {
            let mut found: Vec<String> = text
                .split('{')
                .skip(1)
                .filter_map(|part| part.split('}').next().map(|name| name.to_string()))
                .collect();
            found.sort();
            found
        }

        for (key, de, en) in MESSAGES {
            assert_eq!(
                placeholders(de),
                placeholders(en),
                "{key}: Platzhalter unterscheiden sich"
            );
        }
    }
}
