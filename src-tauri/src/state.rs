//! Zustandsmaschine der Zeiterfassung.
//!
//! Die Gesten kommen aus dem Overlay-Frontend, die Entscheidung darüber, was
//! eine Geste bewirkt, fällt bewusst hier im Backend: so gibt es genau eine
//! Wahrheit über "läuft / pausiert / gestoppt", egal ob der Auslöser eine
//! Geste, das Tray-Menü oder ein Klick im Hauptfenster war.

use std::sync::Mutex;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};

use crate::db::{Db, Project, RESUME_LIMIT_SECONDS};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerStatus {
    Idle,
    Running,
    Paused,
}

impl TrackerStatus {
    /// Beschriftung in der eingestellten Sprache.
    pub fn label(&self) -> String {
        crate::i18n::msg(match self {
            TrackerStatus::Idle => "status.idle",
            TrackerStatus::Running => "status.running",
            TrackerStatus::Paused => "status.paused",
        })
    }
}

/// Das Gesten-Vokabular des MVP. Die Klassifikation der Handlandmarks passiert
/// im Frontend, hier kommt nur noch das Ergebnis an.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
    /// Offene Hand, fünf Finger gestreckt
    OpenHand,
    /// Faust, kein Finger gestreckt
    Fist,
    ThumbUp,
    /// Ein Finger gestreckt -> Projekt-Slot 1
    OneFinger,
    /// Zwei Finger gestreckt -> Projekt-Slot 2
    TwoFingers,
}

/// Das vollständige Vokabular - Grundlage für die Prüfung, ob das Training
/// vollständig ist.
pub const ALL_GESTURES: [Gesture; 5] = [
    Gesture::OpenHand,
    Gesture::Fist,
    Gesture::ThumbUp,
    Gesture::OneFinger,
    Gesture::TwoFingers,
];

impl Gesture {
    /// Bezeichner, wie er über die Schnittstelle und in der Datenbank steht.
    /// Muss zur serde-Schreibweise passen; dazu gibt es einen Test.
    pub fn key(&self) -> &'static str {
        match self {
            Gesture::OpenHand => "open_hand",
            Gesture::Fist => "fist",
            Gesture::ThumbUp => "thumb_up",
            Gesture::OneFinger => "one_finger",
            Gesture::TwoFingers => "two_fingers",
        }
    }

    /// Beschriftung für Meldungen. Die Oberfläche hat eigene Texte; hier geht es
    /// um die Rückmeldung im Overlay.
    pub fn label(&self) -> String {
        crate::i18n::msg(match self {
            Gesture::OpenHand => "gesture.open_hand",
            Gesture::Fist => "gesture.fist",
            Gesture::ThumbUp => "gesture.thumb_up",
            Gesture::OneFinger => "gesture.one_finger",
            Gesture::TwoFingers => "gesture.two_fingers",
        })
    }
}

/// Was eine selbst definierte Geste auslöst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomAction {
    Start,
    Stop,
    Pause,
    Resume,
    // Ausdrücklich benannt: serde würde daraus sonst „slot1" machen.
    #[serde(rename = "slot_1")]
    Slot1,
    #[serde(rename = "slot_2")]
    Slot2,
    /// Ein bestimmtes Projekt starten - unabhängig von den beiden Slots.
    Project,
}

impl CustomAction {
    pub fn key(&self) -> &'static str {
        match self {
            CustomAction::Start => "start",
            CustomAction::Stop => "stop",
            CustomAction::Pause => "pause",
            CustomAction::Resume => "resume",
            CustomAction::Slot1 => "slot_1",
            CustomAction::Slot2 => "slot_2",
            CustomAction::Project => "project",
        }
    }

    pub fn label(&self) -> String {
        crate::i18n::msg(match self {
            CustomAction::Start => "action.start",
            CustomAction::Stop => "action.stop",
            CustomAction::Pause => "action.pause",
            CustomAction::Resume => "action.resume",
            CustomAction::Slot1 => "action.slot_1",
            CustomAction::Slot2 => "action.slot_2",
            CustomAction::Project => "action.project",
        })
    }
}

pub const ALL_CUSTOM_ACTIONS: [CustomAction; 7] = [
    CustomAction::Start,
    CustomAction::Stop,
    CustomAction::Pause,
    CustomAction::Resume,
    CustomAction::Slot1,
    CustomAction::Slot2,
    CustomAction::Project,
];

pub fn parse_custom_action(name: &str) -> AppResult<CustomAction> {
    serde_json::from_value::<CustomAction>(serde_json::Value::String(name.to_string()))
        .map_err(|_| AppError::args("gesture.action_unknown", &[("name", &name)]))
}

/// Kennung einer eigenen Geste in `gesture_samples`.
pub fn custom_sample_key(id: i64) -> String {
    format!("custom:{id}")
}

/// Liest die Kennung wieder aus, `None` bei einer Grundgeste.
pub fn parse_custom_sample_key(key: &str) -> Option<i64> {
    key.strip_prefix("custom:")?.parse().ok()
}

/// Laufende Sitzung: der Teil des Zustands, der nicht in der DB steht.
#[derive(Debug, Clone)]
pub struct Session {
    pub entry_id: i64,
    pub project_id: i64,
    pub start: DateTime<Local>,
    /// Bereits abgeschlossene Pausenzeit dieser Sitzung.
    pub pause_seconds: i64,
    /// Beginn der aktuell laufenden Pause, falls pausiert.
    pub paused_at: Option<DateTime<Local>>,
}

impl Session {
    /// Reine Arbeitszeit: Gesamtdauer abzüglich aller Pausen.
    pub fn worked_seconds(&self, now: DateTime<Local>) -> i64 {
        let gross = (now - self.start).num_seconds().max(0);
        (gross - self.total_pause_seconds(now)).max(0)
    }

    pub fn total_pause_seconds(&self, now: DateTime<Local>) -> i64 {
        let running = self
            .paused_at
            .map(|since| (now - since).num_seconds().max(0))
            .unwrap_or(0);
        self.pause_seconds + running
    }
}

#[derive(Default)]
pub struct Tracker(pub Mutex<Option<Session>>);

impl Tracker {
    pub fn get(&self) -> Option<Session> {
        self.0.lock().expect("Tracker-Mutex vergiftet").clone()
    }

    pub fn set(&self, session: Option<Session>) {
        *self.0.lock().expect("Tracker-Mutex vergiftet") = session;
    }

    pub fn status(&self) -> TrackerStatus {
        match self.get() {
            None => TrackerStatus::Idle,
            Some(session) if session.paused_at.is_some() => TrackerStatus::Paused,
            Some(_) => TrackerStatus::Running,
        }
    }
}

/// Momentaufnahme für Frontend, Tray und Overlay.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub status: TrackerStatus,
    pub status_label: String,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
    pub project_color: Option<String>,
    pub active_slot: Option<u8>,
    /// Reine Arbeitszeit der laufenden Sitzung.
    pub elapsed_seconds: i64,
    pub pause_seconds: i64,
    /// Heute bereits abgeschlossene Zeit (ohne die laufende Sitzung).
    pub today_seconds: i64,
    pub slots: Vec<SlotInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlotInfo {
    pub slot: u8,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
    pub project_color: Option<String>,
}

// ---------- Einstellungen ----------

pub const KEY_HOTKEY: &str = "hotkey";
pub const KEY_CONFIDENCE: &str = "confidence_threshold";
pub const KEY_OVERLAY_TIMEOUT: &str = "overlay_timeout_ms";
pub const KEY_SOUND: &str = "sound_cue";
pub const KEY_SLOT_1: &str = "slot_1_project_id";
pub const KEY_SLOT_2: &str = "slot_2_project_id";
pub const KEY_ACTIVE_SLOT: &str = "active_slot";
/// Sollen die eingelernten Gesten statt der geometrischen Regeln gelten?
pub const KEY_USE_TRAINING: &str = "use_training";
/// Sprache der Oberfläche und der Meldungen: "de" oder "en".
pub const KEY_LANGUAGE: &str = "language";
/// Bildquelle der Erkennung: "builtin" (eingebaute Webcam) oder "network".
pub const KEY_CAMERA_SOURCE: &str = "camera_source";
pub const KEY_CAMERA_URL: &str = "camera_url";
/// Eigenes Zeitfenster für Netzwerk-Kameras - MJPEG über WLAN hat spürbar
/// mehr Latenz als die lokale Webcam.
pub const KEY_OVERLAY_TIMEOUT_NETWORK: &str = "overlay_timeout_network_ms";

pub const DEFAULT_HOTKEY: &str = "CommandOrControl+Alt+Space";
pub const DEFAULT_CONFIDENCE: f64 = 0.85;
pub const DEFAULT_OVERLAY_TIMEOUT_MS: i64 = 3000;
pub const DEFAULT_OVERLAY_TIMEOUT_NETWORK_MS: i64 = 4000;
pub const CAMERA_SOURCE_BUILTIN: &str = "builtin";
pub const CAMERA_SOURCE_NETWORK: &str = "network";

pub fn settings_defaults() -> Vec<(&'static str, String)> {
    vec![
        (KEY_HOTKEY, DEFAULT_HOTKEY.to_string()),
        (KEY_CONFIDENCE, DEFAULT_CONFIDENCE.to_string()),
        (KEY_OVERLAY_TIMEOUT, DEFAULT_OVERLAY_TIMEOUT_MS.to_string()),
        (
            KEY_OVERLAY_TIMEOUT_NETWORK,
            DEFAULT_OVERLAY_TIMEOUT_NETWORK_MS.to_string(),
        ),
        (KEY_CAMERA_SOURCE, CAMERA_SOURCE_BUILTIN.to_string()),
        (KEY_CAMERA_URL, String::new()),
        (KEY_SOUND, "1".to_string()),
        (KEY_ACTIVE_SLOT, "1".to_string()),
        (KEY_USE_TRAINING, "0".to_string()),
        (KEY_LANGUAGE, "de".to_string()),
    ]
}

pub fn confidence_threshold(db: &Db) -> f64 {
    db.setting(KEY_CONFIDENCE)
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<f64>().ok())
        .filter(|value| (0.0..=1.0).contains(value))
        .unwrap_or(DEFAULT_CONFIDENCE)
}

pub fn hotkey(db: &Db) -> String {
    db.setting(KEY_HOTKEY)
        .ok()
        .flatten()
        .filter(|raw| !raw.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HOTKEY.to_string())
}

/// Merkmalssatz, den das Frontend schickt (siehe src/gesture/features.ts).
/// Wird der dort geändert, muss diese Zahl mitwachsen - dann gelten alte
/// Aufnahmen als veraltet und werden nicht mehr verwendet.
pub const FEATURE_VERSION: i64 = 2;
/// Länge eines Merkmalsvektors. Dient nur der Plausibilitätsprüfung.
pub const FEATURE_LEN: usize = 10;

/// Prüft einen eingehenden Gestennamen gegen das bekannte Vokabular.
pub fn parse_gesture(name: &str) -> AppResult<Gesture> {
    serde_json::from_value::<Gesture>(serde_json::Value::String(name.to_string()))
        .map_err(|_| AppError::args("gesture.unknown", &[("name", &name)]))
}

/// Ein von Hand erfasster oder nachträglich geänderter Zeiteintrag.
#[derive(Debug, Clone, Deserialize)]
pub struct EntryInput {
    pub project_id: i64,
    /// Beginn, z. B. „2026-08-19T09:37" oder „2026-08-19 09:37:00".
    pub start: String,
    pub end: String,
    /// Pausendauer in Minuten - so gibt der Nutzer sie ein.
    #[serde(default)]
    pub pause_minutes: i64,
}

/// Fertig geprüfter Eintrag, wie er in die Datenbank geht.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedEntry {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub pause_seconds: i64,
    /// Nettozeit: Gesamtdauer abzüglich Pause.
    pub duration_seconds: i64,
}

/// Nimmt die Eingaben aus dem Formular an. Bewusst nachsichtig beim Format,
/// streng bei der Logik.
pub fn parse_input_ts(raw: &str) -> AppResult<DateTime<Local>> {
    let normalized = raw.trim().replace('T', " ");
    for format in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, format) {
            return Local
                .from_local_datetime(&naive)
                .single()
                .ok_or_else(|| AppError::args("entry.ambiguous_timestamp", &[("value", &raw)]));
        }
    }
    Err(AppError::args("entry.bad_timestamp", &[("value", &raw)]))
}

/// Prüft einen Eintrag und rechnet die Nettozeit aus.
pub fn plan_entry(input: &EntryInput) -> AppResult<PlannedEntry> {
    let start = parse_input_ts(&input.start)?;
    let end = parse_input_ts(&input.end)?;

    let gross = (end - start).num_seconds();
    if gross <= 0 {
        return Err(AppError::key("entry.end_before_start"));
    }
    if input.pause_minutes < 0 {
        return Err(AppError::key("entry.negative_pause"));
    }

    let pause_seconds = input.pause_minutes * 60;
    if pause_seconds >= gross {
        return Err(AppError::key("entry.pause_too_long"));
    }

    Ok(PlannedEntry {
        start,
        end,
        pause_seconds,
        duration_seconds: gross - pause_seconds,
    })
}

/// Prüft die Adresse einer Netzwerk-Kamera. Bewusst nachsichtig: geprüft wird
/// nur das Schema, nicht die Erreichbarkeit - das merkt der Nutzer sofort am
/// roten Rahmen im Overlay.
pub fn validate_camera_url(raw: &str) -> AppResult<()> {
    let url = raw.trim();
    if url.is_empty() {
        return Ok(());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::key("camera.url_scheme"));
    }
    if url.len() < "http://a".len() {
        return Err(AppError::key("camera.url_incomplete"));
    }
    Ok(())
}

pub fn slot_key(slot: u8) -> AppResult<&'static str> {
    match slot {
        1 => Ok(KEY_SLOT_1),
        2 => Ok(KEY_SLOT_2),
        _ => Err(AppError::key("slot.only_two")),
    }
}

pub fn slot_project(db: &Db, slot: u8) -> AppResult<Option<Project>> {
    let key = slot_key(slot)?;
    let Some(raw) = db.setting(key)? else {
        return Ok(None);
    };
    let Ok(id) = raw.parse::<i64>() else {
        return Ok(None);
    };
    db.project(id)
}

/// Zuletzt per Geste gewählter Slot - bestimmt, welches Projekt eine
/// Start-Geste erfasst, solange nichts läuft.
pub fn active_slot(db: &Db) -> u8 {
    db.setting(KEY_ACTIVE_SLOT)
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<u8>().ok())
        .filter(|slot| matches!(slot, 1 | 2))
        .unwrap_or(1)
}

pub fn slot_of_project(db: &Db, project_id: i64) -> Option<u8> {
    for slot in [1u8, 2u8] {
        if let Ok(Some(project)) = slot_project(db, slot) {
            if project.id == project_id {
                return Some(slot);
            }
        }
    }
    None
}

/// Baut die Momentaufnahme aus DB-Stand und laufender Sitzung.
pub fn snapshot(db: &Db, tracker: &Tracker) -> AppResult<Snapshot> {
    let now = Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let session = tracker.get();

    let (project, elapsed, pause) = match &session {
        Some(session) => (
            db.project(session.project_id)?,
            session.worked_seconds(now),
            session.total_pause_seconds(now),
        ),
        None => (None, 0, 0),
    };

    let mut slots = Vec::with_capacity(2);
    for slot in [1u8, 2u8] {
        let project = slot_project(db, slot)?;
        slots.push(SlotInfo {
            slot,
            project_id: project.as_ref().map(|p| p.id),
            project_name: project.as_ref().map(|p| p.name.clone()),
            project_color: project.as_ref().map(|p| p.color.clone()),
        });
    }

    let status = tracker.status();
    Ok(Snapshot {
        status,
        status_label: status.label().to_string(),
        project_id: project.as_ref().map(|p| p.id),
        project_name: project.as_ref().map(|p| p.name.clone()),
        project_color: project.as_ref().map(|p| p.color.clone()),
        active_slot: project
            .as_ref()
            .and_then(|p| slot_of_project(db, p.id))
            .or(Some(active_slot(db))),
        elapsed_seconds: elapsed,
        pause_seconds: pause,
        today_seconds: db.day_total(&today)?,
        slots,
    })
}

/// Stellt nach einem Neustart eine noch offene Sitzung wieder her, solange sie
/// jung genug ist. Ältere Leichen werden mit ihrer bis dahin bekannten Dauer
/// geschlossen, damit keine unrealistischen Einträge entstehen.
pub fn recover_session(db: &Db, tracker: &Tracker) -> AppResult<()> {
    let Some((entry_id, project_id, start, pause_seconds)) = db.dangling_entry()? else {
        return Ok(());
    };
    let now = Local::now();
    let age = (now - start).num_seconds();

    if age <= RESUME_LIMIT_SECONDS {
        tracker.set(Some(Session {
            entry_id,
            project_id,
            start,
            pause_seconds,
            paused_at: Some(now),
        }));
    } else {
        db.close_entry(entry_id, start, pause_seconds, 0)?;
    }
    Ok(())
}

pub fn format_hms(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn session_at(start: DateTime<Local>) -> Session {
        Session {
            entry_id: 1,
            project_id: 1,
            start,
            pause_seconds: 0,
            paused_at: None,
        }
    }

    #[test]
    fn arbeitszeit_ist_bruttozeit_minus_pausen() {
        let now = Local::now();
        let mut session = session_at(now - Duration::minutes(60));
        session.pause_seconds = 15 * 60;

        assert_eq!(session.worked_seconds(now), 45 * 60);
        assert_eq!(session.total_pause_seconds(now), 15 * 60);
    }

    #[test]
    fn laufende_pause_zaehlt_sofort_mit() {
        let now = Local::now();
        let mut session = session_at(now - Duration::minutes(30));
        session.paused_at = Some(now - Duration::minutes(10));

        assert_eq!(session.total_pause_seconds(now), 10 * 60);
        assert_eq!(session.worked_seconds(now), 20 * 60);
    }

    #[test]
    fn arbeitszeit_wird_nie_negativ() {
        let now = Local::now();
        let mut session = session_at(now - Duration::minutes(5));
        // Unplausibler Zustand (z. B. nach Zeitumstellung): darf keine
        // negative Dauer in die Datenbank schreiben.
        session.pause_seconds = 60 * 60;

        assert_eq!(session.worked_seconds(now), 0);
    }

    #[test]
    fn aktionsnamen_passen_zur_serde_schreibweise() {
        for action in ALL_CUSTOM_ACTIONS {
            let json = serde_json::to_string(&action).unwrap();
            assert_eq!(json, format!("\"{}\"", action.key()));
            assert_eq!(parse_custom_action(action.key()).unwrap(), action);
        }
        assert!(parse_custom_action("loeschen").is_err());
    }

    #[test]
    fn kennung_eigener_gesten_ist_umkehrbar() {
        assert_eq!(custom_sample_key(7), "custom:7");
        assert_eq!(parse_custom_sample_key("custom:7"), Some(7));
        assert_eq!(parse_custom_sample_key("open_hand"), None);
        assert_eq!(parse_custom_sample_key("custom:abc"), None);
    }

    #[test]
    fn gestennamen_passen_zur_serde_schreibweise() {
        for gesture in ALL_GESTURES {
            let json = serde_json::to_string(&gesture).unwrap();
            assert_eq!(json, format!("\"{}\"", gesture.key()));
            assert_eq!(parse_gesture(gesture.key()).unwrap(), gesture);
        }
        assert!(parse_gesture("winken").is_err());
    }

    #[test]
    fn zeitangaben_werden_in_beiden_schreibweisen_gelesen() {
        let a = parse_input_ts("2026-08-19T09:37").unwrap();
        let b = parse_input_ts("2026-08-19 09:37:00").unwrap();
        assert_eq!(a, b);

        assert!(parse_input_ts("19.08.2026 09:37").is_err());
        assert!(parse_input_ts("").is_err());
    }

    #[test]
    fn eintrag_wird_geprueft_und_netto_gerechnet() {
        let input = EntryInput {
            project_id: 1,
            start: "2026-08-19T09:00".to_string(),
            end: "2026-08-19T11:00".to_string(),
            pause_minutes: 30,
        };
        let planned = plan_entry(&input).unwrap();
        assert_eq!(planned.pause_seconds, 1800);
        assert_eq!(planned.duration_seconds, 5400, "2 h minus 30 min Pause");
    }

    #[test]
    fn unmoegliche_eintraege_werden_abgelehnt() {
        let base = EntryInput {
            project_id: 1,
            start: "2026-08-19T11:00".to_string(),
            end: "2026-08-19T09:00".to_string(),
            pause_minutes: 0,
        };
        assert!(plan_entry(&base).is_err(), "Ende vor Beginn");

        let same = EntryInput {
            end: "2026-08-19T11:00".to_string(),
            ..base.clone()
        };
        assert!(plan_entry(&same).is_err(), "Dauer null");

        let long_pause = EntryInput {
            start: "2026-08-19T09:00".to_string(),
            end: "2026-08-19T10:00".to_string(),
            pause_minutes: 60,
            ..base.clone()
        };
        assert!(plan_entry(&long_pause).is_err(), "Pause frisst den Eintrag");

        let negative = EntryInput {
            start: "2026-08-19T09:00".to_string(),
            end: "2026-08-19T10:00".to_string(),
            pause_minutes: -5,
            ..base
        };
        assert!(plan_entry(&negative).is_err(), "negative Pause");
    }

    #[test]
    fn kameraadresse_wird_auf_das_schema_geprueft() {
        assert!(validate_camera_url("").is_ok(), "leer = keine Netzwerk-Kamera");
        assert!(validate_camera_url("http://192.168.1.20:4747/video").is_ok());
        assert!(validate_camera_url("https://kamera.local/shot.jpg").is_ok());

        assert!(validate_camera_url("192.168.1.20:4747/video").is_err());
        assert!(validate_camera_url("rtsp://192.168.1.20/live").is_err());
    }

    #[test]
    fn dauer_wird_als_hms_formatiert() {
        assert_eq!(format_hms(0), "00:00:00");
        assert_eq!(format_hms(59), "00:00:59");
        assert_eq!(format_hms(3661), "01:01:01");
        assert_eq!(format_hms(-5), "00:00:00");
    }
}
