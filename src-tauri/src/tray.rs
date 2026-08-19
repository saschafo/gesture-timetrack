//! Tray-Icon: Status auf einen Blick, Grundfunktionen ohne offenes Fenster.

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::commands;
use crate::db::Db;
use crate::error::AppResult;
use crate::i18n;
use crate::overlay;
use crate::panel::{self, Anchor};
use crate::state::{format_hms, Snapshot, TrackerStatus};

pub const TRAY_ID: &str = "gesture-timetrack";

/// Kennung eines Projekt-Eintrags im Menü.
const PROJECT_PREFIX: &str = "project:";
/// Länge, ab der der Projektname in der Menüleiste gekürzt wird.
const TITLE_NAME_LIMIT: usize = 16;

/// Projektliste als Untermenü: startet ein Projekt bzw. wechselt dorthin.
fn project_menu<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &Snapshot,
) -> tauri::Result<Submenu<R>> {
    let title = i18n::msg(if snapshot.status == TrackerStatus::Idle {
        "tray.start_project"
    } else {
        "tray.switch_project"
    });
    let submenu = Submenu::with_id(app, "projects", title, true)?;

    // Die Liste kommt direkt aus der Datenbank: das Tray soll auch ohne
    // offenes Fenster stimmen.
    let projects = app
        .try_state::<Db>()
        .and_then(|db| db.projects(true).ok())
        .unwrap_or_default();

    if projects.is_empty() {
        submenu.append(&MenuItem::with_id(
            app,
            "no-projects",
            i18n::msg("tray.no_projects"),
            false,
            None::<&str>,
        )?)?;
        return Ok(submenu);
    }

    for project in projects {
        let running = snapshot.project_id == Some(project.id);
        let label = match (running, snapshot.status) {
            (true, TrackerStatus::Paused) => {
                i18n::msg_args("tray.project_paused", &[("name".into(), project.name.clone())])
            }
            (true, _) => {
                i18n::msg_args("tray.project_running", &[("name".into(), project.name.clone())])
            }
            _ => project.name.clone(),
        };
        submenu.append(&MenuItem::with_id(
            app,
            format!("{PROJECT_PREFIX}{}", project.id),
            label,
            !running,
            None::<&str>,
        )?)?;
    }
    Ok(submenu)
}

fn build_menu<R: Runtime>(app: &AppHandle<R>, snapshot: &Snapshot) -> tauri::Result<Menu<R>> {
    let status_line = match snapshot.status {
        TrackerStatus::Idle => i18n::msg("tray.stopped"),
        TrackerStatus::Running => format!(
            "{} · {}",
            snapshot.project_name.clone().unwrap_or_default(),
            format_hms(snapshot.elapsed_seconds)
        ),
        TrackerStatus::Paused => i18n::msg_args(
            "tray.break",
            &[
                ("name".into(), snapshot.project_name.clone().unwrap_or_default()),
                ("time".into(), format_hms(snapshot.elapsed_seconds)),
            ],
        ),
    };

    let status = MenuItem::with_id(app, "status", status_line, false, None::<&str>)?;
    let today = MenuItem::with_id(
        app,
        "today",
        i18n::msg_args(
            "tray.today",
            &[("time".into(), format_hms(snapshot.today_seconds))],
        ),
        false,
        None::<&str>,
    )?;

    let projects = project_menu(app, snapshot)?;

    let stop = MenuItem::with_id(
        app,
        "stop",
        i18n::msg("tray.stop"),
        snapshot.status != TrackerStatus::Idle,
        None::<&str>,
    )?;

    let pause_label = i18n::msg(if snapshot.status == TrackerStatus::Paused {
        "tray.resume"
    } else {
        "tray.pause"
    });
    let pause = MenuItem::with_id(
        app,
        "pause",
        pause_label,
        snapshot.status != TrackerStatus::Idle,
        None::<&str>,
    )?;

    let overlay_item =
        MenuItem::with_id(app, "overlay", i18n::msg("tray.gesture"), true, None::<&str>)?;
    let main = MenuItem::with_id(app, "main", i18n::msg("tray.window"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", i18n::msg("tray.quit"), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &status,
            &today,
            &separator,
            &projects,
            &pause,
            &stop,
            &separator,
            &overlay_item,
            &main,
            &separator,
            &quit,
        ],
    )
}

pub fn init(app: &AppHandle, snapshot: &Snapshot) -> AppResult<()> {
    let menu = build_menu(app, snapshot)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(tooltip(snapshot))
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                // Kleines Fenster direkt unter dem Symbol - Start, Pause,
                // Stopp und Projektwechsel ohne Hauptfenster.
                let anchor = Anchor {
                    position: rect.position,
                    size: rect.size,
                };
                if let Err(error) = panel::toggle(tray.app_handle(), Some(anchor)) {
                    eprintln!("[tray] Menüleisten-Fenster: {error}");
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn tooltip(snapshot: &Snapshot) -> String {
    match snapshot.status {
        TrackerStatus::Idle => i18n::msg_args(
            "tray.tooltip_idle",
            &[("today".into(), format_hms(snapshot.today_seconds))],
        ),
        _ => i18n::msg_args(
            "tray.tooltip_active",
            &[
                ("name".into(), snapshot.project_name.clone().unwrap_or_default()),
                ("status".into(), snapshot.status_label.clone()),
                ("running".into(), format_hms(snapshot.elapsed_seconds)),
                ("today".into(), format_hms(snapshot.today_seconds)),
            ],
        ),
    }
}

/// Text neben dem Icon in der Menüleiste: Projekt und laufende Zeit.
#[cfg(target_os = "macos")]
fn title(snapshot: &Snapshot) -> Option<String> {
    if snapshot.status == TrackerStatus::Idle {
        return None;
    }
    let clock = format_hms(snapshot.elapsed_seconds);
    let mark = if snapshot.status == TrackerStatus::Paused {
        "‖ "
    } else {
        ""
    };
    Some(match &snapshot.project_name {
        Some(name) => format!("{mark}{} · {clock}", shorten(name)),
        None => format!("{mark}{clock}"),
    })
}

/// Kürzt lange Projektnamen, damit die Menüleiste nicht überläuft.
#[cfg(target_os = "macos")]
fn shorten(name: &str) -> String {
    if name.chars().count() <= TITLE_NAME_LIMIT {
        return name.to_string();
    }
    let cut: String = name.chars().take(TITLE_NAME_LIMIT - 1).collect();
    format!("{}…", cut.trim_end())
}

/// Nur Uhrzeit und Kurzinfo nachziehen - günstig genug für jede Sekunde.
pub fn set_clock(app: &AppHandle, snapshot: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Err(error) = tray.set_tooltip(Some(tooltip(snapshot))) {
        eprintln!("[tray] Tooltip: {error}");
    }
    #[cfg(target_os = "macos")]
    if let Err(error) = tray.set_title(title(snapshot)) {
        eprintln!("[tray] Titel: {error}");
    }
}

/// Übernimmt einen neuen Zustand vollständig ins Tray, inklusive Menü. Fehler
/// werden geloggt, aber nicht weitergereicht - ein hängendes Tray darf die
/// Erfassung nicht blockieren.
pub fn refresh(app: &AppHandle, snapshot: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    set_clock(app, snapshot);
    match build_menu(app, snapshot) {
        Ok(menu) => {
            if let Err(error) = tray.set_menu(Some(menu)) {
                eprintln!("[tray] Menü: {error}");
            }
        }
        Err(error) => eprintln!("[tray] Menü: {error}"),
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let app = app.clone();
    let id = event.id().as_ref().to_string();

    // Projekt starten bzw. dorthin wechseln - dieselbe Funktion wie im Fenster.
    if let Some(raw) = id.strip_prefix(PROJECT_PREFIX) {
        if let Ok(project_id) = raw.parse::<i64>() {
            if let Err(error) = commands::start_tracking(app, project_id) {
                eprintln!("[tray] {error}");
            }
        }
        return;
    }

    let result = match id.as_str() {
        "stop" => commands::stop_tracking(app.clone()).map(|_| ()),
        "pause" => toggle_pause(&app),
        "overlay" => overlay::show(&app),
        "main" => overlay::show_main(&app),
        "quit" => {
            // Laufende Erfassung sauber beenden, damit nichts verloren geht.
            let _ = commands::stop_tracking(app.clone());
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        eprintln!("[tray] {error}");
    }
}

fn toggle_pause(app: &AppHandle) -> AppResult<()> {
    let tracker = app.state::<crate::state::Tracker>();
    match tracker.status() {
        TrackerStatus::Paused => commands::resume_tracking(app.clone())?,
        TrackerStatus::Running => commands::pause_tracking(app.clone())?,
        TrackerStatus::Idle => return Ok(()),
    };
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::state::{SlotInfo, TrackerStatus};

    fn snapshot(status: TrackerStatus, project: Option<&str>, elapsed: i64) -> Snapshot {
        Snapshot {
            status,
            status_label: status.label().to_string(),
            project_id: project.map(|_| 1),
            project_name: project.map(|name| name.to_string()),
            project_color: None,
            active_slot: Some(1),
            elapsed_seconds: elapsed,
            pause_seconds: 0,
            today_seconds: 0,
            slots: Vec::<SlotInfo>::new(),
        }
    }

    #[test]
    fn menueleiste_zeigt_projekt_und_zeit() {
        let running = snapshot(TrackerStatus::Running, Some("Kunde Meier"), 3661);
        assert_eq!(title(&running).unwrap(), "Kunde Meier · 01:01:01");

        let paused = snapshot(TrackerStatus::Paused, Some("Kunde Meier"), 60);
        assert!(title(&paused).unwrap().starts_with("‖ "));

        // Ohne laufende Erfassung bleibt die Menüleiste frei.
        assert!(title(&snapshot(TrackerStatus::Idle, None, 0)).is_none());
    }

    #[test]
    fn lange_projektnamen_werden_gekuerzt() {
        assert_eq!(shorten("Kunde Meier"), "Kunde Meier");
        let long = shorten("Kunde Mustermann IOT Projekt");
        assert!(long.chars().count() <= TITLE_NAME_LIMIT, "{long}");
        assert!(long.ends_with('…'), "{long}");
    }
}
