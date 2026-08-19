//! Fensterplatzierung über mehrere Bildschirme.
//!
//! Der unangenehme Teil sind die Koordinatensysteme. Unter macOS liefert Tauri
//! Mauszeiger und die Lage des Menüleisten-Symbols in Pixeln des **Haupt**-
//! bildschirms (also Punkte × dessen Skalierung), die Position der Bildschirme
//! selbst dagegen in Punkten. Beides zu mischen führt bei einem Retina- und
//! einem gewöhnlichen Bildschirm dazu, dass Fenster auf dem falschen Monitor
//! aufgehen - genau der beobachtete Fehler.
//!
//! Deshalb rechnet dieses Modul in einer einheitlichen "Arbeitseinheit":
//!
//! * macOS: Punkte des globalen Bildschirmsystems,
//! * sonst: physische Pixel.
//!
//! Alles, was hier hinein- und herausgeht, ist in dieser Einheit - und nur beim
//! Setzen der Fensterposition wird sie wieder plattformgerecht übergeben.

use tauri::{AppHandle, Monitor, Position, Size, WebviewWindow};

use crate::error::AppResult;

/// Rechteck in Arbeitseinheiten.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.left && x < self.right() && y >= self.top && y < self.bottom()
    }
}

/// Skalierung des Hauptbildschirms - Bezugsgröße der Systemangaben unter macOS.
#[cfg(target_os = "macos")]
fn primary_scale(window: &WebviewWindow) -> f64 {
    window
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor.scale_factor())
        .unwrap_or(1.0)
}

/// Bildschirmfläche in Arbeitseinheiten.
pub fn monitor_rect(monitor: &Monitor) -> Rect {
    let origin = monitor.position();
    let size = monitor.size();

    #[cfg(target_os = "macos")]
    let (width, height) = {
        // Die Größe kommt physisch, die Position in Punkten - umrechnen.
        let scale = monitor.scale_factor();
        (size.width as f64 / scale, size.height as f64 / scale)
    };
    #[cfg(not(target_os = "macos"))]
    let (width, height) = (size.width as f64, size.height as f64);

    Rect {
        left: origin.x as f64,
        top: origin.y as f64,
        width,
        height,
    }
}

/// Fenstergröße in Arbeitseinheiten.
pub fn window_size(window: &WebviewWindow) -> AppResult<(f64, f64)> {
    let outer = window.outer_size()?;

    #[cfg(target_os = "macos")]
    {
        let scale = window.scale_factor()?;
        Ok((outer.width as f64 / scale, outer.height as f64 / scale))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok((outer.width as f64, outer.height as f64))
    }
}

/// Mauszeiger in Arbeitseinheiten.
pub fn cursor(app: &AppHandle, window: &WebviewWindow) -> Option<(f64, f64)> {
    let position = app.cursor_position().ok()?;

    #[cfg(target_os = "macos")]
    {
        let scale = primary_scale(window);
        Some((position.x / scale, position.y / scale))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        Some((position.x, position.y))
    }
}

/// Lage des Menüleisten- bzw. Taskleisten-Symbols in Arbeitseinheiten.
pub fn tray_rect(window: &WebviewWindow, position: Position, size: Size) -> Rect {
    #[cfg(target_os = "macos")]
    let scale = primary_scale(window);
    #[cfg(not(target_os = "macos"))]
    let scale = window.scale_factor().unwrap_or(1.0);

    let physical_position = position.to_physical::<f64>(scale);
    let physical_size = size.to_physical::<f64>(scale);

    #[cfg(target_os = "macos")]
    let divisor = scale;
    #[cfg(not(target_os = "macos"))]
    let divisor = 1.0;

    Rect {
        left: physical_position.x / divisor,
        top: physical_position.y / divisor,
        width: physical_size.width / divisor,
        height: physical_size.height / divisor,
    }
}

/// Bildschirm, der den Punkt enthält.
///
/// Bewusst selbst gesucht statt `monitor_from_point`: die Systemfunktion
/// erwartet physische Pixel und hilft in der gemischten Welt oben nicht weiter.
pub fn monitor_at(window: &WebviewWindow, x: f64, y: f64) -> AppResult<Option<Monitor>> {
    for monitor in window.available_monitors()? {
        if monitor_rect(&monitor).contains(x, y) {
            return Ok(Some(monitor));
        }
    }
    Ok(None)
}

/// Bildschirm, auf dem der Nutzer gerade arbeitet: der unter dem Mauszeiger,
/// sonst der des Fensters, sonst der Hauptbildschirm.
pub fn target_monitor(app: &AppHandle, window: &WebviewWindow) -> AppResult<Option<Monitor>> {
    if let Some((x, y)) = cursor(app, window) {
        if let Some(monitor) = monitor_at(window, x, y)? {
            return Ok(Some(monitor));
        }
    }
    if let Some(monitor) = window.current_monitor()? {
        return Ok(Some(monitor));
    }
    Ok(window.primary_monitor()?)
}

/// Setzt die Fensterposition aus Arbeitseinheiten.
pub fn move_to(window: &WebviewWindow, x: f64, y: f64) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        // Punkte: Tauri rechnet sie mit der Skalierung des Fensters um und
        // landet damit wieder im globalen Punktesystem.
        window.set_position(tauri::LogicalPosition::new(x, y))?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_position(tauri::PhysicalPosition::new(x, y))?;
    }
    Ok(())
}

/// Aktuelle Fensterposition in Arbeitseinheiten.
pub fn window_position(window: &WebviewWindow) -> AppResult<(f64, f64)> {
    let position = window.outer_position()?;

    #[cfg(target_os = "macos")]
    {
        let scale = window.scale_factor()?;
        Ok((position.x as f64 / scale, position.y as f64 / scale))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok((position.x as f64, position.y as f64))
    }
}
