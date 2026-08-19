//! Kleines Fenster direkt unter dem Symbol in der Menüleiste.
//!
//! Zweck: Start, Pause, Stopp und Projektwechsel ohne das Hauptfenster zu
//! öffnen - der schnelle Weg, wenn die Hand gerade nicht vor der Kamera ist.
//!
//! Verhalten wie ein Menü: erscheint beim Klick auf das Symbol, verschwindet,
//! sobald es den Fokus verliert.

use tauri::{AppHandle, LogicalSize, Manager, Position, Size, WebviewWindow};

use crate::error::{AppError, AppResult};
use crate::screens;

pub const PANEL_LABEL: &str = "panel";

/// Abstand zwischen Menüleiste und Fenster.
const GAP: f64 = 6.0;
/// Sicherheitsabstand zum Bildschirmrand.
const EDGE: f64 = 8.0;
/// Ersatzhöhe der Menüleiste, falls die Lage des Symbols nicht zum Bildschirm
/// unter dem Mauszeiger passt (etwa auf einem zweiten Bildschirm ohne eigene
/// Menüleiste).
const MENUBAR_FALLBACK: f64 = 26.0;
/// Weiter unten als das kann ein Symbol in der Menüleiste nicht liegen -
/// darüber hinaus gilt die gemeldete Lage als unbrauchbar.
const MENUBAR_LIMIT: f64 = 120.0;

fn panel_window(app: &AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window(PANEL_LABEL)
        .ok_or_else(|| AppError::key("window.panel_missing"))
}

pub fn is_visible(app: &AppHandle) -> bool {
    panel_window(app)
        .and_then(|window| window.is_visible().map_err(Into::into))
        .unwrap_or(false)
}

/// Umschalten - genau wie bei einem Menü, das ein zweiter Klick wieder zuklappt.
pub fn toggle(app: &AppHandle, anchor: Option<Anchor>) -> AppResult<()> {
    if is_visible(app) {
        hide(app)
    } else {
        show(app, anchor)
    }
}

/// Lage des Menüleisten-Symbols, wie sie das Tray-Ereignis liefert. Ob die
/// Werte logisch oder physisch sind, entscheidet die Plattform - deshalb wird
/// hier erst beim Positionieren umgerechnet.
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub position: Position,
    pub size: Size,
}

pub fn show(app: &AppHandle, anchor: Option<Anchor>) -> AppResult<()> {
    let window = panel_window(app)?;
    place(app, &window, anchor)?;
    window.show()?;
    window.set_always_on_top(true)?;
    // Fokus ist hier gewollt: nur mit Fokus lässt sich das Fenster wieder
    // schließen, sobald der Nutzer woanders hinklickt.
    window.set_focus()?;
    Ok(())
}

pub fn hide(app: &AppHandle) -> AppResult<()> {
    panel_window(app)?.hide()?;
    Ok(())
}

/// Setzt das Fenster mittig unter das Symbol in der Menüleiste.
///
/// Maßgeblich ist der Bildschirm unter dem Mauszeiger - dort wurde geklickt.
/// Die gemeldete Lage des Symbols wird nur verwendet, wenn sie zu diesem
/// Bildschirm passt; sonst entscheidet der Mauszeiger und die Menüleistenhöhe.
fn place(app: &AppHandle, window: &WebviewWindow, anchor: Option<Anchor>) -> AppResult<()> {
    let Some(monitor) = screens::target_monitor(app, window)? else {
        return Ok(());
    };
    let screen = screens::monitor_rect(&monitor);
    let (width, height) = screens::window_size(window)?;
    let cursor = screens::cursor(app, window);

    let mut x = cursor.map(|(x, _)| x).unwrap_or(screen.left + screen.width / 2.0) - width / 2.0;
    let mut y = screen.top + MENUBAR_FALLBACK + GAP;

    if let Some(anchor) = anchor {
        let icon = screens::tray_rect(window, anchor.position, anchor.size);
        let center = icon.left + icon.width / 2.0;

        if center >= screen.left && center <= screen.right() {
            x = center - width / 2.0;
        }
        // Nur übernehmen, wenn das Symbol tatsächlich am oberen Rand dieses
        // Bildschirms sitzt - bei mehreren Bildschirmen sind die Werte sonst
        // aus einem fremden Bereich.
        if icon.top >= screen.top - 4.0 && icon.top <= screen.top + MENUBAR_LIMIT {
            y = icon.bottom() + GAP;
        }

        if cfg!(debug_assertions) {
            eprintln!("[panel] Symbol {icon:?}");
        }
    }

    let min_x = screen.left + EDGE;
    let max_x = (screen.right() - width - EDGE).max(min_x);
    let max_y = (screen.bottom() - height - EDGE).max(screen.top + EDGE);
    let x = x.clamp(min_x, max_x);
    let y = y.min(max_y);

    if cfg!(debug_assertions) {
        eprintln!(
            "[panel] Bildschirm {screen:?}, Zeiger {cursor:?} -> Fenster ({x},{y}) {width}×{height}"
        );
    }

    screens::move_to(window, x, y)
}

/// Passt die Höhe an den Inhalt an - die Projektliste wächst mit.
pub fn resize(app: &AppHandle, height: f64) -> AppResult<()> {
    let window = panel_window(app)?;
    let current = window.outer_size()?;
    let scale = window.scale_factor()?;
    let width = current.width as f64 / scale;
    window.set_size(LogicalSize::new(width, height.clamp(160.0, 620.0)))?;

    // Nach dem Wachsen könnte das Fenster unten aus dem Bild ragen.
    keep_on_screen(app, &window)?;
    Ok(())
}

/// Schiebt das Fenster zurück in den sichtbaren Bereich seines Bildschirms.
fn keep_on_screen(app: &AppHandle, window: &WebviewWindow) -> AppResult<()> {
    let Some(monitor) = screens::target_monitor(app, window)? else {
        return Ok(());
    };
    let screen = screens::monitor_rect(&monitor);
    let (width, height) = screens::window_size(window)?;
    let (x, y) = screens::window_position(window)?;

    let min_x = screen.left + EDGE;
    let max_x = (screen.right() - width - EDGE).max(min_x);
    let max_y = (screen.bottom() - height - EDGE).max(screen.top + EDGE);

    screens::move_to(window, x.clamp(min_x, max_x), y.min(max_y))
}
