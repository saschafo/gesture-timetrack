//! CSV-Export für die Abrechnung.
//!
//! Trennzeichen und Dezimaltrenner richten sich nach der Sprache: deutsch
//! Semikolon und Komma, englisch Komma und Punkt. Nur so öffnet Excel die Datei
//! ohne Import-Assistent - die Tabellenkalkulation richtet sich nach der
//! Systemsprache, nicht nach unserem Geschmack.

use std::path::Path;

use crate::db::Db;
use crate::error::AppResult;
use crate::i18n::{self, Lang};

pub fn write_csv(
    db: &Db,
    path: &Path,
    from: &str,
    to: &str,
    project_id: Option<i64>,
) -> AppResult<usize> {
    let entries = db.entries_between(from, to, project_id)?;
    let lang = i18n::lang(db);
    let delimiter = if lang == Lang::En { b',' } else { b';' };

    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_path(path)?;
    writer.write_record([
        i18n::t(lang, "csv.date"),
        i18n::t(lang, "csv.project"),
        i18n::t(lang, "csv.start"),
        i18n::t(lang, "csv.end"),
        i18n::t(lang, "csv.duration"),
        i18n::t(lang, "csv.hours"),
        i18n::t(lang, "csv.break"),
        i18n::t(lang, "csv.gesture"),
    ])?;

    let mut written = 0usize;
    for entry in entries.iter().rev() {
        let (date, start_time) = split_ts(&entry.start_ts);
        let end_time = entry
            .end_ts
            .as_ref()
            .map(|ts| split_ts(ts).1)
            .unwrap_or_else(|| i18n::t(lang, "csv.running"));

        let hours = format!("{:.2}", entry.duration_seconds as f64 / 3600.0);
        writer.write_record([
            date,
            project_name(&entry, lang),
            start_time,
            end_time,
            crate::state::format_hms(entry.duration_seconds),
            if lang == Lang::En {
                hours
            } else {
                hours.replace('.', ",")
            },
            format!("{:.0}", entry.pause_duration_seconds as f64 / 60.0),
            i18n::t(lang, if entry.gesture_triggered { "csv.yes" } else { "csv.no" }),
        ])?;
        written += 1;
    }

    writer.flush()?;
    Ok(written)
}

/// Projektname; gelöschte Projekte werden benannt statt leer gelassen.
fn project_name(entry: &crate::db::TimeEntry, lang: Lang) -> String {
    if entry.project_name.trim().is_empty() {
        i18n::t(lang, "entry.deleted_project")
    } else {
        entry.project_name.clone()
    }
}

fn split_ts(ts: &str) -> (String, String) {
    match ts.split_once(' ') {
        Some((date, time)) => (date.to_string(), time.to_string()),
        None => (ts.to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local};

    use crate::db::testing::TempDb;

    #[test]
    fn csv_enthaelt_kopfzeile_und_deutsche_dezimalstunden() {
        let temp = TempDb::new();
        let db = &temp.db;
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        let project = db.create_project("Kunde Meier", "#4f46e5").unwrap();
        let entry = db.open_entry(project, now - Duration::hours(2), true).unwrap();
        // 1,5 Stunden Arbeit, 30 Minuten Pause.
        db.close_entry(entry, now, 1800, 5400).unwrap();

        let path = std::env::temp_dir().join(format!("gtt-export-{}.csv", std::process::id()));
        let rows = super::write_csv(db, &path, &today, &today, None).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rows, 1);
        assert!(content.starts_with("Datum;Projekt;Beginn;Ende;"));
        assert!(content.contains("Kunde Meier"));
        assert!(content.contains("01:30:00"));
        assert!(content.contains("1,50"), "Dezimalkomma fehlt: {content}");
        assert!(content.contains(";30;"), "Pausenminuten fehlen: {content}");
        assert!(content.trim_end().ends_with(";ja"));
    }

    #[test]
    fn leerer_zeitraum_erzeugt_datei_mit_nur_einer_kopfzeile() {
        let temp = TempDb::new();
        let path = std::env::temp_dir().join(format!("gtt-export-leer-{}.csv", std::process::id()));

        let rows = super::write_csv(&temp.db, &path, "2020-01-01", "2020-01-02", None).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rows, 0);
        assert_eq!(content.lines().count(), 1);
    }
}
