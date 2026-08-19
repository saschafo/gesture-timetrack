//! Gesture TimeTrack - Zeiterfassung per Handgeste, vollständig offline.
//!
//! Aufbau: Das Rust-Backend hält Zustand, Datenbank, Hotkey und Tray. Das
//! Frontend zeigt an und erkennt Gesten; die Kamerabilder verlassen dabei nie
//! den Webview-Prozess - es gibt weder eine Netzwerkschnittstelle noch einen
//! Pfad, auf dem Bildmaterial gespeichert würde.

mod camera;
mod commands;
mod db;
mod error;
mod i18n;
mod export;
mod hotkey;
mod overlay;
mod panel;
mod screens;
mod state;
mod tray;

use std::time::Duration;

use tauri::{Manager, WindowEvent};

use camera::NetworkCamera;
use db::Db;
use hotkey::HotkeyStatus;
use state::{Tracker, TrackerStatus};

/// Takt der Uhr in der Menüleiste, während eine Erfassung läuft.
const TICK: Duration = Duration::from_secs(1);
/// Wie viele Takte zwischen zwei vollständigen Aktualisierungen liegen.
/// Menü und Fenster brauchen das nicht jede Sekunde - die Fenster zählen
/// selbst weiter, das Menü ändert sich nur bei Zustandswechseln.
const FULL_REFRESH_EVERY: u32 = 30;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Aktualisierung: Das Plugin prüft **nichts** von selbst - der Aufruf
        // kommt ausschließlich aus der Oberfläche, auf Knopfdruck.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| hotkey::on_shortcut(app, shortcut, event))
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::apply_gesture,
            commands::start_tracking,
            commands::stop_tracking,
            commands::pause_tracking,
            commands::resume_tracking,
            commands::set_slot,
            commands::get_state,
            commands::get_projects,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::day_totals,
            commands::list_entries,
            commands::create_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::export_csv,
            commands::get_settings,
            commands::set_setting,
            commands::close_overlay,
            commands::open_overlay,
            commands::open_main_window,
            commands::close_panel,
            commands::resize_panel,
            commands::camera_frame,
            commands::camera_error,
            commands::test_camera_url,
            commands::set_camera_preview,
            commands::record_gesture_samples,
            commands::custom_gesture_actions,
            commands::get_custom_gestures,
            commands::create_custom_gesture,
            commands::update_custom_gesture,
            commands::delete_custom_gesture,
            commands::apply_custom_gesture,
            commands::get_gesture_training,
            commands::clear_gesture_training,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let db_path = app
                .path()
                .app_data_dir()?
                .join("gesture-timetrack.sqlite3");
            let db = Db::open(&db_path)?;
            for (key, value) in state::settings_defaults() {
                if db.setting(key)?.is_none() {
                    db.set_setting(key, &value)?;
                }
            }
            // Sprache vor allem anderen setzen: Tray und Meldungen richten sich
            // danach.
            i18n::load_current(&db);
            if cfg!(debug_assertions) {
                eprintln!("[setup] Sprache: {}", i18n::current().code());
            }

            // Aufnahmen zu Gesten aufräumen, die es nicht mehr gibt.
            let known: Vec<&str> = state::ALL_GESTURES.iter().map(|g| g.key()).collect();
            match db.prune_gesture_samples(&known) {
                Ok(removed) if removed > 0 => {
                    eprintln!("[setup] {removed} veraltete Gestenaufnahmen entfernt");
                }
                Err(error) => eprintln!("[setup] Aufräumen fehlgeschlagen: {error}"),
                _ => {}
            }

            app.manage(db);
            app.manage(Tracker::default());
            app.manage(NetworkCamera::default());
            app.manage(HotkeyStatus::default());

            state::recover_session(&app.state::<Db>(), &app.state::<Tracker>())?;

            let snapshot = state::snapshot(&app.state::<Db>(), &app.state::<Tracker>())?;
            tray::init(&handle, &snapshot)?;

            if let Err(error) = hotkey::register_from_settings(&handle) {
                // Kein harter Fehler: die App ist auch ohne Hotkey bedienbar,
                // und die Einstellungen zeigen den Grund an.
                eprintln!("[setup] {error}");
            }

            // Im Entwicklungsmodus die erkannte Bildschirmanordnung ausgeben -
            // Platzierungsfehler bei mehreren Monitoren sind ohne diese Zahlen
            // kaum zu finden.
            if cfg!(debug_assertions) {
                if let Some(window) = handle.get_webview_window(overlay::MAIN_LABEL) {
                    for monitor in window.available_monitors().unwrap_or_default() {
                        eprintln!(
                            "[screens] {:?} Faktor {} -> {:?}",
                            monitor.name(),
                            monitor.scale_factor(),
                            screens::monitor_rect(&monitor)
                        );
                    }
                }
            }

            let ticker = handle.clone();
            std::thread::spawn(move || {
                let mut ticks: u32 = 0;
                loop {
                    std::thread::sleep(TICK);
                    if ticker.state::<Tracker>().status() == TrackerStatus::Idle {
                        continue;
                    }
                    ticks = ticks.wrapping_add(1);

                    let result = if ticks % FULL_REFRESH_EVERY == 0 {
                        commands::broadcast_state(&ticker).map(|_| ())
                    } else {
                        commands::refresh_tray_clock(&ticker)
                    };
                    if let Err(error) = result {
                        eprintln!("[tick] {error}");
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // Die App lebt im Tray weiter, damit der Hotkey erreichbar
                // bleibt. Zurück ins Fenster führen drei Wege: Klick auf das
                // Symbol in der Menüleiste, dessen Menüeintrag „Fenster öffnen"
                // und - siehe unten - das Dock-Symbol.
                if window.label() == overlay::MAIN_LABEL {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // Das Menüleisten-Fenster verhält sich wie ein Menü: Klick daneben
            // schließt es.
            WindowEvent::Focused(false) if window.label() == panel::PANEL_LABEL => {
                let _ = window.hide();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("Anwendung konnte nicht gestartet werden")
        .run(|app, event| {
            // Klick auf das Symbol im Dock, während kein Fenster sichtbar ist.
            // Ohne diese Behandlung wirkt die App nach dem Schließen des
            // Fensters wie verschwunden - sie läuft ja weiter, nur unsichtbar.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = event
            {
                if let Err(error) = overlay::show_main(app) {
                    eprintln!("[dock] Hauptfenster: {error}");
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                let _ = (app, event);
            }
        });
}
