//! Alle Tauri-Commands sowie die Aktionen, die Gesten, Tray-Menü und
//! Hauptfenster gemeinsam nutzen.

use chrono::Local;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::camera::NetworkCamera;
use crate::db::{CustomGesture, Db, GestureSample, Project, ProjectTotal, TimeEntry};
use crate::error::{AppError, AppResult};
use crate::i18n;
use crate::export;
use crate::overlay;
use crate::state::{
    self, format_hms, Gesture, Session, Snapshot, Tracker, TrackerStatus, KEY_ACTIVE_SLOT,
};
use crate::tray;

pub const EVENT_STATE: &str = "tracker:state";

/// Baut die aktuelle Momentaufnahme, schickt sie an alle Fenster und
/// aktualisiert das Tray-Icon.
pub fn broadcast_state(app: &AppHandle) -> AppResult<Snapshot> {
    let snapshot = {
        let db = app.state::<Db>();
        let tracker = app.state::<Tracker>();
        state::snapshot(&db, &tracker)?
    };
    app.emit(EVENT_STATE, &snapshot)?;

    let handle = app.clone();
    let for_tray = snapshot.clone();
    on_main_thread(app, move || tray::refresh(&handle, &for_tray));
    Ok(snapshot)
}

/// Zieht nur die Uhr im Tray nach, ohne Ereignis an die Fenster.
pub fn refresh_tray_clock(app: &AppHandle) -> AppResult<()> {
    let snapshot = {
        let db = app.state::<Db>();
        let tracker = app.state::<Tracker>();
        state::snapshot(&db, &tracker)?
    };

    let handle = app.clone();
    on_main_thread(app, move || tray::set_clock(&handle, &snapshot));
    Ok(())
}

/// Führt Tray-Arbeit auf dem Haupt-Thread aus.
///
/// Nötig, weil die Menüleiste unter macOS nur von dort zuverlässig aktualisiert
/// wird - aus dem Uhren-Thread heraus blieb die Anzeige sonst stehen.
fn on_main_thread<F: FnOnce() + Send + 'static>(app: &AppHandle, task: F) {
    if let Err(error) = app.run_on_main_thread(task) {
        eprintln!("[tray] Aktualisierung nicht möglich: {error}");
    }
}

// ---------- Aktionen ----------

fn start_session(app: &AppHandle, project_id: i64, by_gesture: bool) -> AppResult<String> {
    let db = app.state::<Db>();
    let tracker = app.state::<Tracker>();

    let project = db
        .project(project_id)?
        .ok_or_else(|| AppError::key("project.gone"))?;

    if let Some(current) = tracker.get() {
        if current.project_id == project_id {
            return Err(AppError::args(
                "track.project_running",
                &[("name", &project.name)],
            ));
        }
        stop_session(app, by_gesture)?;
    }

    let now = Local::now();
    let entry_id = db.open_entry(project_id, now, by_gesture)?;
    tracker.set(Some(Session {
        entry_id,
        project_id,
        start: now,
        pause_seconds: 0,
        paused_at: None,
    }));

    if let Some(slot) = state::slot_of_project(&db, project_id) {
        db.set_setting(KEY_ACTIVE_SLOT, &slot.to_string())?;
    }
    Ok(i18n::msg_args("track.start", &[("name".into(), project.name)]))
}

fn stop_session(app: &AppHandle, _by_gesture: bool) -> AppResult<String> {
    let db = app.state::<Db>();
    let tracker = app.state::<Tracker>();

    let session = tracker
        .get()
        .ok_or_else(|| AppError::key("track.not_running"))?;

    let now = Local::now();
    let worked = session.worked_seconds(now);
    let pause = session.total_pause_seconds(now);
    db.close_entry(session.entry_id, now, pause, worked)?;
    tracker.set(None);

    let name = db
        .project(session.project_id)?
        .map(|project| project.name)
        .unwrap_or_else(|| i18n::msg("track.project_fallback"));
    Ok(i18n::msg_args(
        "track.stop",
        &[("name".into(), name), ("time".into(), format_hms(worked))],
    ))
}

fn pause_session(app: &AppHandle) -> AppResult<String> {
    let tracker = app.state::<Tracker>();
    let mut session = tracker
        .get()
        .ok_or_else(|| AppError::key("track.not_running"))?;
    if session.paused_at.is_some() {
        return Err(AppError::key("track.already_paused"));
    }
    session.paused_at = Some(Local::now());
    tracker.set(Some(session));
    Ok(i18n::msg("track.pause"))
}

fn resume_session(app: &AppHandle) -> AppResult<String> {
    let tracker = app.state::<Tracker>();
    let mut session = tracker
        .get()
        .ok_or_else(|| AppError::key("track.not_running"))?;
    let Some(paused_at) = session.paused_at else {
        return Err(AppError::key("track.already_running"));
    };
    let now = Local::now();
    session.pause_seconds += (now - paused_at).num_seconds().max(0);
    session.paused_at = None;
    tracker.set(Some(session));

    // Pausendauer sofort mitschreiben, damit sie einen Absturz überlebt.
    let db = app.state::<Db>();
    if let Some(current) = tracker.get() {
        let conn = db.lock();
        conn.execute(
            "UPDATE time_entries SET pause_duration_seconds = ?2 WHERE id = ?1",
            rusqlite::params![current.entry_id, current.pause_seconds],
        )?;
    }
    Ok(i18n::msg("track.resume"))
}

/// Slot-Wahl: Das Projekt des Slots wird **erfasst**, nicht bloß vorgemerkt.
///
/// Aus jedem Zustand heraus: gestoppt beginnt eine neue Erfassung, pausiert
/// setzt fort bzw. wechselt, laufend wird der offene Eintrag abgeschlossen und
/// das neue Projekt sofort begonnen. Wer den Slot wählt, will arbeiten.
fn select_slot(app: &AppHandle, slot: u8, by_gesture: bool) -> AppResult<String> {
    let db = app.state::<Db>();
    let tracker = app.state::<Tracker>();

    let project = state::slot_project(&db, slot)?
        .ok_or_else(|| AppError::args("slot.unassigned", &[("slot", &slot)]))?;
    db.set_setting(KEY_ACTIVE_SLOT, &slot.to_string())?;
    let session = tracker.get();
    drop(db);

    match session {
        // Schon dieses Projekt: aus der Pause fortsetzen, sonst nichts zu tun.
        Some(session) if session.project_id == project.id => {
            if session.paused_at.is_some() {
                resume_session(app)?;
                Ok(slot_message("slot.resumed", slot, &project.name))
            } else {
                Ok(slot_message("slot.already_running", slot, &project.name))
            }
        }
        Some(session) => {
            let was_paused = session.paused_at.is_some();
            start_session(app, project.id, by_gesture)?;
            Ok(slot_message(
                if was_paused {
                    "slot.switched_from_pause"
                } else {
                    "slot.switched"
                },
                slot,
                &project.name,
            ))
        }
        None => {
            start_session(app, project.id, by_gesture)?;
            Ok(slot_message("slot.start", slot, &project.name))
        }
    }
}

/// Meldung mit Slot-Nummer und Projektnamen.
fn slot_message(key: &str, slot: u8, name: &str) -> String {
    i18n::msg_args(
        key,
        &[
            ("slot".into(), slot.to_string()),
            ("name".into(), name.to_string()),
        ],
    )
}

// ---------- Gesten ----------

#[derive(Debug, Serialize)]
pub struct GestureOutcome {
    /// Wurde die Geste als Aktion übernommen?
    pub accepted: bool,
    /// Grundgeste - bei einer selbst definierten Geste leer.
    pub gesture: Option<Gesture>,
    pub gesture_label: String,
    pub message: String,
    pub snapshot: Snapshot,
}

/// Nimmt eine im Overlay erkannte Geste entgegen.
///
/// Der Konfidenzwert wird hier ein zweites Mal geprüft: das Frontend filtert
/// bereits, aber der verbindliche Schwellwert steht in der Datenbank.
#[tauri::command]
pub fn apply_gesture(
    app: AppHandle,
    gesture: Gesture,
    confidence: f64,
) -> AppResult<GestureOutcome> {
    let threshold = {
        let db = app.state::<Db>();
        state::confidence_threshold(&db)
    };

    if confidence < threshold {
        let snapshot = broadcast_state(&app)?;
        return Ok(GestureOutcome {
            accepted: false,
            gesture: Some(gesture),
            gesture_label: gesture.label().to_string(),
            message: i18n::msg_args(
                "gesture.low_confidence",
                &[
                    ("value".into(), format!("{:.0}", confidence * 100.0)),
                    ("threshold".into(), format!("{:.0}", threshold * 100.0)),
                ],
            ),
            snapshot,
        });
    }

    let result = match gesture {
        // Die offene Hand deckt Start **und** Fortsetzen ab; eine eigene Geste
        // für „Weiter" gibt es deshalb nicht.
        Gesture::OpenHand => start_from_active_slot(&app),
        Gesture::Fist => stop_session(&app, true),
        Gesture::ThumbUp => pause_session(&app),
        Gesture::OneFinger => select_slot(&app, 1, true),
        Gesture::TwoFingers => select_slot(&app, 2, true),
    };

    let snapshot = broadcast_state(&app)?;
    Ok(match result {
        Ok(message) => GestureOutcome {
            accepted: true,
            gesture: Some(gesture),
            gesture_label: gesture.label().to_string(),
            message,
            snapshot,
        },
        Err(error) => GestureOutcome {
            accepted: false,
            gesture: Some(gesture),
            gesture_label: gesture.label().to_string(),
            message: error.to_string(),
            snapshot,
        },
    })
}

/// „Start" ohne genanntes Projekt: nimmt das Projekt des aktiven Slots, aus der
/// Pause heraus wird fortgesetzt.
fn start_from_active_slot(app: &AppHandle) -> AppResult<String> {
    match app.state::<Tracker>().status() {
        TrackerStatus::Paused => resume_session(app),
        TrackerStatus::Running => Err(AppError::key("track.already_running")),
        TrackerStatus::Idle => {
            let db = app.state::<Db>();
            let slot = state::active_slot(&db);
            let project = state::slot_project(&db, slot)?
                .or(state::slot_project(&db, 1)?)
                .or(state::slot_project(&db, 2)?);
            drop(db);
            match project {
                Some(project) => start_session(app, project.id, true),
                None => Err(AppError::key("slot.no_project")),
            }
        }
    }
}

// ---------- Steuerung aus der Oberfläche ----------

#[tauri::command]
pub fn start_tracking(app: AppHandle, project_id: i64) -> AppResult<Snapshot> {
    start_session(&app, project_id, false)?;
    broadcast_state(&app)
}

#[tauri::command]
pub fn stop_tracking(app: AppHandle) -> AppResult<Snapshot> {
    stop_session(&app, false)?;
    broadcast_state(&app)
}

#[tauri::command]
pub fn pause_tracking(app: AppHandle) -> AppResult<Snapshot> {
    pause_session(&app)?;
    broadcast_state(&app)
}

#[tauri::command]
pub fn resume_tracking(app: AppHandle) -> AppResult<Snapshot> {
    resume_session(&app)?;
    broadcast_state(&app)
}

#[tauri::command]
pub fn set_slot(app: AppHandle, slot: u8, project_id: Option<i64>) -> AppResult<Snapshot> {
    let db = app.state::<Db>();
    let key = state::slot_key(slot)?;
    match project_id {
        Some(id) => db.set_setting(key, &id.to_string())?,
        None => db.set_setting(key, "")?,
    }
    drop(db);
    broadcast_state(&app)
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> AppResult<Snapshot> {
    let db = app.state::<Db>();
    let tracker = app.state::<Tracker>();
    state::snapshot(&db, &tracker)
}

// ---------- Projekte ----------

#[tauri::command]
pub fn get_projects(db: State<'_, Db>, only_active: Option<bool>) -> AppResult<Vec<Project>> {
    db.projects(only_active.unwrap_or(false))
}

#[tauri::command]
pub fn create_project(app: AppHandle, name: String, color: String) -> AppResult<Project> {
    let db = app.state::<Db>();
    let id = db.create_project(&name, &color)?;

    // Erstes und zweites Projekt landen automatisch auf den Gesten-Slots.
    for slot in [1u8, 2u8] {
        if state::slot_project(&db, slot)?.is_none() {
            db.set_setting(state::slot_key(slot)?, &id.to_string())?;
            break;
        }
    }

    let project = db
        .project(id)?
        .ok_or_else(|| AppError::key("project.unreadable"))?;
    drop(db);
    broadcast_state(&app)?;
    Ok(project)
}

#[tauri::command]
pub fn update_project(
    app: AppHandle,
    id: i64,
    name: String,
    color: String,
    active: bool,
) -> AppResult<()> {
    app.state::<Db>().update_project(id, &name, &color, active)?;
    broadcast_state(&app)?;
    Ok(())
}

/// Gibt `false` zurück, wenn das Projekt wegen vorhandener Zeiteinträge nur
/// deaktiviert statt gelöscht wurde.
#[tauri::command]
pub fn delete_project(app: AppHandle, id: i64) -> AppResult<bool> {
    let tracker = app.state::<Tracker>();
    if tracker.get().map(|session| session.project_id) == Some(id) {
        return Err(AppError::key("project.running"));
    }
    let deleted = app.state::<Db>().delete_project(id)?;
    broadcast_state(&app)?;
    Ok(deleted)
}

// ---------- Auswertung ----------

#[tauri::command]
pub fn day_totals(db: State<'_, Db>, day: Option<String>) -> AppResult<Vec<ProjectTotal>> {
    let day = day.unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    db.totals_for_day(&day)
}

#[tauri::command]
pub fn list_entries(
    db: State<'_, Db>,
    from: String,
    to: String,
    project_id: Option<i64>,
) -> AppResult<Vec<TimeEntry>> {
    db.entries_between(&from, &to, project_id)
}

/// Prüft, dass ein Eintrag nicht der gerade laufende ist.
///
/// Der offene Eintrag gehört der Zustandsmaschine: würde er nebenher geändert,
/// stimmten Uhr und Datenbank nicht mehr zusammen.
fn ensure_not_running(app: &AppHandle, id: i64) -> AppResult<()> {
    if app.state::<Tracker>().get().map(|session| session.entry_id) == Some(id) {
        return Err(AppError::key("entry.is_running"));
    }
    Ok(())
}

/// Neuer Eintrag von Hand, z. B. für vergessene Zeiten.
#[tauri::command]
pub fn create_entry(app: AppHandle, input: state::EntryInput) -> AppResult<i64> {
    let planned = state::plan_entry(&input)?;
    let db = app.state::<Db>();
    db.project(input.project_id)?
        .ok_or_else(|| AppError::key("project.missing"))?;

    let id = db.insert_entry(
        input.project_id,
        planned.start,
        planned.end,
        planned.pause_seconds,
        planned.duration_seconds,
    )?;
    drop(db);
    broadcast_state(&app)?;
    Ok(id)
}

#[tauri::command]
pub fn update_entry(app: AppHandle, id: i64, input: state::EntryInput) -> AppResult<()> {
    ensure_not_running(&app, id)?;
    let planned = state::plan_entry(&input)?;

    let db = app.state::<Db>();
    db.entry(id)?
        .ok_or_else(|| AppError::key("entry.gone"))?;
    db.project(input.project_id)?
        .ok_or_else(|| AppError::key("project.missing"))?;

    db.update_entry(
        id,
        input.project_id,
        planned.start,
        planned.end,
        planned.pause_seconds,
        planned.duration_seconds,
    )?;
    drop(db);
    broadcast_state(&app)?;
    Ok(())
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, id: i64) -> AppResult<()> {
    ensure_not_running(&app, id)?;
    app.state::<Db>().delete_entry(id)?;
    broadcast_state(&app)?;
    Ok(())
}

#[tauri::command]
pub fn export_csv(
    db: State<'_, Db>,
    path: String,
    from: String,
    to: String,
    project_id: Option<i64>,
) -> AppResult<usize> {
    export::write_csv(&db, std::path::Path::new(&path), &from, &to, project_id)
}

// ---------- Eigene Gesten ----------

#[derive(Debug, Serialize)]
pub struct ActionOption {
    pub key: &'static str,
    /// Beschriftung in der eingestellten Sprache.
    pub label: String,
    /// Braucht diese Aktion die Angabe eines Projekts?
    pub needs_project: bool,
}

/// Auswahlliste für die Oberfläche - so steht das Vokabular nur an einer Stelle.
#[tauri::command]
pub fn custom_gesture_actions() -> Vec<ActionOption> {
    state::ALL_CUSTOM_ACTIONS
        .iter()
        .map(|action| ActionOption {
            key: action.key(),
            label: action.label(),
            needs_project: *action == state::CustomAction::Project,
        })
        .collect()
}

#[tauri::command]
pub fn get_custom_gestures(db: State<'_, Db>) -> AppResult<Vec<CustomGesture>> {
    db.custom_gestures()
}

fn check_action(action: &str, project_id: Option<i64>) -> AppResult<()> {
    let parsed = state::parse_custom_action(action)?;
    if parsed == state::CustomAction::Project && project_id.is_none() {
        return Err(AppError::key("gesture.action_needs_project"));
    }
    Ok(())
}

#[tauri::command]
pub fn create_custom_gesture(
    app: AppHandle,
    name: String,
    action: String,
    project_id: Option<i64>,
) -> AppResult<Vec<CustomGesture>> {
    check_action(&action, project_id)?;
    let db = app.state::<Db>();
    db.create_custom_gesture(&name, &action, project_id)?;
    db.custom_gestures()
}

#[tauri::command]
pub fn update_custom_gesture(
    app: AppHandle,
    id: i64,
    name: String,
    action: String,
    project_id: Option<i64>,
) -> AppResult<Vec<CustomGesture>> {
    check_action(&action, project_id)?;
    let db = app.state::<Db>();
    db.update_custom_gesture(id, &name, &action, project_id)?;
    db.custom_gestures()
}

#[tauri::command]
pub fn delete_custom_gesture(app: AppHandle, id: i64) -> AppResult<Vec<CustomGesture>> {
    let db = app.state::<Db>();
    db.delete_custom_gesture(id)?;
    db.custom_gestures()
}

/// Führt die Aktion einer selbst definierten Geste aus.
///
/// Läuft absichtlich durch dieselben Funktionen wie die Grundgesten - eine
/// eigene Geste kann also nichts, was das Tray-Menü nicht auch könnte.
#[tauri::command]
pub fn apply_custom_gesture(
    app: AppHandle,
    id: i64,
    confidence: f64,
) -> AppResult<GestureOutcome> {
    let (gesture, threshold) = {
        let db = app.state::<Db>();
        (db.custom_gesture(id)?, state::confidence_threshold(&db))
    };
    let gesture =
        gesture.ok_or_else(|| AppError::key("gesture.custom_gone"))?;

    if confidence < threshold {
        let snapshot = broadcast_state(&app)?;
        return Ok(GestureOutcome {
            accepted: false,
            gesture: None,
            gesture_label: gesture.name.clone(),
            message: i18n::msg_args(
                "gesture.low_confidence",
                &[
                    ("value".into(), format!("{:.0}", confidence * 100.0)),
                    ("threshold".into(), format!("{:.0}", threshold * 100.0)),
                ],
            ),
            snapshot,
        });
    }

    let action = state::parse_custom_action(&gesture.action)?;
    let result = match action {
        state::CustomAction::Start => start_from_active_slot(&app),
        state::CustomAction::Stop => stop_session(&app, true),
        state::CustomAction::Pause => pause_session(&app),
        state::CustomAction::Resume => resume_session(&app),
        state::CustomAction::Slot1 => select_slot(&app, 1, true),
        state::CustomAction::Slot2 => select_slot(&app, 2, true),
        state::CustomAction::Project => match gesture.project_id {
            Some(project_id) => start_session(&app, project_id, true),
            None => Err(AppError::key("gesture.custom_no_project")),
        },
    };

    let snapshot = broadcast_state(&app)?;
    Ok(match result {
        Ok(message) => GestureOutcome {
            accepted: true,
            gesture: None,
            gesture_label: gesture.name,
            message,
            snapshot,
        },
        Err(error) => GestureOutcome {
            accepted: false,
            gesture: None,
            gesture_label: gesture.name,
            message: error.to_string(),
            snapshot,
        },
    })
}

// ---------- Eingelernte Gesten ----------

#[derive(Debug, Serialize)]
pub struct GestureTraining {
    /// Merkmalssatz, zu dem die Aufnahmen passen.
    pub version: i64,
    pub samples: Vec<GestureSample>,
    /// Anzahl Aufnahmen je Geste, für die Anzeige.
    pub counts: Vec<(String, i64)>,
    /// Sind alle Gesten des Vokabulars ausreichend eingelernt?
    pub complete: bool,
}

/// Wie viele Aufnahmen eine Geste braucht, damit das Training taugt.
const MIN_SAMPLES: i64 = 8;

fn training(db: &Db) -> AppResult<GestureTraining> {
    let counts = db.gesture_sample_counts(state::FEATURE_VERSION)?;
    let complete = state::ALL_GESTURES.iter().all(|gesture| {
        counts
            .iter()
            .any(|(name, count)| name == gesture.key() && *count >= MIN_SAMPLES)
    });
    Ok(GestureTraining {
        version: state::FEATURE_VERSION,
        samples: db.gesture_samples(state::FEATURE_VERSION)?,
        counts,
        complete,
    })
}

/// Nimmt die Aufnahmen einer Geste entgegen - Grundgeste oder eigene Geste
/// (Kennung `custom:<id>`).
#[tauri::command]
pub fn record_gesture_samples(
    app: AppHandle,
    gesture: String,
    version: i64,
    samples: Vec<Vec<f64>>,
) -> AppResult<GestureTraining> {
    let key = resolve_sample_key(&app, &gesture)?;
    if version != state::FEATURE_VERSION {
        return Err(AppError::key("gesture.feature_mismatch"));
    }
    if samples.is_empty() {
        return Err(AppError::key("gesture.no_samples"));
    }
    if let Some(bad) = samples.iter().find(|sample| sample.len() != state::FEATURE_LEN) {
        return Err(AppError::args(
            "gesture.feature_count",
            &[("found", &bad.len()), ("expected", &state::FEATURE_LEN)],
        ));
    }

    let db = app.state::<Db>();
    db.add_gesture_samples(&key, version, &samples)?;
    training(&db)
}

/// Prüft die Kennung einer Aufnahme: entweder eine bekannte Grundgeste oder
/// eine tatsächlich vorhandene eigene Geste.
fn resolve_sample_key(app: &AppHandle, gesture: &str) -> AppResult<String> {
    if let Some(id) = state::parse_custom_sample_key(gesture) {
        let db = app.state::<Db>();
        db.custom_gesture(id)?
            .ok_or_else(|| AppError::key("gesture.custom_gone"))?;
        return Ok(state::custom_sample_key(id));
    }
    Ok(state::parse_gesture(gesture)?.key().to_string())
}

#[tauri::command]
pub fn get_gesture_training(db: State<'_, Db>) -> AppResult<GestureTraining> {
    training(&db)
}

/// Löscht die Aufnahmen einer Geste - oder alle, wenn keine genannt ist.
#[tauri::command]
pub fn clear_gesture_training(
    app: AppHandle,
    gesture: Option<String>,
) -> AppResult<GestureTraining> {
    let db = app.state::<Db>();
    match gesture {
        Some(name) => {
            let key = resolve_sample_key(&app, &name)?;
            db.clear_gesture_samples(Some(&key))?;
        }
        None => {
            db.clear_gesture_samples(None)?;
        }
    }

    // Ohne vollständiges Training wieder auf die geometrischen Regeln
    // zurückfallen - sonst würde die Erkennung stumm schlechter.
    let training = training(&db)?;
    if !training.complete {
        db.set_setting(state::KEY_USE_TRAINING, "0")?;
    }
    Ok(training)
}

// ---------- Einstellungen ----------

#[derive(Debug, Serialize)]
pub struct AppSettings {
    pub hotkey: String,
    /// Ist der Hotkey tatsächlich registriert?
    pub hotkey_active: bool,
    /// Grund, falls die Registrierung fehlgeschlagen ist.
    pub hotkey_error: Option<String>,
    pub confidence_threshold: f64,
    pub overlay_timeout_ms: i64,
    /// Eigenes Zeitfenster für Netzwerk-Kameras (mehr Latenz über WLAN).
    pub overlay_timeout_network_ms: i64,
    /// "builtin" oder "network".
    pub camera_source: String,
    pub camera_url: String,
    pub sound_cue: bool,
    pub slot_1_project_id: Option<i64>,
    pub slot_2_project_id: Option<i64>,
    pub active_slot: u8,
    /// Gelten die eingelernten Gesten?
    pub use_training: bool,
    /// Sprache der Oberfläche: "de" oder "en".
    pub language: String,
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppResult<AppSettings> {
    let db = app.state::<Db>();
    let read = |key: &str| db.setting(key).ok().flatten();

    // Sprache nachziehen: Die Oberfläche richtet sich nach diesem Wert, die
    // Meldungen des Backends nach dem Prozesszustand. Beides muss übereinstimmen,
    // sonst steht - wie beobachtet - ein englischer Statustext in einer deutschen
    // Oberfläche. Hier ist der Abgleich billig und passiert bei jedem Fensterstart.
    i18n::load_current(&db);
    let hotkey_status = app.state::<crate::hotkey::HotkeyStatus>().get();
    Ok(AppSettings {
        hotkey: state::hotkey(&db),
        hotkey_active: hotkey_status.active.is_some(),
        hotkey_error: hotkey_status.error,
        confidence_threshold: state::confidence_threshold(&db),
        overlay_timeout_ms: read(state::KEY_OVERLAY_TIMEOUT)
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(state::DEFAULT_OVERLAY_TIMEOUT_MS),
        overlay_timeout_network_ms: read(state::KEY_OVERLAY_TIMEOUT_NETWORK)
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(state::DEFAULT_OVERLAY_TIMEOUT_NETWORK_MS),
        camera_source: match read(state::KEY_CAMERA_SOURCE).as_deref() {
            Some(state::CAMERA_SOURCE_NETWORK) => state::CAMERA_SOURCE_NETWORK.to_string(),
            _ => state::CAMERA_SOURCE_BUILTIN.to_string(),
        },
        camera_url: read(state::KEY_CAMERA_URL).unwrap_or_default(),
        sound_cue: read(state::KEY_SOUND).as_deref() != Some("0"),
        slot_1_project_id: read(state::KEY_SLOT_1).and_then(|raw| raw.parse().ok()),
        slot_2_project_id: read(state::KEY_SLOT_2).and_then(|raw| raw.parse().ok()),
        active_slot: state::active_slot(&db),
        use_training: read(state::KEY_USE_TRAINING).as_deref() == Some("1"),
        language: i18n::lang(&db).code().to_string(),
    })
}

#[tauri::command]
pub fn set_setting(app: AppHandle, key: String, value: String) -> AppResult<AppSettings> {
    if key == state::KEY_CAMERA_URL {
        state::validate_camera_url(&value)?;
    }
    // Der Hotkey wird erst gespeichert, wenn er sich auch registrieren lässt -
    // sonst stünde in der Datenbank eine Kombination, die es nicht gibt.
    if key == state::KEY_HOTKEY {
        crate::hotkey::apply(&app, &value)?;
    }
    if key == state::KEY_LANGUAGE {
        i18n::set_current(i18n::Lang::parse(&value));
    }
    {
        let db = app.state::<Db>();
        db.set_setting(&key, value.trim())?;
    }
    broadcast_state(&app)?;
    get_settings(app)
}

// ---------- Fenster ----------

#[tauri::command]
pub fn close_overlay(app: AppHandle) -> AppResult<()> {
    overlay::hide(&app)
}

#[tauri::command]
pub fn open_overlay(app: AppHandle) -> AppResult<()> {
    overlay::show(&app)
}

#[tauri::command]
pub fn open_main_window(app: AppHandle) -> AppResult<()> {
    let _ = crate::panel::hide(&app);
    overlay::show_main(&app)
}

/// Schließt das Fenster in der Menüleiste - etwa nach einer Aktion.
#[tauri::command]
pub fn close_panel(app: AppHandle) -> AppResult<()> {
    crate::panel::hide(&app)
}

/// Passt die Fensterhöhe an den Inhalt an.
#[tauri::command]
pub fn resize_panel(app: AppHandle, height: f64) -> AppResult<()> {
    crate::panel::resize(&app, height)
}

// ---------- Netzwerk-Kamera ----------

/// Liefert das zuletzt empfangene Einzelbild als Rohdaten (JPEG). Leer, solange
/// noch kein vollständiges Bild angekommen ist.
#[tauri::command]
pub fn camera_frame(app: AppHandle) -> tauri::ipc::Response {
    let frame = app.state::<NetworkCamera>().frame().unwrap_or_default();
    tauri::ipc::Response::new(frame)
}

/// Letzter Verbindungsfehler der Netzwerk-Kamera, für die Anzeige im Overlay.
#[tauri::command]
pub fn camera_error(app: AppHandle) -> Option<String> {
    app.state::<NetworkCamera>().error()
}

/// Einmaliger Verbindungstest aus den Einstellungen heraus.
///
/// Läuft absichtlich gegen die übergebene Adresse und nicht gegen die
/// gespeicherte: so lässt sich eine Adresse prüfen, bevor man sie übernimmt.
#[tauri::command]
pub fn test_camera_url(url: String) -> AppResult<String> {
    state::validate_camera_url(&url)?;
    if url.trim().is_empty() {
        return Err(AppError::key("camera.url_missing"));
    }
    crate::camera::probe(url.trim(), i18n::current()).map_err(AppError::msg)
}

/// Hält die Netzwerk-Kamera für die Vorschau im Hauptfenster offen.
#[tauri::command]
pub fn set_camera_preview(app: AppHandle, active: bool) -> AppResult<()> {
    let camera = app.state::<NetworkCamera>();
    camera.set_hold(active);
    if active {
        overlay::start_network_camera(&app);
    } else if !overlay::is_visible(&app) {
        camera.stop();
    }
    Ok(())
}
