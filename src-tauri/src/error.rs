use serde::{Serialize, Serializer};

/// Fehlertyp für alle Tauri-Commands. Die Meldungen sind bewusst deutsch,
/// weil sie im Frontend unverändert angezeigt werden.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{}", crate::i18n::msg_error("db.error", .0))]
    Db(#[from] rusqlite::Error),

    #[error("{}", crate::i18n::msg_error("db.io", .0))]
    Io(#[from] std::io::Error),

    #[error("{}", crate::i18n::msg_error("db.csv", .0))]
    Csv(#[from] csv::Error),

    #[error("{}", crate::i18n::msg_error("db.window", .0))]
    Tauri(#[from] tauri::Error),

    /// Fertiger Text - für Meldungen, die schon übersetzt sind.
    #[error("{0}")]
    Msg(String),

    /// Übersetzbare Meldung: Kennung plus Platzhalter.
    ///
    /// So bleibt der Text bis zur Anzeige unbestimmt und richtet sich nach der
    /// eingestellten Sprache, nicht nach der Sprache des Codes.
    #[error("{}", crate::i18n::msg_args(.0, .1))]
    Key(&'static str, Vec<(String, String)>),
}

impl AppError {
    pub fn msg(text: impl Into<String>) -> Self {
        AppError::Msg(text.into())
    }

    /// Übersetzte Meldung ohne Platzhalter.
    pub fn key(key: &'static str) -> Self {
        AppError::Key(key, Vec::new())
    }

    /// Übersetzte Meldung mit Platzhaltern.
    pub fn args(key: &'static str, args: &[(&str, &dyn std::fmt::Display)]) -> Self {
        AppError::Key(
            key,
            args.iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        )
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
