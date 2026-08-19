//! Das Kamera-Overlay: ein kleines, randloses Fenster oben rechts, das nur
//! während der Erkennung sichtbar ist.
//!
//! Wichtig für das Datenschutzversprechen: Das Fenster wird nicht dauerhaft
//! offengehalten. Kein sichtbares Overlay heißt kein Kamerastream - das
//! Frontend stoppt die Tracks beim Schließen.

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::camera::NetworkCamera;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::screens;
use crate::state;

pub const OVERLAY_LABEL: &str = "overlay";
pub const MAIN_LABEL: &str = "main";

/// Abstand zum Bildschirmrand in logischen Pixeln (oben zusätzlich Platz für
/// Menüleiste bzw. Systemleiste).
const MARGIN: f64 = 24.0;
const TOP_MARGIN: f64 = 48.0;

fn overlay_window(app: &AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| AppError::key("window.overlay_missing"))
}

/// Positioniert das Overlay oben rechts - auf dem Bildschirm, auf dem der
/// Nutzer gerade arbeitet, nicht dort, wo das Fenster zuletzt stand.
fn position_top_right(app: &AppHandle, window: &WebviewWindow) -> AppResult<()> {
    let Some(monitor) = screens::target_monitor(app, window)? else {
        return Ok(());
    };
    let screen = screens::monitor_rect(&monitor);
    let (width, _) = screens::window_size(window)?;

    let x = screen.right() - width - MARGIN;
    let y = screen.top + TOP_MARGIN;

    if cfg!(debug_assertions) {
        eprintln!("[overlay] Bildschirm {screen:?} -> Fenster ({x},{y}) Breite {width}");
    }

    screens::move_to(window, x, y)
}

/// Öffnet das Overlay und weist das Frontend an, die Kamera zu starten.
pub fn show(app: &AppHandle) -> AppResult<()> {
    let window = overlay_window(app)?;
    position_top_right(app, &window)?;
    window.set_always_on_top(true)?;
    window.show()?;
    // Bewusst kein set_focus(): der Nutzer soll in seinem Arbeitsfenster bleiben.
    start_network_camera(app);
    app.emit_to(OVERLAY_LABEL, "overlay:open", ())?;
    Ok(())
}

/// Schließt das Overlay und stoppt damit die Kamera.
pub fn hide(app: &AppHandle) -> AppResult<()> {
    let window = overlay_window(app)?;
    app.emit_to(OVERLAY_LABEL, "overlay:close", ())?;
    window.hide()?;
    app.state::<NetworkCamera>().stop_unless_held();
    Ok(())
}

/// Startet den Abruf der Netzwerk-Kamera, sofern sie als Quelle eingestellt
/// ist. Bei der eingebauten Webcam passiert hier nichts - der Standardweg
/// bleibt vollständig ohne Netzwerk.
pub fn start_network_camera(app: &AppHandle) {
    let Some(url) = configured_camera_url(app) else {
        return;
    };
    let lang = crate::i18n::lang(&app.state::<Db>());
    app.state::<NetworkCamera>().ensure_started(url, lang);
}

/// Adresse der Netzwerk-Kamera, falls sie als Quelle eingestellt ist.
pub fn configured_camera_url(app: &AppHandle) -> Option<String> {
    let db = app.state::<Db>();
    if db.setting(state::KEY_CAMERA_SOURCE).ok().flatten().as_deref()
        != Some(state::CAMERA_SOURCE_NETWORK)
    {
        return None;
    }
    db.setting(state::KEY_CAMERA_URL)
        .ok()
        .flatten()
        .filter(|url| !url.trim().is_empty())
}

pub fn is_visible(app: &AppHandle) -> bool {
    overlay_window(app)
        .and_then(|window| window.is_visible().map_err(Into::into))
        .unwrap_or(false)
}

/// Hotkey-Verhalten: sichtbares Overlay wird wieder geschlossen (Abbruch),
/// sonst geöffnet.
pub fn toggle(app: &AppHandle) -> AppResult<()> {
    if is_visible(app) {
        hide(app)
    } else {
        show(app)
    }
}

/// Holt das Hauptfenster nach vorne - hier ist ein Fokuswechsel gewollt.
pub fn show_main(app: &AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| AppError::key("window.main_missing"))?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}
