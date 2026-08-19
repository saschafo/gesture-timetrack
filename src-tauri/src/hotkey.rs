//! Systemweiter Hotkey. Er ist der einzige Weg, die Kamera zu aktivieren -
//! ohne Tastendruck läuft keine Erkennung.
//!
//! Zwei Fallstricke, die die Oberfläche sichtbar machen muss:
//!
//! * Ist eine Kombination von einer anderen Anwendung belegt, schlägt die
//!   Registrierung fehl - das melden wir als Fehler zurück.
//! * Ist sie vom **Betriebssystem** belegt (auf macOS z. B. ⌘⌥Leertaste für die
//!   Finder-Suche), gelingt die Registrierung, aber das System verbraucht den
//!   Tastendruck vorher. Deshalb meldet das Backend jedes Auslösen als Ereignis:
//!   nur so kann der Nutzer erkennen, ob seine Kombination wirklich ankommt.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::overlay;
use crate::state;

/// Ereignis an das Hauptfenster, sobald der Hotkey wirklich angekommen ist.
pub const EVENT_FIRED: &str = "hotkey:fired";

#[derive(Debug, Clone, Default, Serialize)]
pub struct Status {
    /// Tatsächlich registrierte Kombination.
    pub active: Option<String>,
    /// Grund, falls die letzte Registrierung fehlgeschlagen ist.
    pub error: Option<String>,
}

#[derive(Default)]
pub struct HotkeyStatus(Mutex<Status>);

impl HotkeyStatus {
    pub fn get(&self) -> Status {
        self.0.lock().expect("Hotkey-Mutex vergiftet").clone()
    }

    fn set(&self, status: Status) {
        *self.0.lock().expect("Hotkey-Mutex vergiftet") = status;
    }
}

fn parse(combination: &str) -> AppResult<Shortcut> {
    combination
        .trim()
        .parse::<Shortcut>()
        .map_err(|_| AppError::args("hotkey.invalid", &[("value", &combination)]))
}

/// Registriert die in den Einstellungen hinterlegte Kombination.
pub fn register_from_settings(app: &AppHandle) -> AppResult<()> {
    let combination = {
        let db = app.state::<Db>();
        state::hotkey(&db)
    };
    apply(app, &combination)
}

/// Schaltet auf `combination` um.
///
/// Scheitert die Registrierung, wird die vorher aktive Kombination
/// wiederhergestellt - der Nutzer bleibt also nie ohne Hotkey zurück.
pub fn apply(app: &AppHandle, combination: &str) -> AppResult<()> {
    let shortcut = parse(combination)?;
    let status = app.state::<HotkeyStatus>();
    let previous = status.get().active;

    let _ = app.global_shortcut().unregister_all();

    if app.global_shortcut().register(shortcut).is_ok() {
        status.set(Status {
            active: Some(combination.trim().to_string()),
            error: None,
        });
        return Ok(());
    }

    let restored = previous.and_then(|earlier| {
        let shortcut = parse(&earlier).ok()?;
        app.global_shortcut().register(shortcut).ok()?;
        Some(earlier)
    });

    let message = crate::i18n::msg_args(
        "hotkey.taken",
        &[("value".into(), combination.trim().to_string())],
    );
    status.set(Status {
        active: restored,
        error: Some(message.clone()),
    });
    Err(AppError::msg(message))
}

/// Handler für alle registrierten Shortcuts: Overlay auf, Overlay zu.
pub fn on_shortcut(
    app: &AppHandle,
    _shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    // Rückmeldung an die Einstellungen: die Kombination kommt an.
    let _ = app.emit(EVENT_FIRED, ());
    if let Err(error) = overlay::toggle(app) {
        eprintln!("[hotkey] Overlay konnte nicht geschaltet werden: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn erkennt_gueltige_und_unbrauchbare_kombinationen() {
        assert!(parse("CommandOrControl+Alt+Space").is_ok());
        assert!(parse("CommandOrControl+Shift+G").is_ok());
        // Funktionstasten ohne Zusatztaste sind erlaubt - auf macOS sind
        // F13-F19 meist frei und daher eine gute Ausweichlösung.
        assert!(parse("F13").is_ok());

        assert!(parse("").is_err());
        assert!(parse("Leertaste").is_err());
        assert!(parse("Strg+Ö").is_err());
    }
}
